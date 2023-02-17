use std::pin::Pin;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use ntex::rt;
use ntex::web;
use ntex::util::Bytes;
use ntex::time::interval;
use ntex::http::StatusCode;
use ntex::web::error::{Error, BlockingError};
use futures::Stream;
use tokio::sync::mpsc::{Sender, Receiver, channel};
use metrs_stubs::*;

use crate::error::{HttpError, MetrsError};

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
#[serde(tag = "Type", content = "Data")]
pub enum Event {
  Memory(MemoryInfo),
  Cpu(Vec<CpuInfo>),
  Disk(Vec<DiskInfo>),
  Network(Vec<NetworkInfo>),
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
pub struct EventEmitter {
  inner: Arc<Mutex<EventEmitterInner>>,
}

#[derive(Clone)]
struct EventEmitterInner {
  clients: Vec<Sender<Bytes>>,
}

impl EventEmitter {
  pub fn new() -> Self {
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
    log::debug!("Alive clients: {}", alive_clients.len());
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

  pub async fn subscribe(&self) -> Result<Client, HttpError> {
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

  pub async fn emit(&self, ev: Event) -> Result<(), HttpError> {
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
        let _ = client.send(msg.clone()).await;
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
