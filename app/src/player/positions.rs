//! P5 Where each VOD was left, so it resumes there. One small file, the last 300 VODs.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

const KEEP: usize = 300;
/// Closer to the end than this counts as watched: the next time starts over.
const DONE_SECONDS: f64 = 30.0;

pub struct Positions {
    file: Option<PathBuf>,
    map: Mutex<HashMap<String, Saved>>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Saved {
    at: f64,
    /// Unix seconds, to forget the oldest.
    when: i64,
}

impl Positions {
    pub fn load(file: Option<PathBuf>) -> Self {
        let map = file
            .as_ref()
            .and_then(|f| fs::read_to_string(f).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self {
            file,
            map: Mutex::new(map),
        }
    }

    pub fn get(&self, id: &str) -> Option<f64> {
        self.lock().get(id).map(|s| s.at)
    }

    pub fn save(&self, id: &str, at: f64, length: f64, now: i64) {
        let mut map = self.lock();
        if !at.is_finite() || at < 5.0 || (length > 0.0 && at > length - DONE_SECONDS) {
            map.remove(id);
        } else {
            map.insert(id.to_owned(), Saved { at, when: now });
        }
        if map.len() > KEEP {
            let mut ages: Vec<(String, i64)> = map.iter().map(|(k, v)| (k.clone(), v.when)).collect();
            ages.sort_by_key(|(_, when)| *when);
            for (k, _) in ages.iter().take(map.len() - KEEP) {
                map.remove(k);
            }
        }
        if let Some(f) = &self.file
            && let Ok(json) = serde_json::to_string(&*map)
            && let Err(e) = fs::write(f, json)
        {
            eprintln!("player: couldn't save VOD positions: {e}");
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Saved>> {
        self.map.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_spot_until_the_end() {
        let p = Positions::load(None);
        p.save("1", 600.0, 3600.0, 1);
        assert_eq!(p.get("1"), Some(600.0));
        p.save("1", 3590.0, 3600.0, 2);
        assert_eq!(p.get("1"), None, "watched to the end");
        p.save("2", 2.0, 3600.0, 3);
        assert_eq!(p.get("2"), None, "barely started");
    }

    #[test]
    fn forgets_the_oldest() {
        let p = Positions::load(None);
        for i in 0..=KEEP {
            p.save(&i.to_string(), 100.0, 0.0, i as i64);
        }
        assert_eq!(p.get("0"), None);
        assert_eq!(p.get(&KEEP.to_string()), Some(100.0));
    }
}
