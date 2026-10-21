use std::time::Duration;
use ureq::{Agent, AgentBuilder};

const TIMEOUT: Duration = Duration::from_secs(5);

pub fn web_agent(purpose: &str) -> Agent {
    agent(purpose, TIMEOUT)
}

pub fn agent(purpose: &str, timeout: Duration) -> Agent {
    AgentBuilder::new()
        .timeout_connect(TIMEOUT)
        .timeout_read(timeout)
        .user_agent(&format!(
            "Aquilum/{} (local-first notes; {purpose})",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
}
