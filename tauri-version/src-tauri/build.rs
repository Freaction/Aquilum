fn main() {
    println!("cargo:rerun-if-changed=tauri.conf.json");
    let config = std::fs::read_to_string("tauri.conf.json").expect("tauri.conf.json читается");
    let config: serde_json::Value = serde_json::from_str(&config).expect("tauri.conf.json — валидный JSON");
    let identifier = config["identifier"].as_str().expect("в tauri.conf.json задан identifier");
    println!("cargo:rustc-env=AQUILUM_APP_IDENTIFIER={identifier}");
    tauri_build::build()
}
