pub mod source;
pub mod tokens;

pub fn verdict(rule: &str, findings: Vec<String>) {
    assert!(findings.is_empty(), "{rule}: {} нарушений\n{}", findings.len(), findings.join("\n"));
}
