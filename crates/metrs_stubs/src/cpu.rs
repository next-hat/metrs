#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg(feature = "sysinfo")]
use sysinfo::Cpu;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct CpuInfo {
  pub name: String,
  pub vendor_id: String,
  pub brand: String,
  pub frequency: u64,
  pub usage: f32,
}

#[cfg(feature = "sysinfo")]
impl From<&Cpu> for CpuInfo {
  fn from(cpu: &Cpu) -> Self {
    Self {
      name: cpu.name().to_owned(),
      vendor_id: cpu.vendor_id().to_owned(),
      brand: cpu.brand().to_owned(),
      frequency: cpu.frequency(),
      usage: cpu.cpu_usage(),
    }
  }
}
