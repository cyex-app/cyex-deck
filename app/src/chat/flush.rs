//! Sends each room's queued events to its views, at most every 50 ms (ARCHITECTURE §2).
//! Sleeps while chat is quiet.

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use super::Chat;

const BATCH: Duration = Duration::from_millis(50);

pub fn spawn(app: AppHandle, wake: Arc<Notify>) {
    tauri::async_runtime::spawn(async move {
        loop {
            wake.notified().await;
            tokio::time::sleep(BATCH).await;
            flush(&app);
        }
    });
}

fn flush(app: &AppHandle) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    for entry in inner.rooms.values_mut() {
        if entry.outbox.is_empty() {
            continue;
        }
        let batch = std::mem::take(&mut entry.outbox);
        // A closed window drops its channel; close() removes the view soon after.
        for (_, view) in &entry.views {
            let _ = view.send(batch.clone());
        }
    }
}
