use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::error;

use super::configuration::{AudioDevice, DeviceType};
use super::platform;

/// List all available audio devices on the system
pub async fn list_audio_devices() -> Result<Vec<AudioDevice>> {
    let host = cpal::default_host();

    // Platform-specific device enumeration
    let mut devices = {
        #[cfg(target_os = "windows")]
        {
            platform::configure_windows_audio(&host)?
        }

        #[cfg(target_os = "linux")]
        {
            platform::configure_linux_audio(&host)?
        }

        #[cfg(target_os = "macos")]
        {
            platform::configure_macos_audio(&host)?
        }
    };

    // Add any additional devices from the default host
    if let Ok(other_devices) = host.devices() {
        for device in other_devices {
            if let Ok(name) = device.name() {
                if !devices.iter().any(|d| d.name == name) {
                    devices.push(AudioDevice::new(name, DeviceType::Output));
                }
            }
        }
    }

    Ok(devices)
}

/// Trigger audio permission request on platforms that require it
/// Returns Ok(true) if permission is granted, Ok(false) if denied, Err if something went wrong
pub fn trigger_audio_permission() -> Result<bool> {
    use log::info;

    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            info!("[trigger_audio_permission] No default input device found - permission likely denied");
            return Ok(false);
        }
    };

    let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            info!("[trigger_audio_permission] Failed to get input config: {} - permission likely denied", e);
            return Ok(false);
        }
    };

    // Build and start an input stream to trigger the permission request
    let stream = match device.build_input_stream(
        &config.into(),
        |_data: &[f32], _: &cpal::InputCallbackInfo| {
            // Do nothing, we just want to trigger the permission request
        },
        |err| error!("Error in audio stream: {}", err),
        None,
    ) {
        Ok(s) => s,
        Err(e) => {
            info!("[trigger_audio_permission] Failed to build input stream: {} - permission likely denied", e);
            return Ok(false);
        }
    };

    // Start the stream to actually trigger the permission dialog
    if let Err(e) = stream.play() {
        info!(
            "[trigger_audio_permission] Failed to play stream: {} - permission likely denied",
            e
        );
        return Ok(false);
    }

    // Sleep briefly to allow the permission dialog to appear and for stream to actually work
    std::thread::sleep(std::time::Duration::from_millis(500));

    // If we got here, permission was granted
    info!("[trigger_audio_permission] Stream played successfully - permission granted");

    // Stop the stream
    drop(stream);

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate_system_audio_devices(devices: &[AudioDevice]) -> Result<(), String> {
        let output_devices: Vec<_> = devices
            .iter()
            .filter(|device| matches!(&device.device_type, &DeviceType::Output))
            .collect();

        if output_devices.is_empty() {
            return Err("system audio enumeration returned no output devices".to_string());
        }
        if output_devices
            .iter()
            .any(|device| device.name.trim().is_empty())
        {
            return Err(
                "system audio enumeration returned an output device with an empty name".to_string(),
            );
        }

        Ok(())
    }

    #[test]
    fn system_audio_device_validation_rejects_input_only_list() {
        let devices = vec![AudioDevice::new(
            "Built-in Microphone".to_string(),
            DeviceType::Input,
        )];

        let error = validate_system_audio_devices(&devices)
            .expect_err("an input-only list must not satisfy system audio enumeration");
        assert!(
            error.contains("output"),
            "unexpected validation error: {error}"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "requires real macOS audio hardware"]
    fn macos_system_audio_devices_have_valid_names() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create a Tokio runtime for audio device enumeration");
        let devices = runtime
            .block_on(list_audio_devices())
            .expect("macOS audio device enumeration failed");

        if let Err(error) = validate_system_audio_devices(&devices) {
            panic!("macOS audio device enumeration failed validation: {error}");
        }
    }
}
