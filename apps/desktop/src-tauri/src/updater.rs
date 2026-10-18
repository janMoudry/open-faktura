//! Explicit, local-window-only signed client updates. Never touches server data.
use base64::Engine;
use futures_util::StreamExt;
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_updater::{Update, UpdaterExt};
const MAX_PACKAGE: usize = 256 * 1024 * 1024;
#[derive(Clone)]
pub struct Config {
    pub endpoint: url::Url,
    pub public_key: String,
}
pub fn config(endpoint: Option<&str>, key: Option<&str>) -> Result<Option<Config>, String> {
    match (
        endpoint.filter(|s| !s.is_empty()),
        key.filter(|s| !s.is_empty()),
    ) {
        (None, None) => Ok(None),
        (Some(endpoint), Some(key)) => {
            let endpoint = url::Url::parse(endpoint).map_err(|_| "updater_configuration")?;
            if endpoint.scheme() != "https"
                || endpoint.host_str().is_none()
                || !endpoint.username().is_empty()
                || endpoint.password().is_some()
                || endpoint.fragment().is_some()
            {
                return Err("updater_configuration".into());
            }
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(key.trim())
                .map_err(|_| "updater_configuration")?;
            let text = std::str::from_utf8(&decoded).map_err(|_| "updater_configuration")?;
            minisign_verify::PublicKey::decode(text).map_err(|_| "updater_configuration")?;
            Ok(Some(Config {
                endpoint,
                public_key: key.trim().into(),
            }))
        }
        _ => Err("updater_configuration".into()),
    }
}
#[derive(Clone, Serialize)]
pub struct Candidate {
    pub version: String,
    pub current_version: String,
    pub notes: String,
    pub token: String,
}
struct Pending {
    candidate: Candidate,
    update: Update,
    checked: Instant,
}
pub struct Updates {
    config: Option<Config>,
    pending: Mutex<Option<Pending>>,
    operation: tokio::sync::Mutex<()>,
}
impl Updates {
    pub fn new(config: Option<Config>) -> Self {
        Self {
            config,
            pending: Mutex::new(None),
            operation: tokio::sync::Mutex::new(()),
        }
    }
}
#[derive(Serialize)]
pub struct Status {
    enabled: bool,
    current_version: String,
}
fn local(label: &str, url: &url::Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    if label != "main" {
        return false;
    }
    if url.scheme() == "tauri" && url.host_str() == Some("localhost") && url.port().is_none() {
        return true;
    }
    let origin = url.origin().ascii_serialization();
    matches!(
        origin.as_str(),
        "tauri://localhost" | "http://tauri.localhost" | "https://tauri.localhost"
    ) || cfg!(debug_assertions)
        && matches!(
            origin.as_str(),
            "http://127.0.0.1:5173" | "http://localhost:5173"
        )
}
fn authorize(w: &WebviewWindow) -> Result<(), String> {
    if local(w.label(), &w.url().map_err(|_| "updater_forbidden")?) {
        Ok(())
    } else {
        Err("updater_forbidden".into())
    }
}
fn gate(
    candidate: &Candidate,
    version: &str,
    token: &str,
    confirmed: bool,
    elapsed: Duration,
) -> Result<(), String> {
    if !confirmed
        || candidate.version != version
        || candidate.token != token
        || elapsed > Duration::from_secs(600)
    {
        Err("updater_confirmation".into())
    } else {
        Ok(())
    }
}
#[tauri::command]
pub fn desktop_updater_status(
    window: WebviewWindow,
    state: State<'_, Updates>,
) -> Result<Status, String> {
    authorize(&window)?;
    Ok(Status {
        enabled: state.config.is_some(),
        current_version: env!("CARGO_PKG_VERSION").into(),
    })
}
#[tauri::command]
pub async fn desktop_updater_check(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Updates>,
) -> Result<Option<Candidate>, String> {
    authorize(&window)?;
    let _operation = state.operation.try_lock().map_err(|_| "updater_busy")?;
    let config = state.config.as_ref().ok_or("updater_disabled")?;
    *state.pending.lock().map_err(|_| "updater_failed")? = None;
    let endpoint = endpoint_url(&config.endpoint, &app.package_info().version.to_string())?;
    let Some(raw) = fetch_manifest(&endpoint, false).await? else {
        return Ok(None);
    };
    let update = app
        .updater_builder()
        .endpoints(vec![config.endpoint.clone()])
        .map_err(|_| "updater_configuration")?
        .build()
        .map_err(|_| "updater_configuration")?
        .restore_update(raw)
        .map_err(|_| "updater_check_failed")?;
    let Some(update) = update else {
        return Ok(None);
    };
    // URLs are from the operator's TLS manifest, but packages still need HTTPS and
    // a valid signature/version. Never accept user-provided URLs or installer args.
    if update.download_url.scheme() != "https"
        || !update.download_url.username().is_empty()
        || update.download_url.password().is_some()
    {
        return Err("updater_check_failed".into());
    }
    let candidate = Candidate {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update
            .body
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(8000)
            .collect(),
        token: uuid::Uuid::new_v4().to_string(),
    };
    *state.pending.lock().map_err(|_| "updater_failed")? = Some(Pending {
        candidate: candidate.clone(),
        update,
        checked: Instant::now(),
    });
    Ok(Some(candidate))
}
fn endpoint_url(endpoint: &url::Url, version: &str) -> Result<url::Url, String> {
    use tauri::utils::{config::BundleType, platform::bundle_type};
    let os = if std::env::consts::OS == "macos" {
        "darwin"
    } else {
        std::env::consts::OS
    };
    let installer = match bundle_type() {
        Some(BundleType::App) => "app",
        Some(BundleType::AppImage) => "appimage",
        Some(BundleType::Deb) => "deb",
        Some(BundleType::Nsis) => "nsis",
        Some(BundleType::Msi) => "msi",
        _ => "",
    };
    let mut url = endpoint.to_string();
    for (key, value) in [
        ("current_version", version),
        ("target", os),
        ("arch", std::env::consts::ARCH),
        ("bundle_type", installer),
    ] {
        url = url
            .replace(&format!("{{{{{key}}}}}"), value)
            .replace(&format!("%7B%7B{key}%7D%7D"), value)
    }
    url.parse().map_err(|_| "updater_configuration".into())
}
async fn fetch_manifest(
    url: &url::Url,
    allow_loopback: bool,
) -> Result<Option<serde_json::Value>, String> {
    let bytes = fetch(url, allow_loopback, 512 * 1024, 30, true).await?;
    bytes
        .map(|b| serde_json::from_slice(&b).map_err(|_| "updater_check_failed".into()))
        .transpose()
}
async fn fetch(
    url: &url::Url,
    allow_loopback: bool,
    limit: usize,
    seconds: u64,
    no_content: bool,
) -> Result<Option<Vec<u8>>, String> {
    if url.scheme() != "https"
        && !(allow_loopback && url.scheme() == "http" && url.host_str() == Some("127.0.0.1"))
    {
        return Err("updater_download_failed".into());
    }
    let client = reqwest::Client::builder()
        .https_only(!allow_loopback)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || attempt.url().scheme() != "https" {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .timeout(Duration::from_secs(seconds))
        .build()
        .map_err(|_| "updater_download_failed")?;
    let response = client
        .get(url.clone())
        .header(
            "Accept",
            if no_content {
                "application/json"
            } else {
                "application/octet-stream"
            },
        )
        .send()
        .await
        .map_err(|_| "updater_download_failed")?;
    if response.status() == reqwest::StatusCode::NO_CONTENT && no_content {
        return Ok(None);
    }
    if response.status() != reqwest::StatusCode::OK
        || response.content_length().is_some_and(|s| s > limit as u64)
    {
        return Err("updater_download_failed".into());
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "updater_download_failed")?;
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err("updater_download_failed".into());
        }
        bytes.extend_from_slice(&chunk)
    }
    Ok(Some(bytes))
}
async fn download(update: &Update, allow_loopback: bool) -> Result<Vec<u8>, String> {
    let bytes = fetch(
        &update.download_url,
        allow_loopback,
        MAX_PACKAGE,
        180,
        false,
    )
    .await?
    .ok_or("updater_download_failed")?;
    update
        .verify(&bytes)
        .map_err(|_| "updater_signature_failed")?;
    Ok(bytes)
}
#[tauri::command]
pub async fn desktop_updater_install(
    window: WebviewWindow,
    state: State<'_, Updates>,
    version: String,
    token: String,
    confirmed: bool,
) -> Result<(), String> {
    authorize(&window)?;
    let _operation = state.operation.try_lock().map_err(|_| "updater_busy")?;
    if state.config.is_none() {
        return Err("updater_disabled".into());
    }
    let pending = {
        let mut slot = state.pending.lock().map_err(|_| "updater_failed")?;
        let p = slot.as_ref().ok_or("updater_no_pending")?;
        gate(
            &p.candidate,
            &version,
            &token,
            confirmed,
            p.checked.elapsed(),
        )?;
        slot.take().ok_or("updater_no_pending")?
    };
    let bytes = download(&pending.update, false).await?;
    // The plugin verifies signature and signed version again before any OS install.
    // On Windows the installer exits the app; the confirmation explicitly says so.
    pending
        .update
        .restart_after_install(false)
        .install(&bytes)
        .map_err(|_| "updater_install_failed".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_partial_config_origin_and_confirmation() {
        assert!(config(None, None).unwrap().is_none());
        assert!(config(Some("https://example.test/latest.json"), None).is_err());
        assert!(config(None, Some("invalid")).is_err());
        assert!(config(Some("http://example.test/latest.json"), Some("invalid")).is_err());
        assert!(!local("other", &"tauri://localhost/help".parse().unwrap()));
        assert!(!local(
            "main",
            &"https://evil.example.test".parse().unwrap()
        ));
        assert!(local("main", &"tauri://localhost/help".parse().unwrap()));
        let c = Candidate {
            version: "2.0.0".into(),
            current_version: "1.0.0".into(),
            notes: String::new(),
            token: "synthetic-token".into(),
        };
        assert!(gate(&c, "2.0.0", "synthetic-token", true, Duration::ZERO).is_ok());
        assert!(gate(&c, "2.0.0", "synthetic-token", false, Duration::ZERO).is_err());
        assert!(gate(&c, "2.0.0", "other-token", true, Duration::ZERO).is_err());
        assert!(gate(&c, "3.0.0", "synthetic-token", true, Duration::ZERO).is_err());
        assert!(gate(
            &c,
            "2.0.0",
            "synthetic-token",
            true,
            Duration::from_secs(601)
        )
        .is_err());
    }
    #[tokio::test]
    async fn signed_local_fixture_verifies_tampering_and_version_without_installing() {
        use std::{path::PathBuf, process::Command};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        struct Temp(PathBuf);
        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let temp = Temp(std::env::temp_dir().join(format!(
            "open-faktura-updater-test-{}",
            uuid::Uuid::new_v4()
        )));
        std::fs::create_dir(&temp.0).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&temp.0, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        let key = temp.0.join("throwaway.key");
        let artifact = temp.0.join("synthetic.bin");
        let bytes = b"synthetic signed fixture, not an installer";
        std::fs::write(&artifact, bytes).unwrap();
        // Child output may contain the throwaway secret: capture and discard it,
        // never print it or persist it anywhere beyond this private temp directory.
        let generate = Command::new("pnpm")
            .current_dir(&root)
            .env_remove("TAURI_SIGNING_PRIVATE_KEY")
            .env_remove("TAURI_SIGNING_PRIVATE_KEY_PATH")
            .args([
                "--filter",
                "@open-faktura/desktop",
                "exec",
                "tauri",
                "signer",
                "generate",
                "--ci",
                "--password",
                "",
            ])
            .arg("--write-keys")
            .arg(&key)
            .output()
            .unwrap();
        assert!(generate.status.success());
        let sign = Command::new("pnpm")
            .current_dir(&root)
            .env_remove("TAURI_SIGNING_PRIVATE_KEY")
            .env_remove("TAURI_SIGNING_PRIVATE_KEY_PATH")
            .args([
                "--filter",
                "@open-faktura/desktop",
                "exec",
                "tauri",
                "signer",
                "sign",
                "--password",
                "",
                "--app-version",
                "999.0.0",
                "--private-key-path",
            ])
            .arg(&key)
            .arg(&artifact)
            .output()
            .unwrap();
        assert!(sign.status.success());
        let public_key = std::fs::read_to_string(temp.0.join("throwaway.key.pub")).unwrap();
        let signature = std::fs::read_to_string(temp.0.join("synthetic.bin.sig")).unwrap();
        assert!(config(
            Some("https://updates.example.test/latest.json"),
            Some(&public_key)
        )
        .unwrap()
        .is_some());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let manifest = serde_json::json!({"version":"999.0.0","url":format!("http://{address}/synthetic.bin"),"signature":signature});
        let manifest = serde_json::to_vec(&manifest).unwrap();
        let server = tokio::spawn(async move {
            for data in [manifest, bytes.to_vec()] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    let mut chunk = [0; 1024];
                    let n = socket.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..n]);
                    assert!(request.len() <= 8192);
                    if request.windows(4).any(|v| v == b"\r\n\r\n") {
                        break;
                    }
                }
                let headers=format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n",data.len());
                socket.write_all(headers.as_bytes()).await.unwrap();
                socket.write_all(&data).await.unwrap();
            }
        });
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        let shipped: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let mut shipped_updater = shipped["plugins"]["updater"].clone();
        shipped_updater["pubkey"] = public_key.trim().into();
        context
            .config_mut()
            .plugins
            .0
            .insert("updater".into(), shipped_updater);
        let app = tauri::test::mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        let update = app
            .updater_builder()
            .endpoints(vec![format!("http://{address}/latest.json")
                .parse()
                .unwrap()])
            .unwrap()
            .executable_path(&artifact)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
            .restore_update(
                fetch_manifest(
                    &format!("http://{address}/latest.json").parse().unwrap(),
                    true,
                )
                .await
                .unwrap()
                .unwrap(),
            )
            .unwrap()
            .unwrap();
        assert!(app.updater_builder().endpoints(vec![format!("http://{address}/latest.json").parse().unwrap()]).unwrap().executable_path(&artifact).build().unwrap().restore_update(serde_json::json!({"version":"0.0.0","url":format!("http://{address}/synthetic.bin"),"signature":signature})).unwrap().is_none());
        for target in [
            "darwin-aarch64-app",
            "darwin-x86_64-app",
            "windows-x86_64-nsis",
            "linux-x86_64-appimage",
        ] {
            let raw = serde_json::json!({"version":"999.0.0","platforms":{target:{"url":format!("http://{address}/synthetic.bin"),"signature":signature}}});
            let restored = app
                .updater_builder()
                .target(target)
                .endpoints(vec![format!("http://{address}/latest.json")
                    .parse()
                    .unwrap()])
                .unwrap()
                .executable_path(&artifact)
                .build()
                .unwrap()
                .restore_update(raw)
                .unwrap()
                .unwrap();
            assert_eq!(restored.target, target);
            assert!(restored.verify(bytes).is_ok());
        }
        let downloaded = download(&update, true).await.unwrap();
        assert_eq!(downloaded, bytes);
        assert!(update.verify(b"tampered bytes").is_err());
        let mut wrong = update.clone();
        wrong.version = "999.0.1".into();
        assert!(wrong.verify(bytes).is_err());
        let mut bad = update.clone();
        bad.signature = "not-a-signature".into();
        assert!(bad.verify(bytes).is_err());
        let legacy = Command::new("pnpm")
            .current_dir(&root)
            .env_remove("TAURI_SIGNING_PRIVATE_KEY")
            .env_remove("TAURI_SIGNING_PRIVATE_KEY_PATH")
            .args([
                "--filter",
                "@open-faktura/desktop",
                "exec",
                "tauri",
                "signer",
                "sign",
                "--password",
                "",
                "--private-key-path",
            ])
            .arg(&key)
            .arg(&artifact)
            .output()
            .unwrap();
        assert!(legacy.status.success());
        let legacy_signature = std::fs::read_to_string(temp.0.join("synthetic.bin.sig")).unwrap();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(legacy_signature.trim())
            .unwrap();
        assert!(!String::from_utf8(decoded).unwrap().contains("\tversion:"));
        let mut missing_version = update.clone();
        missing_version.signature = legacy_signature;
        assert!(missing_version.verify(bytes).is_err());
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        // Never call install: no real OS package, executable or application is changed.
        assert_eq!(std::fs::read(&artifact).unwrap(), bytes);
    }
    #[tokio::test]
    async fn bounded_manifest_content_length_chunked_and_no_content() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for mode in 0..3 {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let (mut s, _) = listener.accept().await.unwrap();
                let mut header = Vec::new();
                loop {
                    let mut b = [0; 1024];
                    let n = s.read(&mut b).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    header.extend_from_slice(&b[..n]);
                    assert!(header.len() < 8192);
                    if header.windows(4).any(|b| b == b"\r\n\r\n") {
                        break;
                    }
                }
                if mode == 0 {
                    s.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 524289\r\nConnection: close\r\n\r\n",
                    )
                    .await
                    .unwrap();
                } else if mode == 1 {
                    s.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
                    let body = vec![b' '; 524289];
                    let _ = s
                        .write_all(format!("{:x}\r\n", body.len()).as_bytes())
                        .await;
                    let _ = s.write_all(&body).await;
                    let _ = s.write_all(b"\r\n0\r\n\r\n").await;
                } else {
                    s.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                        .await
                        .unwrap();
                }
            });
            let result = fetch_manifest(
                &format!("http://{address}/latest.json").parse().unwrap(),
                true,
            )
            .await;
            if mode == 2 {
                assert!(result.unwrap().is_none())
            } else {
                assert!(result.is_err())
            }
            server.await.unwrap();
        }
    }
}
