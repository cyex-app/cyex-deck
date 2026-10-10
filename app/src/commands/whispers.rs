//! IPC for whispers (1.26): the thread list, one conversation, sending, read state and
//! thread options. Always the default account's.

use tauri::AppHandle;

use crate::whispers::view::{WhisperConversation, WhisperEdit, WhisperPage, WhisperState};
use crate::whispers::{self};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn whispers_state(app: AppHandle) -> WhisperState {
    whispers::state(&app)
}

/// The next page of threads; the list comes back through `whispers:changed`.
#[tauri::command]
pub fn whispers_more(app: AppHandle) {
    whispers::load_threads(app, true);
}

#[tauri::command]
pub async fn whispers_open(app: AppHandle, login: String) -> CmdResult<WhisperConversation> {
    whispers::open(&app, &login).await
}

#[tauri::command]
pub async fn whispers_older(app: AppHandle, thread_id: String, cursor: String) -> CmdResult<WhisperPage> {
    whispers::older(&app, &thread_id, &cursor).await
}

#[tauri::command]
pub async fn whispers_send(app: AppHandle, to: String, text: String) -> CmdResult<()> {
    whispers::send(&app, &to, &text).await
}

#[tauri::command]
pub async fn whispers_read(app: AppHandle, thread_id: String) -> CmdResult<()> {
    whispers::mark_read(&app, &thread_id).await
}

#[tauri::command]
pub async fn whispers_edit(app: AppHandle, thread_id: String, edit: WhisperEdit) -> CmdResult<()> {
    whispers::edit(&app, &thread_id, edit).await
}

#[tauri::command]
pub async fn whispers_read_all(app: AppHandle) -> CmdResult<()> {
    whispers::read_all(&app).await
}

#[tauri::command]
pub async fn whispers_block_strangers(app: AppHandle, on: bool) -> CmdResult<()> {
    whispers::block_strangers(&app, on).await
}

#[tauri::command]
pub async fn whispers_block(app: AppHandle, login: String) -> CmdResult<()> {
    whispers::block(&app, &login).await
}

/// The thread a focused view shows (no unread count or alerts for it), or null.
#[tauri::command]
pub fn whispers_viewing(app: AppHandle, thread_id: Option<String>) {
    whispers::viewing(&app, thread_id);
}
