use super::http::{HttpClient, HttpError};
use super::models::WikixivHit;
use serde::Deserialize;

const SNIPPET_CHARS: usize = 220;
const MAX_INTITLE: usize = 2;

#[derive(Debug, Deserialize)]
struct RestPage {
    title: Option<String>,
    extract: Option<String>,
    content_urls: Option<ContentUrls>,
    #[serde(rename = "type")]
    page_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentUrls {
    desktop: Option<DesktopUrl>,
}

#[derive(Debug, Deserialize)]
struct DesktopUrl {
    page: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    query: Option<SearchQuery>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    search: Option<Vec<SearchItem>>,
}

#[derive(Debug, Deserialize)]
struct SearchItem {
    title: String,
    snippet: Option<String>,
}


pub fn summary(client: &HttpClient, lang: &str, title: &str) -> Result<Option<WikixivHit>, HttpError> {
    let title = title.trim();
    if title.is_empty() {
        return Ok(None);
    }
    let url = format!(
        "https://{lang}.wikipedia.org/api/rest_v1/page/summary/{}",
        encode_path(title),
    );
    match client.get_text(&url) {
        Ok(body) => Ok(parse_page(lang, &body)),
        Err(HttpError::Status(404, _)) => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn search_title(
    client: &HttpClient,
    lang: &str,
    title: &str,
) -> Result<Option<WikixivHit>, HttpError> {
    let title = title.trim();
    if title.is_empty() {
        return Ok(None);
    }
    let url = format!(
        "https://{lang}.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit=1&srprop=snippet&format=json",
        encode_query(&format!("\"{title}\"")),
    );
    let body = client.get_text(&url)?;
    let item = serde_json::from_str::<SearchResponse>(&body)
        .map_err(|e| HttpError::Io(e.to_string()))?
        .query
        .and_then(|query| query.search)
        .and_then(|mut results| results.drain(..).next());
    Ok(item.map(|item| search_item_to_hit(lang, item)))
}

pub fn search_intitle(
    client: &HttpClient,
    lang: &str,
    term: &str,
) -> Result<Vec<WikixivHit>, HttpError> {
    let term = term.trim();
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let url = format!(
        "https://{lang}.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit={MAX_INTITLE}&srprop=snippet&format=json",
        encode_query(&format!("intitle:{term}")),
    );
    let body = client.get_text(&url)?;
    let parsed: SearchResponse =
        serde_json::from_str(&body).map_err(|e| HttpError::Io(e.to_string()))?;
    Ok(parsed
        .query
        .and_then(|q| q.search)
        .unwrap_or_default()
        .into_iter()
        .map(|item| search_item_to_hit(lang, item))
        .collect())
}

fn search_item_to_hit(lang: &str, item: SearchItem) -> WikixivHit {
    WikixivHit {
        url: format!("https://{lang}.wikipedia.org/wiki/{}", encode_path(&item.title)),
        title: item.title,
        snippet: truncate(strip_html(item.snippet.as_deref().unwrap_or_default())),
        from_filename: false,
        matched_terms: Vec::new(),
    }
}

fn parse_page(lang: &str, body: &str) -> Option<WikixivHit> {
    let page: RestPage = serde_json::from_str(body).ok()?;
    page_to_hit(lang, page)
}

fn page_to_hit(lang: &str, page: RestPage) -> Option<WikixivHit> {
    if page.page_type.as_deref() == Some("disambiguation") {
        return None;
    }
    let title = page.title.filter(|t| !t.is_empty())?;
    let url = page
        .content_urls
        .and_then(|u| u.desktop)
        .and_then(|d| d.page)
        .unwrap_or_else(|| {
            format!("https://{lang}.wikipedia.org/wiki/{}", encode_path(&title))
        });
    Some(WikixivHit {
        title,
        url,
        snippet: truncate(page.extract.unwrap_or_default()),
        from_filename: false,
        matched_terms: Vec::new(),
    })
}

fn truncate(text: impl AsRef<str>) -> String {
    let cleaned = text.as_ref().split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    for ch in cleaned.chars() {
        if out.chars().count() >= SNIPPET_CHARS {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
}

fn encode_path(title: &str) -> String {
    encode_query(&title.trim().replace(' ', "_"))
}

fn encode_query(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_disambiguation() {
        let body = r#"{"type":"disambiguation","title":"Mercury","extract":"Mercury may refer to:"}"#;
        assert!(parse_page("en", body).is_none());
    }

    #[test]
    fn title_search_uses_supported_wikipedia_parameters() {
        let url = "https://ru.wikipedia.org/w/api.php?action=query&list=search&srsearch=%22Макар%20Чудра%22&srlimit=1&srprop=snippet&format=json";
        assert!(!url.contains("srwhat=title"));
    }
}
