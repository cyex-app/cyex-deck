//! Link info through Chatterino's public resolver (D8). X2 uses the final URL after
//! redirects, so shortened links show where they really go. Phase 8 adds previews.

use serde::Deserialize;

use crate::http::json;
use crate::{Http, Result};

const URL: &str = "https://braize.pajlada.com/chatterino/link_resolver";

#[derive(Deserialize)]
struct Response {
    status: u16,
    link: Option<String>,
}

/// Where the link ends up. `None` if the resolver couldn't tell.
pub async fn final_url(http: &Http, url: &str) -> Result<Option<String>> {
    let encoded: String = url::form_urlencoded::byte_serialize(url.as_bytes()).collect();
    let res: Response = json(http.0.get(format!("{URL}/{encoded}"))).await?;
    Ok((res.status == 200).then_some(res.link).flatten())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let r: Response = serde_json::from_str(r#"{"status":200,"tooltip":"x","link":"https://example.com/a"}"#).unwrap();
        assert_eq!(r.link.as_deref(), Some("https://example.com/a"));
        let r: Response = serde_json::from_str(r#"{"status":404,"message":"no"}"#).unwrap();
        assert_eq!(r.link, None);
    }
}
