//! V107-R2 Test builds only: with `CYEX_MEM_PROBE=<file>`, one JSON line every 30 s with the
//! live Rust heap (`sys::heap`), the tokio task count and the main stores' sizes. Soak runs
//! (`tools/perf/heap`) read it next to the process's private bytes.

use std::io::Write;
use std::time::Duration;

use tauri::AppHandle;

const VAR: &str = "CYEX_MEM_PROBE";
const EVERY: Duration = Duration::from_secs(30);

pub fn init(app: &AppHandle) {
    let Some(path) = std::env::var_os(VAR) else { return };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let t0 = std::time::Instant::now();
        loop {
            let line = sample(&app, t0.elapsed().as_secs());
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                let _ = writeln!(f, "{line}");
            }
            tokio::time::sleep(EVERY).await;
        }
    });
}

fn sample(app: &AppHandle, secs: u64) -> serde_json::Value {
    let (live, peak, blocks, total) = crate::sys::heap::stats();
    let tasks = tokio::runtime::Handle::try_current()
        .map(|h| h.metrics().num_alive_tasks())
        .unwrap_or(0);
    serde_json::json!({
        "s": secs, "heapMB": mb(live), "peakMB": mb(peak), "blocks": blocks, "allocs": total,
        "tasks": tasks, "chat": crate::chat::probe(app), "emotes": crate::emotes::probe(app),
    })
}

fn mb(b: usize) -> f64 {
    (b as f64 / 1_048_576.0 * 100.0).round() / 100.0
}
