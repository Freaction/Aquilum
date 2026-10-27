use base64::Engine;

pub(super) const ENDPOINT: &str = "https://github.com/Freaction/Aquilum/releases/latest/download/latest.json";
pub(super) const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEI4QTY0MzM1MDczQUNDMTQKUldRVXpEb0hOVU9tdUFCWnVMRWlBVTRGWmlFOW0wd2VteVlXd3luMFAyMmczWktQbkRvVmhTQSsK";

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub(super) url: String,
    pub(super) signature: String,
}

fn target() -> Option<String> {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "android") {
        "android"
    } else {
        return None;
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        return None;
    };
    Some(format!("{os}-{arch}"))
}

fn version(text: &str) -> Vec<u64> {
    text.trim().trim_start_matches('v').split(['.', '-', '+']).map_while(|part| part.parse().ok()).collect()
}

pub(super) fn newer(candidate: &str, current: &str) -> bool {
    version(candidate) > version(current)
}

pub(super) fn find() -> Result<Option<Release>, String> {
    let agent = aquilum_core::web_agent::web_agent("updates");
    let manifest: serde_json::Value = serde_json::from_reader(agent.get(ENDPOINT).call().map_err(|e| e.to_string())?.into_reader()).map_err(|e| e.to_string())?;
    let version = manifest["version"].as_str().ok_or("в latest.json нет версии")?.to_owned();
    if !newer(&version, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }
    let target = target().ok_or("платформа не поддерживается")?;
    let platform = &manifest["platforms"][&target];
    let url = platform["url"].as_str().ok_or_else(|| format!("в latest.json нет сборки для {target}"))?.to_owned();
    let signature = platform["signature"].as_str().ok_or("в latest.json нет подписи")?.to_owned();
    Ok(Some(Release { version, url, signature }))
}

fn decoded(text: &str) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(text.trim()).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

pub(super) fn verify(data: &[u8], signature: &str, key: &str) -> Result<(), String> {
    let key = minisign_verify::PublicKey::decode(&decoded(key)?).map_err(|e| e.to_string())?;
    let signature = minisign_verify::Signature::decode(&decoded(signature)?).map_err(|e| e.to_string())?;
    key.verify(data, &signature, true).map_err(|e| e.to_string())
}
