#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct NetworkInfo {
  pub name: String,
  pub mac_addr: String,
  pub received: u64,
  pub transmitted: u64,
  pub packets_received: u64,
  pub packets_transmitted: u64,
  pub error_received: u64,
  pub error_transmitted: u64,
}
