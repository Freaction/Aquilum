use super::{refused, Failure};

pub(crate) struct Head {
    pub method: String,
    pub target: String,
    version: u8,
    headers: Vec<(String, String)>,
}

pub enum Body {
    Sized(u64),
    Chunked,
}

impl Head {
    pub fn from_parsed(request: &httparse::Request) -> Self {
        Self {
            method: request.method.unwrap_or_default().to_owned(),
            target: request.path.unwrap_or_default().to_owned(),
            version: request.version.unwrap_or(1),
            headers: request
                .headers
                .iter()
                .map(|header| {
                    let value = String::from_utf8_lossy(header.value).into_owned();
                    (header.name.to_owned(), value)
                })
                .collect(),
        }
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn keeps_alive(&self) -> bool {
        let connection_says = |option: &str| {
            self.header("connection").is_some_and(|value| {
                value
                    .split(',')
                    .any(|item| item.trim().eq_ignore_ascii_case(option))
            })
        };
        if self.version >= 1 {
            !connection_says("close")
        } else {
            connection_says("keep-alive")
        }
    }

    pub fn expects_continue(&self) -> bool {
        self.header("expect")
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("100-continue"))
    }

    pub fn body(&self) -> Result<Body, Failure> {
        if let Some(coding) = self.header("transfer-encoding") {
            return if coding.trim().eq_ignore_ascii_case("chunked") {
                Ok(Body::Chunked)
            } else {
                Err(refused(501, "Поддерживается только Transfer-Encoding: chunked"))
            };
        }
        match self.header("content-length") {
            None => Ok(Body::Sized(0)),
            Some(length) => length
                .trim()
                .parse()
                .map(Body::Sized)
                .map_err(|_| refused(400, "Некорректный Content-Length")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Head;

    fn head(version: u8, connection: Option<&str>) -> Head {
        Head {
            method: "POST".to_owned(),
            target: "/".to_owned(),
            version,
            headers: connection
                .map(|value| ("Connection".to_owned(), value.to_owned()))
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn keeps_the_connection_as_the_http_version_and_the_client_ask() {
        assert!(head(1, None).keeps_alive());
        assert!(!head(1, Some("close")).keeps_alive());
        assert!(!head(1, Some("Upgrade, Close")).keeps_alive());
        assert!(!head(0, None).keeps_alive());
        assert!(head(0, Some("Keep-Alive")).keeps_alive());
    }
}
