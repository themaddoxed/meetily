use crate::audio::{GpuType, PerformanceTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperCompiledBackend {
    Metal,
    Cpu,
}

fn compiled_backend_for_arch(arch: &str) -> WhisperCompiledBackend {
    if arch == "aarch64" {
        WhisperCompiledBackend::Metal
    } else {
        WhisperCompiledBackend::Cpu
    }
}

impl WhisperCompiledBackend {
    pub fn current() -> Self {
        compiled_backend_for_arch(std::env::consts::ARCH)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Metal => "Metal",
            Self::Cpu => "Cpu",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhisperContextAcceleration {
    pub compiled_backend: WhisperCompiledBackend,
    pub runtime_detected_gpu: GpuType,
    pub use_gpu: bool,
    pub flash_attn: bool,
    pub gpu_device: i32,
}

impl WhisperContextAcceleration {
    pub fn status_label(self) -> &'static str {
        match (self.compiled_backend, self.flash_attn) {
            (WhisperCompiledBackend::Metal, true) => "Metal GPU with Flash Attention (Ultra-Fast)",
            (WhisperCompiledBackend::Metal, false) => "Metal GPU acceleration",
            (WhisperCompiledBackend::Cpu, _) => "CPU processing only",
        }
    }
}

pub fn whisper_context_acceleration_for(
    compiled_backend: WhisperCompiledBackend,
    runtime_detected_gpu: GpuType,
    performance_tier: PerformanceTier,
) -> WhisperContextAcceleration {
    let use_gpu = !matches!(compiled_backend, WhisperCompiledBackend::Cpu);
    let fast_tier = matches!(
        performance_tier,
        PerformanceTier::High | PerformanceTier::Ultra
    );
    let flash_attn = matches!(compiled_backend, WhisperCompiledBackend::Metal) && fast_tier;

    WhisperContextAcceleration {
        compiled_backend,
        runtime_detected_gpu,
        use_gpu,
        flash_attn: use_gpu && flash_attn,
        gpu_device: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architecture_selection_routes_intel_to_cpu_and_arm64_to_metal() {
        assert_eq!(
            compiled_backend_for_arch("x86_64"),
            WhisperCompiledBackend::Cpu
        );
        assert_eq!(
            compiled_backend_for_arch("aarch64"),
            WhisperCompiledBackend::Metal
        );
    }

    #[test]
    fn acceleration_metal_backend_uses_gpu_without_runtime_probe() {
        let params = whisper_context_acceleration_for(
            WhisperCompiledBackend::Metal,
            GpuType::None,
            PerformanceTier::High,
        );

        assert_eq!(params.compiled_backend, WhisperCompiledBackend::Metal);
        assert_eq!(params.runtime_detected_gpu, GpuType::None);
        assert!(params.use_gpu);
        assert!(params.flash_attn);
    }

    #[test]
    fn acceleration_cpu_backend_disables_gpu_and_flash_attention() {
        for runtime_gpu in [GpuType::None, GpuType::Metal] {
            let params = whisper_context_acceleration_for(
                WhisperCompiledBackend::Cpu,
                runtime_gpu,
                PerformanceTier::Ultra,
            );

            assert!(!params.use_gpu);
            assert!(!params.flash_attn);
        }
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    #[test]
    fn current_backend_uses_cpu_on_intel_macos() {
        assert_eq!(
            WhisperCompiledBackend::current(),
            WhisperCompiledBackend::Cpu
        );
    }
}
