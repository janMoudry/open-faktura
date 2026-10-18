#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod updater;
use tauri::Manager;
fn main() {
    let config = updater::config(
        option_env!("OPEN_FAKTURA_UPDATER_ENDPOINT"),
        option_env!("OPEN_FAKTURA_UPDATER_PUBLIC_KEY"),
    )
    .expect("invalid desktop updater build configuration");
    tauri::Builder::default()
        .setup(move |app| {
            if let Some(config) = &config {
                app.handle().plugin(
                    tauri_plugin_updater::Builder::new()
                        .pubkey(config.public_key.clone())
                        .build(),
                )?;
            }
            app.manage(updater::Updates::new(config));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            updater::desktop_updater_status,
            updater::desktop_updater_check,
            updater::desktop_updater_install
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Open Faktura");
}
