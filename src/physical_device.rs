//! Windows physical drive path support.

use std::fs::File;
use std::io::{self, Seek, SeekFrom};

pub const PHYSICAL_DEVICE_SIZE_FALLBACK: u64 = 1024 * 1024 * 1024 * 1024;
const DEFAULT_MAX_PHYSICAL_DRIVE_NUMBER: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalDriveInfo {
    pub path: String,
    pub size: Option<u64>,
    pub metadata: PhysicalDriveMetadata,
    pub accessible: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhysicalDriveMetadata {
    pub bytes_per_sector: Option<u32>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub bus_type: Option<String>,
    pub removable: Option<bool>,
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
                let size = get_physical_drive_size(&mut file);
                let metadata = get_physical_drive_metadata(&file);
                drives.push(PhysicalDriveInfo {
                    path,
                    size,
                    metadata,
                    accessible: true,
                    note: None,
                });
            }
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                drives.push(PhysicalDriveInfo {
                    path,
                    size: None,
                    metadata: PhysicalDriveMetadata::default(),
                    accessible: false,
                    note: Some("access denied; run as Administrator".to_string()),
                });
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                drives.push(PhysicalDriveInfo {
                    path,
                    size: None,
                    metadata: PhysicalDriveMetadata::default(),
                    accessible: false,
                    note: Some(err.to_string()),
                });
            }
        }
    }

    drives
}

pub fn get_physical_drive_size(file: &mut File) -> Option<u64> {
    get_windows_physical_drive_size(file).or_else(|| get_stream_size(file))
}

pub fn get_physical_drive_metadata(file: &File) -> PhysicalDriveMetadata {
    get_windows_physical_drive_metadata(file)
}

