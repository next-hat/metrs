#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct MemoryInfo {
  pub total: u64,
  pub free: u64,
  pub used: u64,
  pub swap_total: u64,
  pub swap_free: u64,
  pub swap_used: u64,
}
