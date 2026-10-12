//! Local stdio adapter. Every data call revalidates the session and membership.
use faktura_application::{
    auth::AuthStore,
    company::{self, CompanyError},
};
use faktura_infrastructure::{auth::PgAuth, company::PgCompany};
use rmcp::{model::*, service::RequestContext, ErrorData as McpError, RoleServer, ServerHandler};
use sqlx::PgPool;
#[derive(Clone)]
pub struct CompanyMcp {
    auth: PgAuth,
    store: PgCompany,
    token: String,
    ares: faktura_infrastructure::ares::Ares,
    drafts: faktura_infrastructure::draft::PgDraft,
    contacts: faktura_infrastructure::contact::PgContact,
    catalog: faktura_infrastructure::catalog::PgCatalog,
    sales: faktura_infrastructure::sales::PgSales,
    delivery: faktura_infrastructure::delivery::PgDelivery,
    shares: faktura_infrastructure::share::PgShare,
    invoice_delivery: faktura_infrastructure::invoice_delivery::PgInvoiceDelivery,
}
impl CompanyMcp {
    pub fn new(pool: PgPool, token: String) -> Self {
        Self {
            auth: PgAuth::new(pool.clone(), None),
            drafts: faktura_infrastructure::draft::PgDraft(pool.clone()),
            contacts: faktura_infrastructure::contact::PgContact(pool.clone()),
            catalog: faktura_infrastructure::catalog::PgCatalog(pool.clone()),
            sales: faktura_infrastructure::sales::PgSales(pool.clone()),
            delivery: faktura_infrastructure::delivery::PgDelivery {
                pool: pool.clone(),
                mail: None,
            },
            shares: faktura_infrastructure::share::PgShare {
                pool: pool.clone(),
                base_url: None,
            },
            invoice_delivery: faktura_infrastructure::invoice_delivery::PgInvoiceDelivery {
                pool: pool.clone(),
                mail: None,
            },
            store: PgCompany(pool),
            token,
            ares: Default::default(),
        }
    }
    pub fn with_delivery(mut self, store: faktura_infrastructure::delivery::PgDelivery) -> Self {
        self.invoice_delivery = faktura_infrastructure::invoice_delivery::PgInvoiceDelivery {
            pool: store.pool.clone(),
            mail: store.mail.clone(),
        };
        self.delivery = store;
        self
    }
    pub fn with_share(mut self, store: faktura_infrastructure::share::PgShare) -> Self {
        self.shares = store;
        self
    }
    async fn actor(&self) -> Result<String, McpError> {
        self.auth
            .session(&self.token)
            .await
            .map(|u| u.id)
            .map_err(|_| McpError::invalid_request("unauthorized", None))
    }
}
fn tools() -> Vec<Tool> {
    let empty = serde_json::json!({"type":"object","properties":{},"additionalProperties":false});
    let company = serde_json::json!({"type":"object","properties":{"company_id":{"type":"string"}},"required":["company_id"],"additionalProperties":false});
    let mut result = vec![
        Tool::new(
            "companies_list",
            "List only the authenticated user's company memberships",
            empty.as_object().unwrap().clone(),
        )
        .with_annotations(ToolAnnotations::new().read_only(true)),
        Tool::new(
            "company_context",
            "Read one authorized company, settings and role; select explicitly by company_id",
            company.as_object().unwrap().clone(),
        )
        .with_annotations(ToolAnnotations::new().read_only(true)),
    ];
    let schemas: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/contact.tools.json"))
            .expect("generated contact schemas");
    for schema in schemas.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(), "Profile-scoped contact directory. Writes require owner/editor; update also archives/restores with a revision.", schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name == "contacts_list" || name == "contact_get" || name == "contact_lookup_ares")));
    }
    let attachment_tools: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/attachment.tools.json"))
            .expect("attachment tools");
    for schema in attachment_tools.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped document files. Owner/editor uploads and deletes; viewers download. Historical downloads require an authorized delivery. Binary payload uses base64; maximum 2 MiB per file, 5 active files per document.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(!["attachment_upload","attachment_delete"].contains(&name))));
    }
    let smtp_tools: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/smtp.tools.json"))
            .expect("SMTP schemas");
    for schema in smtp_tools.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Installation administrator SMTP settings. smtp_test sends a real email: explicit recipient authorization required. Password is write-only; never log secrets.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name=="smtp_get")));
    }
    let invoice_mail: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/invoice-delivery.tools.json"
    ))
    .expect("invoice delivery schemas");
    for schema in invoice_mail.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped issued invoice email and history. invoice_send queues an actual email with PDF attachment; use only with explicit authorization for the recipient. Issuing and payment are not changed. PDF returns base64.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name!="invoice_send")));
    }
    let schemas: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/catalog.tools.json"))
            .expect("generated catalog schemas");
    for schema in schemas.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(), "Profile-scoped catalog directory. Writes require owner/editor; update also archives/restores with a revision.", schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name == "catalogs_list" || name == "catalog_get" || name == "catalog_line")));
    }
    let schemas: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/sales.tools.json"))
            .expect("sales schemas");
    for schema in schemas.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped templates and quotations. sales_send queues an actual email with a PDF attachment or selected public share_id: use only with explicit authorization to send to the supplied recipient. sales_sharecreate grants anyone holding the returned URL read access to the current quotation until expiry or revocation. Conversion only saves a draft. PDF tools return base64.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name=="sales_list"||name=="sales_get"||name=="sales_pdf"||name=="sales_history"||name=="sales_deliverypdf"||name=="sales_sharelist"||name=="sales_shareviews")));
    }
    let drafts: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/draft.tools.json"))
            .expect("draft schema");
    for schema in drafts.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(
            Tool::new(
                name.to_owned(),
                "Profile-scoped invoices. invoice_issue explicitly locks a saved revision and allocates a final number; invoice_pdf returns base64 PDF for issued invoices. No tool sends a document.",
                schema["inputSchema"].as_object().unwrap().clone(),
            )
            .with_annotations(ToolAnnotations::new().read_only(
                name == "invoices_list" || name == "invoice_get" || name == "invoice_pdf" || name == "invoice_payments" || name == "company_overview",
            )),
        );
    }
    let imports: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/import.tools.json"))
            .expect("import schemas");
    for schema in imports.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped historical import. Prepare persists a preview only; apply explicitly commits up to 25 records, repeated calls resume safely. Inspect errors and report; nothing is sent.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name=="imports_list"||name=="import_get")));
    }
    let invoice: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/invoice.tools.json"))
            .expect("generated invoice schema");
    result.push(
        Tool::new(
            "invoice_calculate",
            "Calculate a versioned preview; does not save or issue an invoice.",
            invoice.as_object().unwrap().clone(),
        )
        .with_annotations(ToolAnnotations::new().read_only(true)),
    );
    result
}
impl ServerHandler for CompanyMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        self.actor().await?;
        Ok(ListToolsResult {
            tools: tools(),
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let actor = self.actor().await?;
        let args = request.arguments.unwrap_or_default();
        let result = match request.name.as_ref() {
            "imports_list" | "import_get" | "import_prepare" | "import_apply" => {
                import_tool(
                    &faktura_infrastructure::import::PgImport(self.drafts.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "company_overview"
            | "invoice_payments"
            | "invoice_register_payment"
            | "invoice_reverse_payment" => {
                payment_tool(
                    &faktura_infrastructure::payment::PgPayment(self.drafts.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "companies_list" if args.is_empty() => company::list(&self.store, &actor)
                .await
                .and_then(|v| serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)),
            "company_context" if args.len() == 1 => match args
                .get("company_id")
                .and_then(|v| v.as_str())
            {
                Some(id) => company::context(&self.store, &actor, id)
                    .await
                    .and_then(|v| serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)),
                None => Err(CompanyError::Invalid(vec!["company_id".into()])),
            },
            "contact_lookup_ares" if args.len() == 2 => {
                match (
                    args.get("company_id").and_then(|v| v.as_str()),
                    args.get("ico").and_then(|v| v.as_str()),
                ) {
                    (Some(company), Some(ico)) => faktura_application::ares::lookup(
                        &self.store,
                        &self.ares,
                        &actor,
                        company,
                        ico,
                    )
                    .await
                    .and_then(|v| serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)),
                    _ => Err(CompanyError::Invalid(vec!["arguments".into()])),
                }
            }
            "invoices_list"
            | "invoice_get"
            | "invoice_create_draft"
            | "invoice_update_draft"
            | "invoice_issue"
            | "invoice_pdf" => draft_tool(&self.drafts, &actor, request.name.as_ref(), args).await,
            "invoice_calculate" => {
                let mut args = args;
                let company = args
                    .remove("company_id")
                    .and_then(|v| v.as_str().map(str::to_owned));
                match (
                    company,
                    serde_json::from_value(serde_json::Value::Object(args)),
                ) {
                    (Some(company), Ok(input)) => faktura_application::invoice::calculate(
                        &self.store,
                        &actor,
                        &company,
                        input,
                    )
                    .await
                    .and_then(|v| serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)),
                    _ => Err(CompanyError::Invalid(vec!["arguments".into()])),
                }
            }
            "attachment_list"
            | "attachment_upload"
            | "attachment_download"
            | "attachment_delete"
            | "attachment_historydownload" => {
                attachment_tool(
                    &faktura_infrastructure::attachment::PgAttachment(self.delivery.pool.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "smtp_get" | "smtp_save" | "smtp_test" => {
                use faktura_application::smtp::{self, SmtpStore};
                let store = faktura_infrastructure::smtp::PgSmtp {
                    pool: self.delivery.pool.clone(),
                    fallback: self.delivery.mail.clone(),
                };
                let value = serde_json::Value::Object(args);
                match request.name.as_ref() {
                    "smtp_get" => store
                        .get(&actor)
                        .await
                        .map(|v| serde_json::to_value(v).unwrap()),
                    "smtp_save" => match serde_json::from_value(value) {
                        Ok(cmd) => smtp::save(&store, &actor, cmd)
                            .await
                            .map(|v| serde_json::to_value(v).unwrap()),
                        Err(_) => Err(CompanyError::Invalid(vec!["arguments".into()])),
                    },
                    _ => match serde_json::from_value(value) {
                        Ok(cmd) => smtp::test(&store, &actor, cmd)
                            .await
                            .map(|v| serde_json::to_value(v).unwrap()),
                        Err(_) => Err(CompanyError::Invalid(vec!["arguments".into()])),
                    },
                }
            }
            "invoice_send" | "invoice_history" | "invoice_deliverypdf" => {
                delivery_tool(&self.invoice_delivery, &actor, request.name.as_ref(), args).await
            }
            "sales_pdf" | "sales_deliverypdf" | "sales_send" | "sales_history" => {
                delivery_tool(&self.delivery, &actor, request.name.as_ref(), args).await
            }
            "sales_sharecreate" | "sales_sharelist" | "sales_sharerevoke" | "sales_shareviews" => {
                share_tool(&self.shares, &actor, request.name.as_ref(), args).await
            }
            "sales_list" | "sales_get" | "sales_create" | "sales_update" | "sales_convert" => {
                sales_tool(&self.sales, &actor, request.name.as_ref(), args).await
            }
            "catalogs_list" | "catalog_get" | "catalog_create" | "catalog_update"
            | "catalog_line" => {
                catalog_tool(&self.catalog, &actor, request.name.as_ref(), args).await
            }
            "contacts_list" | "contact_get" | "contact_create" | "contact_update" => {
                contact_tool(&self.contacts, &actor, request.name.as_ref(), args).await
            }
            _ => {
                return Err(McpError::invalid_params(
                    "Unknown tool or invalid arguments",
                    None,
                ))
            }
        };
        Ok(match result {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(match e {
                CompanyError::NotFound => "not_found",
                CompanyError::Forbidden => "forbidden",
                CompanyError::Invalid(_) => "invalid_input",
                CompanyError::Conflict => "revision_conflict",
                CompanyError::Unavailable => "unavailable",
            })]),
        }
        .into())
    }
}

async fn contact_tool(
    store: &faktura_infrastructure::contact::PgContact,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::contact;
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = if name == "contact_get" || name == "contact_update" {
        args.remove("id")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?
    } else {
        String::new()
    };
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "contacts_list" => {
            serde_json::to_value(contact::list(store, actor, &company, parse(args)?).await?)
        }
        "contact_get" if args.is_empty() => {
            serde_json::to_value(contact::get(store, actor, &company, &id).await?)
        }
        "contact_create" => {
            serde_json::to_value(contact::create(store, actor, &company, parse(args)?).await?)
        }
        "contact_update" => {
            serde_json::to_value(contact::update(store, actor, &company, &id, parse(args)?).await?)
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn catalog_tool(
    store: &faktura_infrastructure::catalog::PgCatalog,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::catalog;
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = if name == "catalog_get" || name == "catalog_update" || name == "catalog_line" {
        args.remove("id")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?
    } else {
        String::new()
    };
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "catalogs_list" => {
            serde_json::to_value(catalog::list(store, actor, &company, parse(args)?).await?)
        }
        "catalog_get" if args.is_empty() => {
            serde_json::to_value(catalog::get(store, actor, &company, &id).await?)
        }
        "catalog_line" => {
            serde_json::to_value(catalog::line(store, actor, &company, &id, parse(args)?).await?)
        }
        "catalog_create" => {
            serde_json::to_value(catalog::create(store, actor, &company, parse(args)?).await?)
        }
        "catalog_update" => {
            serde_json::to_value(catalog::update(store, actor, &company, &id, parse(args)?).await?)
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn draft_tool(
    store: &faktura_infrastructure::draft::PgDraft,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::draft;
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = if name == "invoice_get"
        || name == "invoice_update_draft"
        || name == "invoice_issue"
        || name == "invoice_pdf"
    {
        args.remove("id")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?
    } else {
        String::new()
    };
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "invoices_list" => {
            serde_json::to_value(draft::list(store, actor, &company, parse(args)?).await?)
        }
        "invoice_get" if args.is_empty() => {
            serde_json::to_value(draft::get(store, actor, &company, &id).await?)
        }
        "invoice_issue" => {
            serde_json::to_value(draft::issue(store, actor, &company, &id, parse(args)?).await?)
        }
        "invoice_pdf" if args.is_empty() => {
            use base64::Engine;
            let bytes = draft::pdf(store, actor, &company, &id).await?;
            Ok(serde_json::json!({"mime_type":"application/pdf", "base64":base64::engine::general_purpose::STANDARD.encode(bytes)}))
        }
        "invoice_create_draft" => {
            serde_json::to_value(draft::create(store, actor, &company, parse(args)?).await?)
        }
        "invoice_update_draft" => {
            serde_json::to_value(draft::update(store, actor, &company, &id, parse(args)?).await?)
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn payment_tool(
    store: &faktura_infrastructure::payment::PgPayment,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::payment;
    fn take(
        args: &mut serde_json::Map<String, serde_json::Value>,
        key: &str,
    ) -> company::Result<String> {
        args.remove(key)
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec![key.into()]))
    }
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    let company = take(&mut args, "company_id")?;
    if name == "company_overview" && args.is_empty() {
        return serde_json::to_value(payment::overview(store, actor, &company).await?)
            .map_err(|_| CompanyError::Unavailable);
    }
    let id = take(&mut args, "id")?;
    let result = match name {
        "invoice_payments" if args.is_empty() => payment::get(store, actor, &company, &id).await?,
        "invoice_register_payment" => {
            payment::register(store, actor, &company, &id, parse(args)?).await?
        }
        "invoice_reverse_payment" => {
            let payment = take(&mut args, "payment_id")?;
            payment::reverse(store, actor, &company, &id, &payment, parse(args)?).await?
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    };
    serde_json::to_value(result).map_err(|_| CompanyError::Unavailable)
}

async fn import_tool(
    store: &faktura_infrastructure::import::PgImport,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::import::{self as migration, ImportStore};
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = args
        .remove("id")
        .and_then(|v| v.as_str().map(str::to_owned));
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "imports_list" if args.is_empty() && id.is_none() => {
            serde_json::to_value(store.list(actor, &company).await?)
        }
        "import_get" if args.is_empty() && id.is_some() => {
            serde_json::to_value(store.get(actor, &company, id.as_deref().unwrap()).await?)
        }
        "import_prepare" if id.is_none() => {
            serde_json::to_value(migration::prepare(store, actor, &company, parse(args)?).await?)
        }
        "import_apply" if id.is_some() => serde_json::to_value(
            migration::apply(store, actor, &company, id.as_deref().unwrap(), parse(args)?).await?,
        ),
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn sales_tool(
    store: &faktura_infrastructure::sales::PgSales,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::sales;
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = if ["sales_get", "sales_update", "sales_convert"].contains(&name) {
        args.remove("id")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?
    } else {
        String::new()
    };
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "sales_list" => {
            serde_json::to_value(sales::list(store, actor, &company, parse(args)?).await?)
        }
        "sales_get" if args.is_empty() => {
            serde_json::to_value(sales::get(store, actor, &company, &id).await?)
        }
        "sales_create" => {
            serde_json::to_value(sales::create(store, actor, &company, parse(args)?).await?)
        }
        "sales_update" => {
            serde_json::to_value(sales::update(store, actor, &company, &id, parse(args)?).await?)
        }
        "sales_convert" => {
            serde_json::to_value(sales::convert(store, actor, &company, &id, parse(args)?).await?)
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn delivery_tool(
    store: &dyn faktura_application::delivery::DeliveryStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::delivery;
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = args
        .remove("id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    match name {
        "invoice_send" => serde_json::to_value(
            faktura_application::invoice_delivery::send(
                store,
                actor,
                &c,
                &id,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        )
        .map_err(|_| CompanyError::Unavailable),
        "sales_send" => serde_json::to_value(
            delivery::send(
                store,
                actor,
                &c,
                &id,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        )
        .map_err(|_| CompanyError::Unavailable),
        "sales_history" | "invoice_history" => {
            let page = match args.remove("page") {
                Some(v) => v.as_i64().ok_or_else(invalid)?,
                None => 1,
            };
            if !args.is_empty() {
                return Err(invalid());
            }
            serde_json::to_value(delivery::history(store, actor, &c, &id, page).await?)
                .map_err(|_| CompanyError::Unavailable)
        }
        "sales_pdf" | "sales_deliverypdf" | "invoice_deliverypdf" => {
            let d = if name.ends_with("_deliverypdf") {
                Some(
                    args.remove("delivery")
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .ok_or_else(invalid)?,
                )
            } else {
                None
            };
            if !args.is_empty() {
                return Err(invalid());
            }
            use base64::Engine;
            Ok(
                serde_json::json!({"content_type":"application/pdf","base64":base64::engine::general_purpose::STANDARD.encode(delivery::pdf(store,actor,&c,&id,d.as_deref()).await?)}),
            )
        }
        _ => Err(invalid()),
    }
}

async fn share_tool(
    s: &faktura_infrastructure::share::PgShare,
    a: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::share;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let d = args
        .remove("id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let result = match name {
        "sales_sharecreate" => serde_json::to_value(
            share::create(
                s,
                a,
                &c,
                &d,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        "sales_sharelist" => {
            let p = match args.remove("page") {
                None => 1,
                Some(v) => v.as_i64().ok_or_else(invalid)?,
            };
            if !args.is_empty() {
                return Err(invalid());
            }
            serde_json::to_value(share::list(s, a, &c, &d, p).await?)
        }
        "sales_sharerevoke" | "sales_shareviews" => {
            let id = args
                .remove("share")
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or_else(invalid)?;
            if name == "sales_sharerevoke" {
                if !args.is_empty() {
                    return Err(invalid());
                }
                share::revoke(s, a, &c, &d, &id).await?;
                Ok(serde_json::Value::Null)
            } else {
                let p = match args.remove("page") {
                    None => 1,
                    Some(v) => v.as_i64().ok_or_else(invalid)?,
                };
                if !args.is_empty() {
                    return Err(invalid());
                }
                serde_json::to_value(share::views(s, a, &c, &d, &id, p).await?)
            }
        }
        _ => return Err(invalid()),
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn attachment_tool(
    store: &dyn faktura_application::attachment::AttachmentStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Value, CompanyError> {
    use base64::Engine;
    use faktura_application::attachment;
    let mut take = |k: &str| {
        args.remove(k)
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec![k.into()]))
    };
    let c = take("company_id")?;
    let kind = take("kind")?;
    let doc = take("document_id")?;
    let file = if !["attachment_list", "attachment_upload"].contains(&name) {
        Some(take("id")?)
    } else {
        None
    };
    let delivery = if name == "attachment_historydownload" {
        Some(take("delivery")?)
    } else {
        None
    };
    if name != "attachment_upload" && !args.is_empty() {
        return Err(CompanyError::Invalid(vec!["arguments".into()]));
    }
    match name {
        "attachment_list" => {
            Ok(serde_json::to_value(store.list(actor, &c, &kind, &doc).await?).unwrap())
        }
        "attachment_upload" => {
            let cmd = serde_json::from_value(serde_json::Value::Object(args))
                .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))?;
            Ok(
                serde_json::to_value(attachment::upload(store, actor, &c, &kind, &doc, cmd).await?)
                    .unwrap(),
            )
        }
        "attachment_delete" => {
            store
                .delete(actor, &c, &kind, &doc, file.as_deref().unwrap())
                .await?;
            Ok(serde_json::json!({"deleted":true}))
        }
        _ => {
            let f = store
                .download(
                    actor,
                    &c,
                    &kind,
                    &doc,
                    file.as_deref().unwrap(),
                    delivery.as_deref(),
                )
                .await?;
            Ok(
                serde_json::json!({"metadata":f.metadata,"base64":base64::engine::general_purpose::STANDARD.encode(f.content)}),
            )
        }
    }
}
