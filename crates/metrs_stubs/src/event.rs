use super::{CpuInfo, DiskInfo, MemoryInfo, NetworkInfo};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MetrsdEvent {
  pub memory: MemoryInfo,
  pub cpus: Vec<CpuInfo>,
  pub disks: Vec<DiskInfo>,
  pub networks: Vec<NetworkInfo>,
}

#[cfg(feature = "bytes")]
impl TryFrom<MetrsdEvent> for ntex_bytes::Bytes {
  type Error = serde_json::error::Error;

  fn try_from(value: MetrsdEvent) -> Result<Self, Self::Error> {
    serde_json::to_string(&value).map(|res| ntex_bytes::Bytes::from(res + "\n"))
  }
}
