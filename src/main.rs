use std::ffi::OsString;
use std::path::{PathBuf, Path};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use ntex::http::StatusCode;
use ntex::rt::{self, JoinError};
use ntex::time::interval;
use ntex::util::Bytes;
use futures::Stream;
use ntex::web::error::BlockingError;
use serde::Serialize;
use sysinfo::{SystemExt, Disk, DiskType, DiskExt};
use tokio::sync::mpsc::{channel, Receiver, Sender};
/// Metrs is a metrics server
/// It is a simple server that can be use to receive metrics from an host
/// You can listen on the metrics server to get the metrics in real time.
/// TODO: Add SSL/TLS support for the server
///
use ntex::web::{middleware, Error, self};
use ntex::web::{App, HttpServer};

use clap::Parser;
use serde_json::json;

#[derive(Debug)]
enum MetrsError {
  Error(String),
}

#[derive(Debug, Parser)]
struct Cli {
  /// Hosts to listen on
  hosts: Option<Vec<String>>,
}

impl std::fmt::Display for MetrsError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      MetrsError::Error(err) => write!(f, "{err}"),
    }
  }
}

// Wrap Receiver in our own type, with correct error type
pub struct Client(Receiver<Bytes>);

impl Stream for Client {
  type Item = Result<Bytes, Error>;

  fn poll_next(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    match Pin::new(&mut self.0).poll_recv(cx) {
      Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
      Poll::Ready(None) => Poll::Ready(None),
      Poll::Pending => Poll::Pending,
    }
  }
}

#[derive(Clone)]
struct EventEmitter {
  inner: Arc<Mutex<EventEmitterInner>>,
}

#[derive(Clone)]
struct EventEmitterInner {
  clients: Vec<Sender<Bytes>>,
}

#[derive(Debug)]
struct HttpError {
  status: StatusCode,
  msg: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct MemoryInfo {
  total: u64,
  free: u64,
  used: u64,
  swap_total: u64,
  swap_free: u64,
  swap_used: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct CpuInfo {
  cpu_usage: f32,
  cpu_cores: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[serde(tag = "Type", content = "Data")]
enum Event {
  MemoryInfo(MemoryInfo),
  DiskInfo(Vec<DiskInfo>),
}

// enum DiskTypeInfo {

// }

#[derive(Clone, Debug, Serialize)]
struct DiskInfo {
  r#type: DiskInfoType,
  device_name: String,
  file_system: Vec<u8>,
  mount_point: String,
  total_space: u64,
  available_space: u64,
  is_removable: bool,
}

#[derive(Clone, Debug, Serialize)]
enum DiskInfoType {
  /// HDD type.
  HDD,
  /// SSD type.
  SSD,
  /// Unknown type.
  Unknown(isize),
}

impl From<DiskType> for DiskInfoType {
  fn from(disk_type: DiskType) -> Self {
    match disk_type {
      DiskType::HDD => Self::HDD,
      DiskType::SSD => Self::SSD,
      DiskType::Unknown(val) => Self::Unknown(val),
    }
  }
}

impl From<&Disk> for DiskInfo {
  fn from(disk: &Disk) -> Self {
    Self {
      r#type: disk.type_().into(),
      device_name: disk.name().to_str().unwrap_or_default().to_owned(),
      file_system: disk.file_system().to_vec(),
      mount_point: disk.mount_point().display().to_string(),
      total_space: disk.total_space(),
      available_space: disk.available_space(),
      is_removable: disk.is_removable(),
    }
  }
}

impl TryFrom<Event> for Bytes {
  type Error = MetrsError;

  fn try_from(value: Event) -> Result<Self, Self::Error> {
    serde_json::to_string(&value)
      .map_err(|err| {
        MetrsError::Error(format!("Unable to serialize memory info: {err}"))
      })
      .map(|res| Bytes::from(res + "\n"))
  }
}

impl std::fmt::Display for HttpError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{status}]: {msg}", status = self.status, msg = self.msg)
  }
}

impl ntex::web::WebResponseError for HttpError {
  // Builds the actual response to send back when an error occurs
  fn error_response(&self, _: &web::HttpRequest) -> web::HttpResponse {
    log::error!("{self}");
    let err_json = json!({ "msg": self.msg });
    web::HttpResponse::build(self.status).json(&err_json)
  }
}

impl EventEmitter {
  fn new() -> Self {
    let this = Self {
      inner: Arc::new(Mutex::new(EventEmitterInner { clients: vec![] })),
    };
    this.clone().spawn_check_connection();
    this
  }

  /// Spawn a task that will check if clients are still connected
  fn spawn_check_connection(mut self) {
    rt::spawn(async move {
      loop {
        let task = interval(Duration::from_secs(10));
        task.tick().await;
        if let Err(err) = self.check_connection() {
          log::error!("{err}");
        }
      }
    });
  }

