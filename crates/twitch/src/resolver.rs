//! Link info through Chatterino's public resolver (D8): where a link really ends up (X2)
//! and a preview (1.52), the resolver's HTML turned into plain lines here so the UI
//! never renders HTML.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::http::json;
use crate::{Http, Result};

const URL: &str = "https://braize.pajlada.com/chatterino/link_resolver";

#[derive(Deserialize)]
struct Response {
    status: u16,
    link: Option<String>,
    tooltip: Option<String>,
    thumbnail: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LinkInfo {
    /// Where it ends up after redirects, when the resolver knows.
    pub link: Option<String>,
    /// Text lines; an empty line is a divider.
    pub lines: Vec<Vec<LinkText>>,
    /// https only.
    pub thumbnail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LinkText {
    pub text: String,
    pub bold: bool,
}

/// `None` if the resolver has nothing on it.
pub async fn resolve(http: &Http, url: &str) -> Result<Option<LinkInfo>> {
    let encoded: String = url::form_urlencoded::byte_serialize(url.as_bytes()).collect();
    let res: Response = json(http.0.get(format!("{URL}/{encoded}"))).await?;
    Ok(info(res))
}

fn info(res: Response) -> Option<LinkInfo> {
    if res.status != 200 {
        return None;
    }
    let html = res.tooltip.map(|t| percent_decode(&t)).unwrap_or_default();
    Some(LinkInfo {
        link: res.link,
        lines: lines(&html),
        thumbnail: res.thumbnail.map(|t| tidy_url(&t)).filter(|t| t.starts_with("https://") && !t.contains(char::is_whitespace)),
    })
}

/// Some thumbnails end in encoded line breaks.
fn tidy_url(url: &str) -> String {
    let mut t = url.trim();
    while let Some(rest) = ["%0A", "%0D", "%0a", "%0d", "%20"].iter().find_map(|s| t.strip_suffix(s)) {
        t = rest.trim_end();
    }
    t.to_owned()
}

fn percent_decode(s: &str) -> String {
    url::form_urlencoded::parse(format!("x={}", s.replace('+', "%2B")).as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default()
}

/// `<b>`, `<br>`, `<hr>` and `<li>` become bold parts, lines and dividers; other tags
/// go, their text stays.
fn lines(html: &str) -> Vec<Vec<LinkText>> {
    let mut out: Vec<Vec<LinkText>> = vec![Vec::new()];
    let mut bold = 0u32;
    let mut rest = html;
    let push_text = |out: &mut Vec<Vec<LinkText>>, text: &str, bold: bool| {
        let text = entities(&text.replace(['\n', '\r'], ""));
        if text.is_empty() {
            return;
        }
        let line = out.last_mut().expect("never empty");
        match line.last_mut() {
            Some(last) if last.bold == bold => last.text.push_str(&text),
            _ => line.push(LinkText { text, bold }),
        }
    };
    while let Some(lt) = rest.find('<') {
        push_text(&mut out, &rest[..lt], bold > 0);
        let Some(gt) = rest[lt..].find('>') else { break };
        let tag = rest[lt + 1..lt + gt].trim().to_ascii_lowercase();
        let name = tag.trim_start_matches('/').split([' ', '/']).next().unwrap_or("");
        let closing = tag.starts_with('/');
        match name {
            "b" | "strong" if closing => bold = bold.saturating_sub(1),
            "b" | "strong" => bold += 1,
            "br" | "li" | "div" | "p" if !out.last().is_some_and(Vec::is_empty) => out.push(Vec::new()),
            "hr" => {
                if !out.last().is_some_and(Vec::is_empty) {
                    out.push(Vec::new());
                }
                out.push(Vec::new());
            }
            _ => {}
        }
        rest = &rest[lt + gt + 1..];
    }
    push_text(&mut out, rest, bold > 0);
    for line in &mut out {
        if let Some(first) = line.first_mut() {
            first.text = first.text.trim_start().to_owned();
        }
        if let Some(last) = line.last_mut() {
            last.text = last.text.trim_end().to_owned();
        }
        line.retain(|p| !p.text.is_empty());
    }
    // A divider is one empty line between two with text.
    let mut tidy: Vec<Vec<LinkText>> = Vec::new();
    for line in out {
        if line.is_empty() && tidy.last().is_none_or(Vec::is_empty) {
            continue;
        }
        tidy.push(line);
    }
    while tidy.last().is_some_and(Vec::is_empty) {
        tidy.pop();
    }
    tidy
}

fn entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let end = tail.find(';').filter(|e| *e <= 10);
        let decoded = end.and_then(|e| {
            let name = &tail[1..e];
            let c = match name {
                "nbsp" => ' ',
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                _ => {
                    let n = name.strip_prefix("#x").map(|h| u32::from_str_radix(h, 16)).or_else(|| name.strip_prefix('#').map(str::parse))?;
                    char::from_u32(n.ok()?)?
                }
            };
            Some((c, e))
        });
        match decoded {
            Some((c, e)) => {
                out.push(c);
                rest = &tail[e + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Option<LinkInfo> {
        info(serde_json::from_str(json).unwrap())
    }

    #[test]
    fn previews() {
        let tip = "%3Cdiv%20style%3D%22text-align%3A%20left%3B%22%3E%3Cb%3Enope%3C%2Fb%3E%3Chr%3E%3Cb%3EClipped%20by%3A%3C%2Fb%3E%20DexM23%3Cbr%3E%3Cb%3EViews%3A%3C%2Fb%3E%2045%2C524%3C%2Fdiv%3E";
        let i = parse(&format!(r#"{{"status":200,"tooltip":"{tip}","thumbnail":"https://x/t.jpg%0A%0A","link":"https://clips.twitch.tv/a"}}"#)).unwrap();
        let text: Vec<String> = i.lines.iter().map(|l| l.iter().map(|p| p.text.as_str()).collect()).collect();
        assert_eq!(text, ["nope", "", "Clipped by: DexM23", "Views: 45,524"]);
        assert!(i.lines[2][0].bold && !i.lines[2][1].bold);
        assert_eq!(i.thumbnail.as_deref(), Some("https://x/t.jpg"));
        assert_eq!(i.link.as_deref(), Some("https://clips.twitch.tv/a"));
        assert!(parse(r#"{"status":404,"message":"no"}"#).is_none());
    }

    #[test]
    fn html_entities() {
        assert_eq!(entities("a&nbsp;&#8226;&nbsp;b &#34;q&#34; &amp; &bogus"), "a • b \"q\" & &bogus");
        let l = lines("<b>Twitch (service)&nbsp;•&nbsp;x</b><br>It &#34;streams&#34;.");
        assert_eq!(l.len(), 2);
        assert_eq!(l[1][0].text, "It \"streams\".");
    }
}
