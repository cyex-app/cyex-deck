//! 1.43 review: `CYEX_GIFS_MOCK=1` (mocks builds, test data folder only) shows the GIFs
//! tab in every chat with drawn tiles, so it can be seen without a Tier 2 account. Nothing
//! reaches Twitch or GIPHY. Sending answers a 30 s wait; search "unavailable" answers
//! `TEMPORARILY_UNAVAILABLE`.

use cyex_twitch::giphy::Gif;
use cyex_twitch::gql::gifs::GifSent;
use tauri::AppHandle;

pub fn on(app: &AppHandle) -> bool {
    crate::paths::webview(app).is_some() && std::env::var("CYEX_GIFS_MOCK").is_ok()
}

const COLORS: [&str; 6] = ["145aff", "e5484d", "30a46c", "f5a524", "8e4ec6", "12a594"];
const SIZES: [(u32, u32); 4] = [(200, 200), (200, 112), (200, 260), (200, 150)];

pub fn search(query: &str, offset: u32) -> Vec<Gif> {
    if offset >= 48 {
        return Vec::new();
    }
    let label = if query.is_empty() { "Trending" } else { query };
    (offset..offset + 24)
        .map(|i| {
            let (w, h) = SIZES[i as usize % SIZES.len()];
            let color = COLORS[i as usize % COLORS.len()];
            let svg = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}'><rect width='100%' height='100%' fill='#{color}'/>\
                 <text x='50%' y='50%' fill='white' font-family='sans-serif' font-size='22' text-anchor='middle'>{i}</text></svg>"
            );
            let src = format!(
                "data:image/svg+xml,{}",
                svg.replace('%', "%25").replace('#', "%23").replace('<', "%3C").replace('>', "%3E").replace(' ', "%20")
            );
            Gif {
                id: format!("mock{i}"),
                title: format!("{label} GIF {i}"),
                preview: src.clone(),
                still: src.clone(),
                width: w,
                height: h,
                url: src,
            }
        })
        .collect()
}

pub fn send(search: &str) -> GifSent {
    if search == "unavailable" {
        return GifSent {
            error: Some("TEMPORARILY_UNAVAILABLE".into()),
            wait_secs: None,
        };
    }
    GifSent {
        error: None,
        wait_secs: Some(30),
    }
}
