//! V106-24 Panel images that are fully see-through (streamers use them as spacers): read
//! the small ones, check their pixels (`sys::thumb::all_clear`) and drop the blank ones.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cyex_twitch::Http;
use cyex_twitch::gql::About;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::{Instant, timeout_at};

/// Bigger files are visible art; marc's spacers are at most 1.411 B.
const MAX_BYTES: usize = 4096;
const PARALLEL: usize = 8;
const BUDGET: Duration = Duration::from_secs(2);
const CACHE_MAX: usize = 1000;

/// URL -> fully see-through, for this run of the app.
static SEEN: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

/// Removes `about`'s blank panel images; a panel left with nothing goes. Anything that
/// can't be read in time counts as visible.
pub async fn drop_blank(http: &Http, about: &mut About) {
    let urls = about.panel_images();
    let (mut blank, unknown) = with_seen(|seen| split(seen, &urls));
    if !unknown.is_empty() {
        let (got, files): (Vec<String>, Vec<Vec<u8>>) = fetch(http, unknown).await.into_iter().unzip();
        let clear = tokio::task::spawn_blocking(move || crate::sys::thumb::all_clear(files))
            .await
            .unwrap_or_default();
        with_seen(|seen| {
            for (url, clear) in got.into_iter().zip(clear) {
                // Not a PNG, or one that won't decode: shown, and not asked again.
                let clear = clear.unwrap_or(false);
                if clear {
                    blank.insert(url.clone());
                }
                remember(seen, url, clear);
            }
        });
    }
    if !blank.is_empty() {
        about.without_images(&blank);
    }
}

/// The small files among `urls`, at most `PARALLEL` at a time, within `BUDGET`. A file
/// too big comes back empty (visible, never a PNG); failures and late ones are left out.
async fn fetch(http: &Http, urls: Vec<String>) -> Vec<(String, Vec<u8>)> {
    let gate = Arc::new(Semaphore::new(PARALLEL));
    let mut set = JoinSet::new();
    for url in urls {
        let (http, gate) = (http.clone(), gate.clone());
        set.spawn(async move {
            let _permit = gate.acquire_owned().await.ok()?;
            let bytes = http.get_small(&url, MAX_BYTES).await.ok()?;
            Some((url, bytes.unwrap_or_default()))
        });
    }
    let end = Instant::now() + BUDGET;
    let mut out = Vec::new();
    while let Ok(Some(done)) = timeout_at(end, set.join_next()).await {
        if let Ok(Some(file)) = done {
            out.push(file);
        }
    }
    out
}

fn with_seen<T>(f: impl FnOnce(&mut HashMap<String, bool>) -> T) -> T {
    let mut seen = SEEN.lock().unwrap_or_else(|p| p.into_inner());
    f(seen.get_or_insert_with(HashMap::new))
}

/// `urls` split into the ones known blank and the ones not seen yet.
fn split(seen: &HashMap<String, bool>, urls: &[String]) -> (HashSet<String>, Vec<String>) {
    let mut blank = HashSet::new();
    let mut unknown = Vec::new();
    for url in urls {
        match seen.get(url) {
            Some(true) => {
                blank.insert(url.clone());
            }
            Some(false) => {}
            None => unknown.push(url.clone()),
        }
    }
    (blank, unknown)
}

/// Caches one answer; a full cache starts over.
fn remember(seen: &mut HashMap<String, bool>, url: String, clear: bool) {
    if seen.len() >= CACHE_MAX && !seen.contains_key(&url) {
        seen.clear();
    }
    seen.insert(url, clear);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn known_answers_skip_the_fetch() {
        let mut seen = HashMap::new();
        remember(&mut seen, "a".into(), true);
        remember(&mut seen, "b".into(), false);
        let (blank, unknown) = split(&seen, &urls(&["a", "b", "c"]));
        assert_eq!(blank, HashSet::from(["a".to_owned()]));
        assert_eq!(unknown, urls(&["c"]));
    }

    #[test]
    fn a_full_cache_starts_over() {
        let mut seen = HashMap::new();
        for i in 0..CACHE_MAX {
            remember(&mut seen, i.to_string(), false);
        }
        remember(&mut seen, "0".into(), true);
        assert_eq!(seen.len(), CACHE_MAX);
        remember(&mut seen, "new".into(), true);
        assert_eq!(seen.len(), 1);
        assert_eq!(seen.get("new"), Some(&true));
    }
}
