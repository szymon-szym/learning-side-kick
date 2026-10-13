use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tauri::{AppHandle, Emitter, State};

use commands::transcription::start_transcription;
use state::AppState;

mod commands;
mod models;
mod state;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn stop_recording(state: State<'_, AppState>) -> Result<(), String> {
    println!("stop recording");
    state.is_recording.store(false, Ordering::SeqCst);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            start_transcription,
            stop_recording
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
