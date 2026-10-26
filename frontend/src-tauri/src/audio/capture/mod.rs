// Audio capture implementations module

pub mod backend;
pub mod macos;

#[cfg(target_os = "macos")]
pub mod core_audio;

#[cfg(target_os = "macos")]
pub use core_audio::{CoreAudioCapture, CoreAudioStream};
