const MACHO_64_LE_MAGIC: [u8; 4] = [0xcf, 0xfa, 0xed, 0xfe];
const CPU_TYPE_X86_64: [u8; 4] = [0x07, 0x00, 0x00, 0x01];
const CPU_TYPE_ARM64: [u8; 4] = [0x0c, 0x00, 0x00, 0x01];

pub fn matches_macos_target(header: &[u8], target: &str) -> bool {
    if header.len() < 8 || header[..4] != MACHO_64_LE_MAGIC {
        return false;
    }

    let expected_cpu_type = if target.starts_with("aarch64-") {
        CPU_TYPE_ARM64
    } else if target.starts_with("x86_64-") {
        CPU_TYPE_X86_64
    } else {
        return false;
    };

    header[4..8] == expected_cpu_type
}

#[cfg(test)]
mod tests {
    use super::matches_macos_target;

    #[test]
    fn recognizes_arm64_macho_for_apple_silicon_target() {
        let header = [0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0x00, 0x00, 0x01];

        assert!(matches_macos_target(&header, "aarch64-apple-darwin"));
        assert!(!matches_macos_target(&header, "x86_64-apple-darwin"));
    }

    #[test]
    fn recognizes_x86_64_macho_for_intel_target() {
        let header = [0xcf, 0xfa, 0xed, 0xfe, 0x07, 0x00, 0x00, 0x01];

        assert!(matches_macos_target(&header, "x86_64-apple-darwin"));
        assert!(!matches_macos_target(&header, "aarch64-apple-darwin"));
    }

    #[test]
    fn rejects_non_macho_data() {
        assert!(!matches_macos_target(b"not-mach-o", "aarch64-apple-darwin"));
    }
}
