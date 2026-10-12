//! 1.43 The GIF picker's search, straight from GIPHY with the key Twitch hands out, the
//! way twitch.tv does: 50 at a time, rated PG or lower, no ads.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::http::json;
use crate::{Http, Result};

const API: &str = "https://api.giphy.com/v1/gifs";
pub const PAGE: u32 = 50;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Gif {
    pub id: String,
    pub title: String,
    /// Small moving picture for the picker.
    pub preview: String,
    pub still: String,
    pub width: u32,
    pub height: u32,
    /// What gets sent.
    pub url: String,
}

/// `query` empty: trending.
pub async fn search(http: &Http, key: &str, query: &str, offset: u32, rating: &str) -> Result<Vec<Gif>> {
    let offset = offset.to_string();
    let limit = PAGE.to_string();
    let mut params = vec![
        ("api_key", key),
        ("limit", limit.as_str()),
        ("offset", offset.as_str()),
        ("rating", rating),
    ];
    let url = if query.trim().is_empty() {
        format!("{API}/trending")
    } else {
        params.push(("q", query.trim()));
        format!("{API}/search")
    };
    let page: Page = json(http.0.get(url).query(&params)).await?;
    Ok(page.data.into_iter().filter_map(RawGif::gif).collect())
}

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    data: Vec<RawGif>,
}

#[derive(Deserialize)]
struct RawGif {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    rating: String,
    #[serde(default)]
    is_ad: bool,
    images: RawImages,
}

#[derive(Deserialize)]
struct RawImages {
    original: Option<RawImage>,
    fixed_height: Option<RawImage>,
    fixed_height_still: Option<RawImage>,
}

#[derive(Deserialize)]
struct RawImage {
    url: Option<String>,
    webp: Option<String>,
    width: Option<String>,
    height: Option<String>,
}

impl RawGif {
    fn gif(self) -> Option<Gif> {
        if self.is_ad || !matches!(self.rating.as_str(), "y" | "g" | "pg") {
            return None;
        }
        let small = self.images.fixed_height?;
        Some(Gif {
            title: self.title.trim().to_owned(),
            preview: small.webp.clone().or(small.url.clone())?,
            still: self.images.fixed_height_still.and_then(|s| s.url)?,
            width: small.width.as_deref()?.parse().ok()?,
            height: small.height.as_deref()?.parse().ok()?,
            url: self.images.original?.url?,
            id: self.id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_filters() {
        let p: Page = serde_json::from_str(
            r#"{"data":[
            {"id":"a","title":" Hello GIF ","rating":"g","images":{"original":{"url":"https://media4.giphy.com/media/a/giphy.gif"},
              "fixed_height":{"url":"https://x/200.gif","webp":"https://x/200.webp","width":"356","height":"200"},
              "fixed_height_still":{"url":"https://x/200_s.gif"}}},
            {"id":"b","title":"ad","rating":"g","is_ad":true,"images":{}},
            {"id":"c","title":"spicy","rating":"r","images":{}}]}"#,
        )
        .unwrap();
        let gifs: Vec<Gif> = p.data.into_iter().filter_map(RawGif::gif).collect();
        assert_eq!(gifs.len(), 1);
        assert_eq!(
            (gifs[0].title.as_str(), gifs[0].width, gifs[0].preview.as_str()),
            ("Hello GIF", 356, "https://x/200.webp")
        );
    }
}
