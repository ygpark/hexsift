//! Windows physical drive path support.

use std::fs::File;
use std::io::{self, Seek, SeekFrom};

pub const PHYSICAL_DEVICE_SIZE_FALLBACK: u64 = 1024 * 1024 * 1024 * 1024;
const DEFAULT_MAX_PHYSICAL_DRIVE_NUMBER: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalDriveInfo {
    pub path: String,
    pub size: Option<u64>,
    pub accessible: bool,
    pub note: Option<String>,
}

/// Check whether a path targets a Windows raw physical drive such as `\\.\PHYSICALDRIVE0`.
pub fn is_physical_drive_path(path: &str) -> bool {
    let normalized = path.replace('/', "\\").to_ascii_uppercase();
    let Some(drive_number) = normalized.strip_prefix(r"\\.\PHYSICALDRIVE") else {
        return false;
    };

    !drive_number.is_empty() && drive_number.chars().all(|ch| ch.is_ascii_digit())
}

pub fn list_physical_drives() -> Vec<PhysicalDriveInfo> {
    list_physical_drives_up_to(DEFAULT_MAX_PHYSICAL_DRIVE_NUMBER)
}

fn list_physical_drives_up_to(max_drive_number: u32) -> Vec<PhysicalDriveInfo> {
    let mut drives = Vec::new();

    for index in 0..max_drive_number {
        let path = format!(r"\\.\PHYSICALDRIVE{}", index);
        match File::open(&path) {
            Ok(mut file) => {
                let size = get_stream_size(&mut file);
                drives.push(PhysicalDriveInfo {
                    path,
                    size,
                    accessible: true,
                    note: None,
                });
            }
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                drives.push(PhysicalDriveInfo {
                    path,
                    size: None,
                    accessible: false,
                    note: Some("access denied; run as Administrator".to_string()),
                });
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                drives.push(PhysicalDriveInfo {
                    path,
                    size: None,
                    accessible: false,
                    note: Some(err.to_string()),
                });
            }
        }
    }

    drives
}

fn get_stream_size(file: &mut File) -> Option<u64> {
    let current_pos = file.stream_position().ok();
    let size = file
        .seek(SeekFrom::End(0))
        .ok()
        .filter(|size| *size > 0)
        .or_else(|| {
            file.metadata()
                .ok()
                .map(|metadata| metadata.len())
                .filter(|size| *size > 0)
        });

    if let Some(pos) = current_pos {
        let _ = file.seek(SeekFrom::Start(pos));
    }

    size
}

pub fn format_drive_size(size: Option<u64>) -> String {
    let Some(size) = size else {
        return "unknown".to_string();
    };

    const UNITS: &[(&str, u64)] = &[
        ("TB", 1024_u64.pow(4)),
        ("GB", 1024_u64.pow(3)),
        ("MB", 1024_u64.pow(2)),
        ("KB", 1024),
        ("B", 1),
    ];

    for &(unit, divisor) in UNITS {
        if size >= divisor {
            return format!("{:.2} {}", size as f64 / divisor as f64, unit);
        }
    }

    "0 B".to_string()
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

    #[test]
    fn test_format_drive_size() {
        assert_eq!(format_drive_size(None), "unknown");
        assert_eq!(format_drive_size(Some(0)), "0 B");
        assert_eq!(format_drive_size(Some(512)), "512.00 B");
        assert_eq!(format_drive_size(Some(1024)), "1.00 KB");
        assert_eq!(format_drive_size(Some(1024 * 1024 * 1024)), "1.00 GB");
    }
}
