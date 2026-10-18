use axum::http::{header, HeaderValue, Method};
use faktura_application::auth::AuthStore;
use faktura_infrastructure::auth::PgAuth;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let database_url = match std::env::var("DATABASE_URL_FILE") {
        Ok(path) => std::fs::read_to_string(path)?.trim().to_owned(),
        Err(_) => std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL or DATABASE_URL_FILE is missing")?,
    };
    let pool = faktura_infrastructure::pool(&database_url)?;
    faktura_infrastructure::migrate(&pool).await?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("recover-admin") {
        if args.len() != 3 {
            return Err("Usage: recover-admin EMAIL PASSWORD_FILE".into());
        }
        let password = if args[2] == "-" {
            use std::io::Read;
            let mut value = String::new();
            std::io::stdin().take(1026).read_to_string(&mut value)?;
            value
        } else {
            std::fs::read_to_string(&args[2])?
        };
        faktura_application::auth::recover(
            &PgAuth::new(pool.clone(), None),
            &args[1],
            password.trim_end_matches(['\n', '\r']),
        )
        .await
        .map_err(|_| "Recovery failed (account, password policy or database)")?;
        println!("Administrator password replaced; all sessions revoked.");
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("mcp-login") {
        if args.len() != 4 {
            return Err("Usage: mcp-login EMAIL PASSWORD_FILE SESSION_FILE".into());
        }
        let password = std::fs::read_to_string(&args[2])?;
        let (_, token) = faktura_application::auth::login(
            &PgAuth::new(pool.clone(), None),
            &args[1],
            password.trim_end_matches(['\n', '\r']),
        )
        .await
        .map_err(|_| "MCP login failed")?;
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = options
            .open(&args[3])
            .and_then(|mut f| f.write_all(token.as_bytes()));
        if result.is_err() {
            let _ = PgAuth::new(pool.clone(), None).logout(&token).await;
        }
        result?;
        eprintln!("Private MCP session saved. Expires in 12 hours; recovery revokes it.");
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("mcp") {
        if args.len() != 2 {
            return Err("Usage: mcp SESSION_FILE".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if std::fs::metadata(&args[1])?.permissions().mode() & 0o077 != 0 {
                return Err("MCP session file must be private (chmod 600)".into());
            }
        }
        let token = std::fs::read_to_string(&args[1])?.trim().to_owned();
        PgAuth::new(pool.clone(), None)
            .session(&token)
            .await
            .map_err(|_| "Invalid MCP session")?;
        use rmcp::ServiceExt;
        faktura_server::mcp::CompanyMcp::new(pool.clone(), token)
            .with_share(faktura_infrastructure::share::PgShare {
                pool: pool.clone(),
                base_url: faktura_infrastructure::share::base_url_from_env()?,
            })
            .with_delivery(faktura_infrastructure::delivery::PgDelivery {
                pool: pool.clone(),
                mail: faktura_infrastructure::delivery::MailConfig::from_env()?
                    .map(std::sync::Arc::new),
            })
            .serve(rmcp::transport::stdio())
            .await?
            .waiting()
            .await?;
        return Ok(());
    }
    if !args.is_empty() {
        return Err("Unknown command".into());
    }
    let production = std::env::var("APP_ENV").unwrap_or_else(|_| "development".into());
    if !["production", "development"].contains(&production.as_str()) {
        return Err("APP_ENV must be production or development".into());
    }
    let secure = production == "production";
    let origins: Vec<String> = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| {
            if secure {
                String::new()
            } else {
                "http://localhost:5173,http://127.0.0.1:5173".into()
            }
        })
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    if origins.iter().any(|v| {
        v.is_empty() || v == "*" || (secure && !v.starts_with("https://")) || v.ends_with('/')
    }) {
        return Err("CORS_ORIGINS must contain explicit origins (HTTPS in production), without trailing slash".into());
    }
    let origin_headers = origins
        .iter()
        .map(|v| v.parse::<HeaderValue>())
        .collect::<Result<Vec<_>, _>>()?;
    let secret = match std::env::var("BOOTSTRAP_SECRET_FILE") {
        Ok(path) => Some(std::fs::read_to_string(path)?.trim().to_owned()),
        Err(_) => None,
    };
    if secret
        .as_ref()
        .is_some_and(|s| s.len() < 32 || s.len() > 512)
    {
        return Err("Bootstrap secret must contain 32–512 bytes".into());
    }
    if PgAuth::new(pool.clone(), None)
        .setup_required()
        .await
        .map_err(|_| "Cannot read installation state")?
        && secret.is_none()
    {
        return Err("New installation requires BOOTSTRAP_SECRET_FILE; generate a private random secret before starting".into());
    }
    let delivery = faktura_infrastructure::delivery::PgDelivery {
        pool: pool.clone(),
        mail: faktura_infrastructure::delivery::MailConfig::from_env()?.map(std::sync::Arc::new),
    };
    let (stop_delivery, mut delivery_shutdown) = tokio::sync::watch::channel(false);
    let invoice_delivery = faktura_infrastructure::invoice_delivery::PgInvoiceDelivery {
        pool: pool.clone(),
        mail: delivery.mail.clone(),
    };
    let communication_store = faktura_infrastructure::communication::PgCommunication(pool.clone());
    let recurring_store = faktura_infrastructure::recurring::PgRecurring(pool.clone());
    let recurring_mail = invoice_delivery.clone();
    let mut recurring_shutdown = delivery_shutdown.clone();
    let recurring_worker = tokio::spawn(async move {
        loop {
            if *recurring_shutdown.borrow() {
                break;
            }
            if communication_store
                .process_one(&recurring_mail)
                .await
                .is_err()
            {
                tracing::warn!("Invoice communication unavailable");
            }
            if recurring_store.process_one().await.is_err() {
                tracing::warn!("Recurring generation unavailable");
            }
            if recurring_store.enqueue_one(&recurring_mail).await.is_err() {
                tracing::warn!("Recurring delivery enqueue unavailable");
            }
            tokio::select! { _=recurring_shutdown.changed()=>{}, _=tokio::time::sleep(std::time::Duration::from_secs(2))=>{} }
        }
    });
    let bank_store = faktura_infrastructure::bank_connection::PgConnection::new(pool.clone());
    let bank_matching = faktura_infrastructure::banking::PgBank(pool.clone());
    let mut bank_shutdown = delivery_shutdown.clone();
    let bank_worker = tokio::spawn(async move {
        loop {
            if *bank_shutdown.borrow() {
                break;
            }
            if bank_matching.process_matching_one().await.is_err() {
                tracing::warn!("Bank matching worker unavailable");
            }
            if bank_store.process_one().await.is_err() {
                tracing::warn!("Bank synchronization worker unavailable");
            }
            tokio::select! { _=bank_shutdown.changed()=>{}, _=tokio::time::sleep(std::time::Duration::from_secs(5))=>{} }
        }
    });
    let invoice_worker = invoice_delivery.clone();
    let worker_store = delivery.clone();
    let delivery_worker = tokio::spawn(async move {
        loop {
            if *delivery_shutdown.borrow() {
                break;
            }
            let invoice_work = match invoice_worker.process_one().await {
                Ok(work) => work,
                Err(_) => {
                    tracing::warn!("Invoice delivery worker unavailable");
                    false
                }
            };
            match worker_store.process_one().await {
                Ok(true) => continue,
                Ok(false) if invoice_work => continue,
                Ok(false) => {}
                Err(_) => tracing::warn!("Quotation delivery worker unavailable"),
            }
            tokio::select! { _=delivery_shutdown.changed()=>{}, _=tokio::time::sleep(std::time::Duration::from_secs(1))=>{} }
        }
    });
    let share = faktura_infrastructure::share::PgShare {
        pool: pool.clone(),
        base_url: faktura_infrastructure::share::base_url_from_env()?,
    };
    let app = faktura_server::router(pool.clone())
        .merge(faktura_server::advance_settlement::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::banking::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::retail::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::inbox::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::period_report::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::stock::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::preferences::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::cash::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::received::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::credit_note::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::communication::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::recurring::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::tax_receipt::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::advance::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::export::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::attachment::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::smtp::router(
            faktura_infrastructure::smtp::PgSmtp {
                pool: pool.clone(),
                fallback: delivery.mail.clone(),
            },
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::invoice_delivery::router(
            pool.clone(),
            origins.clone(),
            secure,
            invoice_delivery,
        ))
        .merge(faktura_server::invoice_share::router(
            faktura_infrastructure::invoice_share::PgInvoiceShare {
                pool: pool.clone(),
                base_url: share.base_url.clone(),
            },
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::share::router(
            share,
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::delivery::router(
            pool.clone(),
            origins.clone(),
            secure,
            delivery,
        ))
        .merge(faktura_server::auth::router(
            pool.clone(),
            secret,
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::import::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::invoice::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::catalog::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::sales::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::exchange_rate::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::contact::router(
            pool.clone(),
            origins.clone(),
            secure,
        ))
        .merge(faktura_server::company::router(
            pool.clone(),
            origins,
            secure,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(origin_headers)
                .allow_credentials(true)
                .allow_headers([
                    header::CONTENT_TYPE,
                    axum::http::HeaderName::from_static("x-faktura-request"),
                ])
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]),
        )
        .layer(TraceLayer::new_for_http());
    let app = if let Ok(directory) = std::env::var("STATIC_DIR") {
        app.fallback_service(
            tower_http::services::ServeDir::new(&directory).not_found_service(
                tower_http::services::ServeFile::new(format!("{directory}/index.html")),
            ),
        )
    } else {
        app
    };
    let address = std::env::var("API_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3001".into());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!(%address,"API listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    let _ = stop_delivery.send(true);
    let _ = delivery_worker.await;
    let _ = recurring_worker.await;
    let _ = bank_worker.await;
    pool.close().await;
    Ok(())
}
