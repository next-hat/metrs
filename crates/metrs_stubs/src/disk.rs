#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg(feature = "sysinfo")]
use sysinfo::{Disk, DiskKind, DiskExt};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct DiskInfo {
  pub kind: DiskInfoKind,
  pub device_name: String,
  pub file_system: Vec<u8>,
  pub mount_point: String,
  pub total_space: u64,
  pub available_space: u64,
  pub is_removable: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
#[allow(clippy::upper_case_acronyms)]
pub enum DiskInfoKind {
  /// HDD type.
  HDD,
  /// SSD type.
  SSD,
  /// Unknown type.
  Unknown(isize),
}

#[cfg(feature = "sysinfo")]
impl From<DiskKind> for DiskInfoKind {
  fn from(disk_type: DiskKind) -> Self {
    match disk_type {
      DiskKind::HDD => Self::HDD,
      DiskKind::SSD => Self::SSD,
      DiskKind::Unknown(val) => Self::Unknown(val),
    }
  }
}

#[cfg(feature = "sysinfo")]
impl From<&Disk> for DiskInfo {
  fn from(disk: &Disk) -> Self {
    Self {
      kind: disk.kind().to_owned().into(),
      device_name: disk.name().to_str().unwrap_or_default().to_owned(),
      file_system: disk.file_system().to_vec(),
      mount_point: disk.mount_point().display().to_string(),
      total_space: disk.total_space(),
      available_space: disk.available_space(),
      is_removable: disk.is_removable(),
    }
  }
}
