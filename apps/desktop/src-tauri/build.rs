fn main() {
    println!("cargo:rerun-if-env-changed=OPEN_FAKTURA_UPDATER_ENDPOINT");
    println!("cargo:rerun-if-env-changed=OPEN_FAKTURA_UPDATER_PUBLIC_KEY");
    tauri_build::build()
}
