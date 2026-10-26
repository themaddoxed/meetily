// Platform-specific audio device implementations

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::configure_macos_audio;