#[cfg(windows)]
fn get_windows_physical_drive_size(file: &File) -> Option<u64> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle;
    use std::ptr::null_mut;

    const IOCTL_DISK_GET_LENGTH_INFO: u32 = 0x0007_405c;
    const IOCTL_DISK_GET_DRIVE_GEOMETRY_EX: u32 = 0x0007_00a0;

    #[repr(C)]
    struct GetLengthInformation {
        length: i64,
    }

    #[repr(C)]
    struct DiskGeometry {
        cylinders: i64,
        media_type: i32,
        tracks_per_cylinder: u32,
        sectors_per_track: u32,
        bytes_per_sector: u32,
    }

    #[repr(C)]
    struct DiskGeometryEx {
        geometry: DiskGeometry,
        disk_size: i64,
    }

    extern "system" {
        fn DeviceIoControl(
            h_device: *mut c_void,
            dw_io_control_code: u32,
            lp_in_buffer: *mut c_void,
            n_in_buffer_size: u32,
            lp_out_buffer: *mut c_void,
            n_out_buffer_size: u32,
            lp_bytes_returned: *mut u32,
            lp_overlapped: *mut c_void,
        ) -> i32;
    }

    let mut length_info = GetLengthInformation { length: 0 };
    let mut bytes_returned = 0;
    let success = unsafe {
        DeviceIoControl(
            file.as_raw_handle() as *mut c_void,
            IOCTL_DISK_GET_LENGTH_INFO,
            null_mut(),
            0,
            &mut length_info as *mut _ as *mut c_void,
            size_of::<GetLengthInformation>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    let length_size = (success != 0)
        .then_some(length_info.length)
        .and_then(|length| u64::try_from(length).ok())
        .filter(|size| *size > 0);

    if length_size.is_some() {
        return length_size;
    }

    let mut geometry = DiskGeometryEx {
        geometry: DiskGeometry {
            cylinders: 0,
            media_type: 0,
            tracks_per_cylinder: 0,
            sectors_per_track: 0,
            bytes_per_sector: 0,
        },
        disk_size: 0,
    };
    let success = unsafe {
        DeviceIoControl(
            file.as_raw_handle() as *mut c_void,
            IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
            null_mut(),
            0,
            &mut geometry as *mut _ as *mut c_void,
            size_of::<DiskGeometryEx>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    (success != 0)
        .then_some(geometry.disk_size)
        .and_then(|length| u64::try_from(length).ok())
        .filter(|size| *size > 0)
}

#[cfg(not(windows))]
fn get_windows_physical_drive_size(_file: &File) -> Option<u64> {
    None
}

#[cfg(windows)]
fn get_windows_physical_drive_metadata(file: &File) -> PhysicalDriveMetadata {
    let mut metadata = PhysicalDriveMetadata::default();

    if let Some((bytes_per_sector, _disk_size)) = query_windows_drive_geometry(file) {
        metadata.bytes_per_sector = Some(bytes_per_sector);
    }

    if let Some(descriptor) = query_windows_storage_descriptor(file) {
        metadata.vendor = descriptor.vendor;
        metadata.model = descriptor.model;
        metadata.serial = descriptor.serial;
        metadata.bus_type = descriptor.bus_type;
        metadata.removable = descriptor.removable;
    }

    metadata
}

#[cfg(not(windows))]
fn get_windows_physical_drive_metadata(_file: &File) -> PhysicalDriveMetadata {
    PhysicalDriveMetadata::default()
}

#[cfg(windows)]
fn query_windows_drive_geometry(file: &File) -> Option<(u32, u64)> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle;
    use std::ptr::null_mut;

    const IOCTL_DISK_GET_DRIVE_GEOMETRY_EX: u32 = 0x0007_00a0;

    #[repr(C)]
    struct DiskGeometry {
        cylinders: i64,
        media_type: i32,
        tracks_per_cylinder: u32,
        sectors_per_track: u32,
        bytes_per_sector: u32,
    }

    #[repr(C)]
    struct DiskGeometryEx {
        geometry: DiskGeometry,
        disk_size: i64,
    }

    extern "system" {
        fn DeviceIoControl(
            h_device: *mut c_void,
            dw_io_control_code: u32,
            lp_in_buffer: *mut c_void,
            n_in_buffer_size: u32,
            lp_out_buffer: *mut c_void,
            n_out_buffer_size: u32,
            lp_bytes_returned: *mut u32,
            lp_overlapped: *mut c_void,
        ) -> i32;
    }

    let mut geometry = DiskGeometryEx {
        geometry: DiskGeometry {
            cylinders: 0,
            media_type: 0,
            tracks_per_cylinder: 0,
            sectors_per_track: 0,
            bytes_per_sector: 0,
        },
        disk_size: 0,
    };
    let mut bytes_returned = 0;
    let success = unsafe {
        DeviceIoControl(
            file.as_raw_handle() as *mut c_void,
            IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
            null_mut(),
            0,
            &mut geometry as *mut _ as *mut c_void,
            size_of::<DiskGeometryEx>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if success == 0 || geometry.geometry.bytes_per_sector == 0 {
        return None;
    }

    let disk_size = u64::try_from(geometry.disk_size)
        .ok()
        .filter(|size| *size > 0)?;
    Some((geometry.geometry.bytes_per_sector, disk_size))
}

#[cfg(windows)]
struct StorageDescriptorInfo {
    vendor: Option<String>,
    model: Option<String>,
    serial: Option<String>,
    bus_type: Option<String>,
    removable: Option<bool>,
}

#[cfg(windows)]
fn query_windows_storage_descriptor(file: &File) -> Option<StorageDescriptorInfo> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle;
    use std::ptr::null_mut;

    const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002d_1400;
    const STORAGE_DEVICE_PROPERTY: u32 = 0;
    const PROPERTY_STANDARD_QUERY: u32 = 0;
    const STORAGE_DESCRIPTOR_BUFFER_SIZE: usize = 4096;

    #[repr(C)]
    struct StoragePropertyQuery {
        property_id: u32,
        query_type: u32,
        additional_parameters: [u8; 1],
    }

    #[repr(C)]
    struct StorageDeviceDescriptor {
        version: u32,
        size: u32,
        device_type: u8,
        device_type_modifier: u8,
        removable_media: u8,
        command_queueing: u8,
        vendor_id_offset: u32,
        product_id_offset: u32,
        product_revision_offset: u32,
        serial_number_offset: u32,
        bus_type: u32,
        raw_properties_length: u32,
    }

    extern "system" {
        fn DeviceIoControl(
            h_device: *mut c_void,
            dw_io_control_code: u32,
            lp_in_buffer: *mut c_void,
            n_in_buffer_size: u32,
            lp_out_buffer: *mut c_void,
            n_out_buffer_size: u32,
            lp_bytes_returned: *mut u32,
            lp_overlapped: *mut c_void,
        ) -> i32;
    }

    let mut query = StoragePropertyQuery {
        property_id: STORAGE_DEVICE_PROPERTY,
        query_type: PROPERTY_STANDARD_QUERY,
        additional_parameters: [0],
    };
    let mut buffer = vec![0u8; STORAGE_DESCRIPTOR_BUFFER_SIZE];
    let mut bytes_returned = 0;
    let success = unsafe {
        DeviceIoControl(
            file.as_raw_handle() as *mut c_void,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &mut query as *mut _ as *mut c_void,
            size_of::<StoragePropertyQuery>() as u32,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if success == 0 || bytes_returned < size_of::<StorageDeviceDescriptor>() as u32 {
        return None;
    }

    let descriptor = unsafe { &*(buffer.as_ptr() as *const StorageDeviceDescriptor) };
    Some(StorageDescriptorInfo {
        vendor: read_descriptor_string(&buffer, descriptor.vendor_id_offset),
        model: read_descriptor_string(&buffer, descriptor.product_id_offset),
        serial: read_descriptor_string(&buffer, descriptor.serial_number_offset),
        bus_type: map_storage_bus_type(descriptor.bus_type),
        removable: Some(descriptor.removable_media != 0),
    })
}

#[cfg(windows)]
fn read_descriptor_string(buffer: &[u8], offset: u32) -> Option<String> {
    let offset = usize::try_from(offset).ok()?;
    if offset == 0 || offset >= buffer.len() {
        return None;
    }

    let value = buffer[offset..]
        .iter()
        .take_while(|byte| **byte != 0)
        .copied()
        .collect::<Vec<_>>();

    let value = String::from_utf8_lossy(&value).trim().to_string();
    (!value.is_empty()).then_some(value)
}

#[cfg(windows)]
fn map_storage_bus_type(bus_type: u32) -> Option<String> {
    let name = match bus_type {
        0x01 => "SCSI",
        0x02 => "ATAPI",
        0x03 => "ATA",
        0x04 => "IEEE1394",
        0x05 => "SSA",
        0x06 => "Fibre",
        0x07 => "USB",
        0x08 => "RAID",
        0x09 => "iSCSI",
        0x0a => "SAS",
        0x0b => "SATA",
        0x0c => "SD",
        0x0d => "MMC",
        0x0e => "Virtual",
        0x0f => "FileBackedVirtual",
        0x10 => "Spaces",
        0x11 => "NVMe",
        0x12 => "SCM",
        0x13 => "UFS",
        _ => return None,
    };

    Some(name.to_string())
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

pub fn format_sector_size(bytes_per_sector: Option<u32>) -> String {
    bytes_per_sector
        .filter(|size| *size > 0)
        .map(|size| format!("{size} B"))
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn format_drive_details(metadata: &PhysicalDriveMetadata) -> String {
    let mut details = Vec::new();

    if let Some(bus_type) = metadata.bus_type.as_deref() {
        details.push(bus_type.to_string());
    }

    let identity = [metadata.vendor.as_deref(), metadata.model.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    if !identity.is_empty() {
        details.push(identity);
    }

    if let Some(serial) = metadata.serial.as_deref() {
        details.push(format!("serial={serial}"));
    }

    if metadata.removable == Some(true) {
        details.push("removable".to_string());
    }

    if details.is_empty() {
        "-".to_string()
    } else {
        details.join("; ")
    }
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

    #[test]
    fn test_format_sector_size() {
        assert_eq!(format_sector_size(None), "unknown");
        assert_eq!(format_sector_size(Some(0)), "unknown");
        assert_eq!(format_sector_size(Some(512)), "512 B");
    }

    #[test]
    fn test_format_drive_details() {
        assert_eq!(format_drive_details(&PhysicalDriveMetadata::default()), "-");

        let metadata = PhysicalDriveMetadata {
            bytes_per_sector: Some(512),
            vendor: Some("Acme".to_string()),
            model: Some("FastDisk".to_string()),
            serial: Some("ABC123".to_string()),
            bus_type: Some("USB".to_string()),
            removable: Some(true),
        };

        assert_eq!(
            format_drive_details(&metadata),
            "USB; Acme FastDisk; serial=ABC123; removable"
        );
    }
}