  async fn subscribe(&self) -> Result<Client, HttpError> {
    let this = self.clone();
    let (tx, rx) = channel(100);
    web::block(move || {
      this
        .inner
        .lock()
        .map_err(|err| HttpError {
          status: StatusCode::INTERNAL_SERVER_ERROR,
          msg: format!("Unable to lock event emitter mutex: {err}"),
        })?
        .clients
        .push(tx);
      Ok::<(), HttpError>(())
    })
    .await
    .map_err(|err| match err {
      BlockingError::Error(err) => err,
      BlockingError::Canceled => HttpError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: "Unable to subscribe to metrics server furture got cancelled"
          .into(),
      },
    })?;
    Ok(Client(rx))
  }

  /// Check if clients are still connected
  fn check_connection(&mut self) -> Result<(), HttpError> {
    log::debug!("Checking alive connection...");
    let mut alive_clients = Vec::new();
    let clients = self
      .inner
      .lock()
      .map_err(|err| HttpError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: format!("Unable to lock event emitter mutex: {err}"),
      })?
      .clients
      .clone();
    for client in clients {
      let result = client.clone().try_send(Bytes::from(""));
      if let Ok(()) = result {
        alive_clients.push(client.clone());
      }
    }
    log::debug!("Alive clients: {alive_clients:?}");
    self
      .inner
      .lock()
      .map_err(|err| HttpError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: format!("Unable to lock event emitter mutex: {err}"),
      })?
      .clients = alive_clients;
    Ok(())
  }

  async fn emit(&self, ev: Event) -> Result<(), HttpError> {
    let this = self.clone();
    rt::spawn(async move {
      let clients = this
        .inner
        .lock()
        .map_err(|err| HttpError {
          status: StatusCode::INTERNAL_SERVER_ERROR,
          msg: format!("Unable to lock event emitter mutex: {err}"),
        })?
        .clients
        .clone();
      for client in clients {
        let msg = Bytes::try_from(ev.clone()).map_err(|err| HttpError {
          status: StatusCode::INTERNAL_SERVER_ERROR,
          msg: format!("Unable to serialize event: {err}"),
        })?;
        client.send(msg.clone()).await.map_err(|err| HttpError {
          status: StatusCode::INTERNAL_SERVER_ERROR,
          msg: format!("Unable to send message to client: {err}"),
        })?;
      }
      Ok::<(), HttpError>(())
    })
    .await
    .map_err(|err| HttpError {
      status: StatusCode::INTERNAL_SERVER_ERROR,
      msg: format!("Unable to spawn task to emit message: {err}"),
    })??;
    Ok(())
  }
}

#[ntex::web::get("/subscribe")]
async fn subscribe(
  req: web::HttpRequest,
  event_emitter: web::types::State<EventEmitter>,
) -> Result<web::HttpResponse, HttpError> {
  let client = event_emitter.subscribe().await?;
  Ok(
    web::HttpResponse::Ok()
      .content_type("text/event-stream")
      .streaming(client),
  )
}

fn conf_srv(config: &mut web::ServiceConfig) {
  config.service(subscribe);
}

/// Todo add ssl support
fn gen_srv(
  event_emitter: EventEmitter,
) -> Result<ntex::server::Server, MetrsError> {
  let srv = HttpServer::new(move || {
    App::new()
      .state(event_emitter.clone())
      .wrap(middleware::Logger::default())
      .configure(conf_srv)
  })
  .bind("0.0.0.0:8080")
  .map_err(|err| MetrsError::Error(format!("Unable to bind server: {err}")))?;

  Ok(srv.run())
}

fn send_memory_usage(event_emitter: EventEmitter) {
  rt::spawn(async move {
    let mut sys = sysinfo::System::default();
    sys.refresh_memory();
    let memory = MemoryInfo {
      total: sys.total_memory(),
      used: sys.used_memory(),
      free: sys.free_memory(),
      swap_total: sys.total_swap(),
      swap_used: sys.used_swap(),
      swap_free: sys.free_swap(),
    };
    if let Err(err) = event_emitter.emit(Event::MemoryInfo(memory)).await {
      log::error!("{err}");
    }
  });
}

fn send_disk_info(event_emitter: EventEmitter) {
  rt::spawn(async move {
    let mut sys = sysinfo::System::default();
    sys.refresh_disks_list();
    let disks = sys.disks().iter().map(DiskInfo::from).collect::<Vec<_>>();
    if let Err(err) = event_emitter.emit(Event::DiskInfo(disks)).await {
      log::error!("{err}");
    }
  });
}

// fn send_cpu_usage(event_emitter: EventEmitter) {
//   rt::spawn(async move {
//     let mut sys = sysinfo::System::default();
//     sys.refresh_cpu();

//     let cpus = sys.
//     println!("{cpus:?}");

//     // let cpu = CpuInfo {
//     //   cpu_usage: sys.cpus()[0]
//     //   cpu_cores: sys.get_processors().len(),
//     // };
//     // let ev = Event::CpuInfo(cpu);
//     // match Bytes::try_from(ev) {
//     //   Ok(msg) => {
//     //     if let Err(err) = event_emitter.emit(msg).await {
//     //       log::error!("{err}");
//     //     }
//     //   }
//     //   Err(err) => log::error!("{err}"),
//     // }
//   });
// }

fn spawn_background_loop(event_emitter: EventEmitter) {
  rt::Arbiter::new().exec_fn(move || {
    rt::spawn(async move {
      let interval = interval(Duration::from_secs(2));
      // First we update all information of our `System` struct.
      loop {
        send_disk_info(event_emitter.clone());
        send_memory_usage(event_emitter.clone());
        interval.tick().await;
      }
    });
  });
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
  // Build env logger
  if std::env::var("LOG_LEVEL").is_err() {
    std::env::set_var("LOG_LEVEL", "metrs=info,warn,error,metrs=debug");
  }
  env_logger::Builder::new()
    .parse_env("LOG_LEVEL")
    .format_target(false)
    .init();

  let event_emitter = EventEmitter::new();

  spawn_background_loop(event_emitter.clone());

  log::info!("Starting server");
  let srv = match gen_srv(event_emitter) {
    Err(err) => {
      println!("{err}");
      std::process::exit(1);
    }
    Ok(srv) => srv,
  };
  srv.await?;
  log::info!("Server stopped");
  Ok(())
}
