use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Transcription {
    pub text: String,
    pub is_final: bool,
}
