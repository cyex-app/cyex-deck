//! P5 Where each VOD was left, so it resumes there, and V107-Mv how far each was watched,
//! for the cards. One small local file, the last 500 VODs; never in exports.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

const KEEP: usize = 500;
/// Closer to the end than this: the next time starts over.
const DONE_SECONDS: f64 = 30.0;
/// V107-Mv This much seen counts as watched, for good.
const WATCHED: f64 = 0.9;
/// Less than this isn't worth resuming.
const MIN_SECONDS: f64 = 5.0;

pub struct Positions {
    file: Option<PathBuf>,
    map: Mutex<HashMap<String, Saved>>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Saved {
    at: f64,
    /// Seconds; 0 when unknown (files from before v1.07).
    #[serde(default)]
    length: f64,
    #[serde(default)]
    watched: bool,
    /// Unix seconds, to forget the oldest.
    when: i64,
}

impl Saved {
    fn near_end(&self) -> bool {
        self.length > 0.0 && self.at > self.length - DONE_SECONDS
    }
}

/// V107-Mv How far a VOD was watched, for its card.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VodProgress {
    pub id: String,
    /// 0 to 1.
    pub part: f64,
    pub watched: bool,
}

/// Written beside, then moved over: a crash mid-write can't cut the file short and lose all 500.
fn write_whole(file: &std::path::Path, json: &str) -> std::io::Result<()> {
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, file)
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

    /// Where to resume, if anywhere.
    pub fn get(&self, id: &str) -> Option<f64> {
        self.lock()
            .get(id)
            .filter(|s| s.at >= MIN_SECONDS && !s.near_end())
            .map(|s| s.at)
    }

    pub fn save(&self, id: &str, at: f64, length: f64, now: i64) {
        if !at.is_finite() {
            return;
        }
        let mut map = self.lock();
        let before = map.get(id).copied();
        let length = if length.is_finite() && length > 0.0 {
            length
        } else {
            before.map_or(0.0, |b| b.length)
        };
        // A short VOD's last 30 s count too.
        let watched = before.is_some_and(|b| b.watched)
            || (length > 0.0 && (at >= length * WATCHED || at > length - DONE_SECONDS));
        if at < MIN_SECONDS && !watched {
            map.remove(id);
        } else {
            map.insert(
                id.to_owned(),
                Saved {
                    at: at.max(0.0),
                    length,
                    watched,
                    when: now,
                },
            );
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
            && let Err(e) = write_whole(f, &json)
        {
            eprintln!("player: couldn't save VOD positions: {e}");
        }
    }

    /// V107-Mv The VODs among `ids` that were started.
    pub fn progress(&self, ids: &[String]) -> Vec<VodProgress> {
        let map = self.lock();
        ids.iter()
            .filter_map(|id| {
                let s = map.get(id)?;
                let part = if s.near_end() {
                    1.0
                } else if s.length > 0.0 {
                    (s.at / s.length).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                Some(VodProgress {
                    id: id.clone(),
                    part,
                    watched: s.watched,
                })
            })
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Saved>> {
        self.map.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(p: &Positions, id: &str) -> Option<VodProgress> {
        p.progress(&[id.to_owned()]).pop()
    }

    #[test]
    fn keeps_the_spot_until_the_end() {
        let p = Positions::load(None);
        p.save("1", 600.0, 3600.0, 1);
        assert_eq!(p.get("1"), Some(600.0));
        p.save("1", 3590.0, 3600.0, 2);
        assert_eq!(p.get("1"), None, "watched to the end starts over");
        p.save("2", 2.0, 3600.0, 3);
        assert_eq!(p.get("2"), None, "barely started");
        assert_eq!(one(&p, "2"), None);
    }

    #[test]
    fn watched_from_ninety_percent_and_stays() {
        let p = Positions::load(None);
        p.save("1", 3000.0, 3600.0, 1);
        assert!(!one(&p, "1").unwrap().watched, "83 %");
        p.save("1", 3240.0, 3600.0, 2);
        let w = one(&p, "1").unwrap();
        assert!(w.watched, "90 %");
        assert!((w.part - 0.9).abs() < 1e-9);
        assert_eq!(p.get("1"), Some(3240.0), "still resumes before the last 30 s");
        // Watched again from the start: stays watched, even back under 5 s.
        p.save("1", 1.0, 0.0, 3);
        let w = one(&p, "1").unwrap();
        assert!(w.watched);
        assert_eq!(p.get("1"), None);
    }

    #[test]
    fn the_end_fills_the_bar() {
        let p = Positions::load(None);
        p.save("1", 3590.0, 3600.0, 1);
        assert_eq!(one(&p, "1"), Some(VodProgress { id: "1".into(), part: 1.0, watched: true }));
        assert_eq!(p.progress(&["1".into(), "none".into()]).len(), 1);
    }

    #[test]
    fn forgets_the_oldest() {
        let p = Positions::load(None);
        for i in 0..=KEEP {
            p.save(&i.to_string(), 100.0, 0.0, i as i64);
        }
        assert_eq!(p.get("0"), None);
        assert_eq!(p.get(&KEEP.to_string()), Some(100.0));
        assert_eq!(p.lock().len(), KEEP);
    }

    #[test]
    fn survives_a_restart_and_reads_old_files() {
        let dir = std::env::temp_dir().join(format!("cyex-positions-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("vod-positions.json");
        fs::write(&file, r#"{"old":{"at":120.0,"when":1}}"#).unwrap();
        let p = Positions::load(Some(file.clone()));
        assert_eq!(p.get("old"), Some(120.0), "pre-v1.07 entry");
        p.save("1", 3300.0, 3600.0, 2);
        assert!(!file.with_extension("json.tmp").exists(), "no temp file left");
        let again = Positions::load(Some(file));
        assert!(one(&again, "1").unwrap().watched);
        assert_eq!(again.get("1"), Some(3300.0));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_file_over_the_cap_trims_on_the_next_save() {
        let dir = std::env::temp_dir().join(format!("cyex-positions-cap-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("vod-positions.json");
        let many: HashMap<String, Saved> = (0..650)
            .map(|i| (i.to_string(), Saved { at: 100.0, length: 3600.0, watched: false, when: i }))
            .collect();
        fs::write(&file, serde_json::to_string(&many).unwrap()).unwrap();
        let p = Positions::load(Some(file.clone()));
        assert_eq!(p.progress(&["0".into(), "649".into()]).len(), 2, "all read at load");
        p.save("new", 100.0, 3600.0, 1000);
        let again = Positions::load(Some(file));
        assert_eq!(again.lock().len(), KEEP);
        assert_eq!(again.get("new"), Some(100.0));
        assert_eq!(again.get("0"), None, "the oldest went");
        assert_eq!(again.get("649"), Some(100.0));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ignores_nonsense() {
        let p = Positions::load(None);
        p.save("1", f64::NAN, 3600.0, 1);
        assert_eq!(one(&p, "1"), None);
    }
}
