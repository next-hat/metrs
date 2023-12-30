use std::time::Duration;

use ntex::{rt, time::interval};
use sysinfo::{System, Networks, Disks};

use metrs_stubs::{CpuInfo, DiskInfo, MemoryInfo, NetworkInfo, MetrsdEvent};

use crate::event_emitter::EventEmitter;

async fn sync_metrics(event_emitter: &EventEmitter, tick_interval: u64) {
  let mut sys = System::new();
  let interval = interval(Duration::from_secs(tick_interval));
  loop {
    sys.refresh_all();
    let networks = Networks::new_with_refreshed_list()
      .into_iter()
      .map(|(name, net)| NetworkInfo {
        name: name.clone(),
        mac_addr: net.mac_address().to_string(),
        received: net.received(),
        transmitted: net.transmitted(),
        packets_received: net.packets_received(),
        packets_transmitted: net.packets_transmitted(),
        error_received: net.errors_on_received(),
        error_transmitted: net.errors_on_transmitted(),
      })
      .collect::<Vec<NetworkInfo>>();
    let memory = MemoryInfo {
      total: sys.total_memory(),
      used: sys.used_memory(),
      free: sys.free_memory(),
      swap_total: sys.total_swap(),
      swap_used: sys.used_swap(),
      swap_free: sys.free_swap(),
    };
    let disks = Disks::new_with_refreshed_list()
      .iter()
      .map(DiskInfo::from)
      .collect::<Vec<_>>();
    let cpus = sys.cpus().iter().map(CpuInfo::from).collect::<Vec<_>>();
    let event = MetrsdEvent {
      cpus,
      disks,
      networks,
      memory,
    };
    if let Err(err) = event_emitter.emit(event).await {
      log::error!("{err}");
    }
    // Sleeping for minimum 500 ms to let time for the system to run for long
    // enough to have useful information.
    interval.tick().await;
  }
}

pub fn spawn_metrics(event_emitter: EventEmitter, tick_interval: u64) {
  rt::Arbiter::new().exec_fn(move || {
    rt::spawn(async move {
      sync_metrics(&event_emitter, tick_interval).await;
    });
  });
}
