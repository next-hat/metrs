use std::time::Duration;

use ntex::rt;
use ntex::time::interval;
use sysinfo::{System, SystemExt, NetworksExt, NetworkExt};

use metrs_stubs::{CpuInfo, DiskInfo, MemoryInfo, NetworkInfo};

use crate::event_emitter::{Event, EventEmitter};

pub fn sync_cpu_info(event_emitter: EventEmitter) {
  rt::spawn(async move {
    let mut sys = System::new();
    let interval = interval(Duration::from_secs(2));

    loop {
      sys.refresh_cpu();
      let cpus = sys.cpus().iter().map(CpuInfo::from).collect::<Vec<_>>();

      if let Err(err) = event_emitter.emit(Event::Cpu(cpus)).await {
        log::error!("{err}");
      }
      // Sleeping for 500 ms to let time for the system to run for long
      // enough to have useful information.
      interval.tick().await;
    }
  });
}

pub fn sync_disk_info(event_emitter: EventEmitter) {
  rt::spawn(async move {
    let mut sys = System::new();
    let interval = interval(Duration::from_secs(2));
    loop {
      sys.refresh_disks_list();
      let disks = sys.disks().iter().map(DiskInfo::from).collect::<Vec<_>>();
      if let Err(err) = event_emitter.emit(Event::Disk(disks)).await {
        log::error!("{err}");
      }
      interval.tick().await;
    }
  });
}

pub fn sync_memory_info(event_emitter: EventEmitter) {
  rt::spawn(async move {
    let mut sys = System::new();
    let interval = interval(Duration::from_secs(2));
    loop {
      sys.refresh_memory();
      let memory = MemoryInfo {
        total: sys.total_memory(),
        used: sys.used_memory(),
        free: sys.free_memory(),
        swap_total: sys.total_swap(),
        swap_used: sys.used_swap(),
        swap_free: sys.free_swap(),
      };
      if let Err(err) = event_emitter.emit(Event::Memory(memory)).await {
        log::error!("{err}");
      }
      interval.tick().await;
    }
  });
}

pub fn sync_network_info(event_emitter: EventEmitter) {
  rt::spawn(async move {
    let mut sys = System::new();
    let interval = interval(Duration::from_secs(2));
    loop {
      sys.refresh_networks_list();
      let networks = sys
        .networks()
        .iter()
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
      if let Err(err) = event_emitter.emit(Event::Network(networks)).await {
        log::error!("{err}");
      }
      interval.tick().await;
    }
  });
}

pub fn spawn_metrics(event_emitter: EventEmitter) {
  rt::Arbiter::new().exec_fn(move || {
    sync_cpu_info(event_emitter.clone());
    sync_network_info(event_emitter.clone());
    sync_disk_info(event_emitter.clone());
    sync_memory_info(event_emitter.clone());
  });
}
