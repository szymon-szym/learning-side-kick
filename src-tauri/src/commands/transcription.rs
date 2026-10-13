use std::{
    sync::{Arc, Mutex, atomic::Ordering},
    time::Duration,
};

use crate::{models::transcription::Transcription, state::AppState};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::{AppHandle, Emitter, Manager, State};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[tauri::command]
pub async fn start_transcription(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Set recording state active globally
    state.is_recording.store(true, Ordering::SeqCst);
    let is_recording = state.is_recording.clone();

    tokio::spawn(async move {
        let model_path = app_handle
            .path()
            .resolve(
                "models/ggml-small.bin",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| format!("failed to resolve path: {}", e))?;

        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let samples_clone = samples.clone();

        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(dev) => dev,
            None => {
                eprintln!("No default input device found");
                is_recording.store(false, Ordering::SeqCst);
                return;
            }
        };

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: 16000,
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = match device.build_input_stream(
            config,
            move |data: &[f32], _: &_| {
                if let Ok(mut buf) = samples_clone.lock() {
                    buf.extend_from_slice(data);
                }
            },
            move |err| eprintln!("Stream error: {}", err),
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to build input stream: {}", e);
                is_recording.store(false, Ordering::SeqCst);
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("Failed to play stream: {}", e);
            is_recording.store(false, Ordering::SeqCst);
            return;
        }

        println!("🎙 Listening continuously in Tokio...");

        let ctx = match WhisperContext::new_with_params(
            model_path,
            WhisperContextParameters::default(),
        ) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Failed to load Whisper context: {}", e);
                is_recording.store(false, Ordering::SeqCst);
                return;
            }
        };

        let mut interval = tokio::time::interval(Duration::from_millis(1000));

        loop {
            interval.tick().await;

            // Check if recording was stopped while waiting
            if !is_recording.load(Ordering::SeqCst) {
                println!("Recording flag turned false before capture.");
                break;
            }

            let audio_snapshot = {
                let buf = samples.lock().unwrap();
                buf.clone()
            };

            let total_samples = audio_snapshot.len();
            if total_samples < 32000 {
                continue;
            }

            let window_size = 5 * 16000;
            let start_idx = total_samples.saturating_sub(window_size);
            let audio_window = audio_snapshot[start_idx..].to_vec();

            let ctx_clone = ctx.clone();

            let transcript = tokio::task::spawn_blocking(move || {
                let mut state = ctx_clone.create_state().ok()?;

                let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
                params.set_language(Some("pl"));
                params.set_print_special(false);
                params.set_print_progress(false);
                params.set_print_realtime(false);
                params.set_no_context(true);

                state.full(params, &audio_window).ok()?;

                let mut text = String::new();
                let num_segments = state.full_n_segments();

                for i in 0..num_segments {
                    if let Some(segment) = state.get_segment(i) {
                        if let Ok(s) = segment.to_str() {
                            text.push_str(s);
                        }
                    }
                }

                Some(text)
            })
            .await
            .ok()
            .flatten();

            // Check if recording state changed during inference
            let is_final = !is_recording.load(Ordering::SeqCst);

            if let Some(text) = transcript {
                let cleaned = text.trim();
                if !cleaned.is_empty() {
                    let payload = Transcription {
                        text: cleaned.to_string(),
                        is_final,
                    };
                    let _ = app_handle.emit("transcription-stream", payload);
                    println!("chunk emitted (is_final: {})", is_final);
                }
            }

            if is_final {
                println!("End transcription task after sending final chunk");
                break;
            }
        }
    });

    Ok(())
}
