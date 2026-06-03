//! Windows physical drive path support.

pub const PHYSICAL_DEVICE_SIZE_FALLBACK: u64 = 1024 * 1024 * 1024 * 1024;

/// Check whether a path targets a Windows raw physical drive such as `\\.\PHYSICALDRIVE0`.
pub fn is_physical_drive_path(path: &str) -> bool {
    let normalized = path.replace('/', "\\").to_ascii_uppercase();
    let Some(drive_number) = normalized.strip_prefix(r"\\.\PHYSICALDRIVE") else {
        return false;
    };

    !drive_number.is_empty() && drive_number.chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_physical_drive_path() {
        assert!(is_physical_drive_path(r"\\.\PHYSICALDRIVE0"));
        assert!(is_physical_drive_path(r"\\.\physicaldrive12"));
        assert!(is_physical_drive_path(r"//./PHYSICALDRIVE1"));
        assert!(!is_physical_drive_path(r"\\.\PHYSICALDRIVE"));
        assert!(!is_physical_drive_path(r"\\.\PHYSICALDRIVEX"));
        assert!(!is_physical_drive_path(r"C:\PHYSICALDRIVE0"));
    }
}
