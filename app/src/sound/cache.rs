//! V1051-01 Decoded sound files, newest last: at most 8 and 16 MB, so a mention with its own
//! file plays at once after the first time.

use std::collections::VecDeque;
use std::sync::Arc;

const MAX_ITEMS: usize = 8;
/// 16 MB of f32.
const MAX_SAMPLES: usize = 4 * 1024 * 1024;

#[derive(Default)]
pub struct Cache(VecDeque<(String, Arc<[f32]>)>);

impl Cache {
    pub fn get(&mut self, path: &str) -> Option<Arc<[f32]>> {
        let at = self.0.iter().position(|(p, _)| p == path)?;
        let item = self.0.remove(at)?;
        let pcm = item.1.clone();
        self.0.push_back(item);
        Some(pcm)
    }

    /// Keeps `pcm` unless it alone is over the limit; drops the oldest to make room.
    pub fn put(&mut self, path: &str, pcm: Arc<[f32]>) {
        self.0.retain(|(p, _)| p != path);
        if pcm.len() > MAX_SAMPLES {
            return;
        }
        self.0.push_back((path.to_owned(), pcm));
        while self.0.len() > MAX_ITEMS || self.0.iter().map(|(_, s)| s.len()).sum::<usize>() > MAX_SAMPLES {
            self.0.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(n: usize) -> Arc<[f32]> {
        vec![0.0; n].into()
    }

    #[test]
    fn keeps_eight_and_drops_the_oldest() {
        let mut c = Cache::default();
        for i in 0..9 {
            c.put(&i.to_string(), pcm(2));
        }
        assert!(c.get("0").is_none());
        assert!(c.get("1").is_some() && c.get("8").is_some());
    }

    #[test]
    fn a_used_sound_stays() {
        let mut c = Cache::default();
        for i in 0..8 {
            c.put(&i.to_string(), pcm(2));
        }
        c.get("0");
        c.put("new", pcm(2));
        assert!(c.get("0").is_some() && c.get("1").is_none());
    }

    #[test]
    fn stays_under_16_mb() {
        let mut c = Cache::default();
        c.put("a", pcm(MAX_SAMPLES / 2));
        c.put("b", pcm(MAX_SAMPLES / 2 + 1));
        assert!(c.get("a").is_none() && c.get("b").is_some());
        c.put("huge", pcm(MAX_SAMPLES + 1));
        assert!(c.get("huge").is_none());
    }
}
