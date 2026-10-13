use std::sync::{Arc, atomic::AtomicBool};

pub struct AppState {
    pub is_recording: Arc<AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
        }
    }
}
