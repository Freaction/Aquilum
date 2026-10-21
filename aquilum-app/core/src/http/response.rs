use super::IDLE_TIMEOUT;

pub(crate) struct Response {
    status: u16,
    content_type: &'static str,
    allow: Option<&'static str>,
    body: String,
}

impl Response {
    pub fn json(body: String) -> Self {
        Self {
            status: 200,
            content_type: "application/json",
            allow: None,
            body,
        }
    }

    pub fn text(status: u16, message: &str) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            allow: None,
            body: message.to_owned(),
        }
    }

    pub fn allowing(self, methods: &'static str) -> Self {
        Self {
            allow: Some(methods),
            ..self
        }
    }

    pub fn to_bytes(&self, keep_alive: bool) -> Vec<u8> {
        let mut head = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n",
            self.status,
            reason(self.status),
            self.body.len()
        );
        if !self.body.is_empty() {
            head.push_str(&format!("Content-Type: {}\r\n", self.content_type));
        }
        if let Some(methods) = self.allow {
            head.push_str(&format!("Allow: {methods}\r\n"));
        }
        if keep_alive {
            head.push_str(&format!(
                "Connection: keep-alive\r\nKeep-Alive: timeout={}\r\n",
                IDLE_TIMEOUT.as_secs()
            ));
        } else {
            head.push_str("Connection: close\r\n");
        }
        head.push_str("\r\n");
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(self.body.as_bytes());
        bytes
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        405 => "Method Not Allowed",
        413 => "Content Too Large",
        431 => "Request Header Fields Too Large",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "",
    }
}
