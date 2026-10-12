use super::release::{ENDPOINT, PUBLIC_KEY, newer, verify};

#[test]
fn versions_compare_numerically() {
    assert!(newer("0.2.10", "0.2.9"));
    assert!(newer("v0.3.0", "0.2.6"));
    assert!(!newer("0.2.6", "0.2.6"));
    assert!(!newer("0.2.5", "0.2.6"));
}

#[test]
#[ignore]
fn verifies_real_release_signature() {
    let manifest: serde_json::Value = serde_json::from_reader(aquilum_core::web_agent::web_agent("updates").get(ENDPOINT).call().unwrap().into_reader()).unwrap();
    let platform = &manifest["platforms"]["windows-x86_64"];
    let mut data = Vec::new();
    std::io::Read::read_to_end(&mut aquilum_core::web_agent::web_agent("updates").get(platform["url"].as_str().unwrap()).call().unwrap().into_reader(), &mut data).unwrap();
    verify(&data, platform["signature"].as_str().unwrap(), PUBLIC_KEY).unwrap();
    data[100] ^= 1;
    assert!(verify(&data, platform["signature"].as_str().unwrap(), PUBLIC_KEY).is_err());
}
