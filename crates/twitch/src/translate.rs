//! C17 Translation through Google's free endpoint (D11), no key, and Microsoft's free
//! one when Google refuses (C33). One module, so the services can be swapped. Several
//! texts go in one request (Google wants a GET; its POST ends at a captcha).

use serde_json::Value;

use crate::http::json;
use crate::{Error, Http, Result};

const URL: &str = "https://translate.googleapis.com/translate_a/t";
const MICROSOFT: &str = "https://edge.microsoft.com/translate/translatetext";
/// Longer texts are cut, like moltorino does.
const MAX_CHARS: usize = 1200;
/// Characters per request, so the URL stays short.
const BATCH_CHARS: usize = 1500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Translated {
    pub text: String,
    /// What Google thinks it was, e.g. `de`.
    pub from: String,
}

/// Translates each text into `to` (`en`, `de`, …), in order. Google first, then
/// Microsoft.
pub async fn translate(http: &Http, texts: &[String], to: &str) -> Result<Vec<Translated>> {
    match google(http, texts, to).await {
        Ok(out) => Ok(out),
        Err(_) => microsoft(http, texts, to).await,
    }
}

async fn microsoft(http: &Http, texts: &[String], to: &str) -> Result<Vec<Translated>> {
    let body: Vec<String> = texts
        .iter()
        .map(|t| escape(&t.chars().take(MAX_CHARS).collect::<String>()))
        .collect();
    let req = http
        .0
        .post(MICROSOFT)
        .query(&[("isEnterpriseClient", "false"), ("to", to)])
        .json(&body);
    let raw: Vec<Value> = json(req).await?;
    let out: Vec<Translated> = raw
        .iter()
        .map(|r| Translated {
            text: unescape(
                r.pointer("/translations/0/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
            from: r
                .pointer("/detectedLanguage/language")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        })
        .collect();
    if out.len() != texts.len() {
        return Err(Error::Api {
            status: 200,
            message: "The translation came back incomplete.".into(),
        });
    }
    Ok(out)
}

/// Microsoft reads HTML.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

async fn google(http: &Http, texts: &[String], to: &str) -> Result<Vec<Translated>> {
    let mut out = Vec::with_capacity(texts.len());
    let mut batch: Vec<String> = Vec::new();
    let mut size = 0;
    for t in texts {
        let t: String = t.chars().take(MAX_CHARS).collect();
        let n = t.chars().count();
        if !batch.is_empty() && size + n > BATCH_CHARS {
            out.extend(one(http, &std::mem::take(&mut batch), to).await?);
            size = 0;
        }
        size += n;
        batch.push(t);
    }
    if !batch.is_empty() {
        out.extend(one(http, &batch, to).await?);
    }
    Ok(out)
}

async fn one(http: &Http, texts: &[String], to: &str) -> Result<Vec<Translated>> {
    let mut query: Vec<(&str, &str)> = vec![("client", "gtx"), ("sl", "auto"), ("tl", to)];
    query.extend(texts.iter().map(|t| ("q", t.as_str())));
    let body: Value = json(http.0.get(URL).query(&query)).await?;
    let out = parse(&body);
    if out.len() != texts.len() {
        return Err(Error::Api {
            status: 200,
            message: "The translation came back incomplete.".into(),
        });
    }
    Ok(out)
}

/// `[["Hello","de"], …]`; a fixed source language gives plain strings.
fn parse(body: &Value) -> Vec<Translated> {
    body.as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| match item {
                    Value::Array(pair) => Translated {
                        text: pair
                            .first()
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        from: pair.get(1).and_then(Value::as_str).unwrap_or_default().to_owned(),
                    },
                    other => Translated {
                        text: other.as_str().unwrap_or_default().to_owned(),
                        from: String::new(),
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_round_trip() {
        assert_eq!(unescape(&escape("a < b & c")), "a < b & c");
    }

    #[test]
    fn batches_parse() {
        let v: Value =
            serde_json::from_str(r#"[["Hello World","de"],["hello world","fr"],["hello","en"]]"#).unwrap();
        let out = parse(&v);
        assert_eq!(out.len(), 3);
        assert_eq!(
            out[1],
            Translated {
                text: "hello world".into(),
                from: "fr".into()
            }
        );
        let flat: Value = serde_json::from_str(r#"["Hello"]"#).unwrap();
        assert_eq!(parse(&flat)[0].text, "Hello");
    }
}
