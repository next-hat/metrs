use ntex::web;
use ntex::http::StatusCode;

use crate::event_emitter::EventEmitter;
use crate::error::{MetrsError, HttpError};

#[ntex::web::get("/subscribe")]
async fn subscribe(
  event_emitter: web::types::State<EventEmitter>,
) -> Result<web::HttpResponse, HttpError> {
  let client = event_emitter.subscribe().await?;
  Ok(
    web::HttpResponse::Ok()
      .content_type("text/event-stream")
      .streaming(client),
  )
}

async fn unhandled_route() -> Result<web::HttpResponse, HttpError> {
  Err(HttpError {
    status: StatusCode::NOT_FOUND,
    msg: "Unhandled route".into(),
  })
}

pub fn gen_srv<T>(
  hosts: &[T],
  event_emitter: EventEmitter,
) -> Result<ntex::server::Server, MetrsError>
where
  T: Into<String> + Clone,
{
  let mut srv = web::HttpServer::new(move || {
    web::App::new()
      .state(event_emitter.clone())
      .service(subscribe)
      .default_service(web::route().to(unhandled_route))
  });

  for host in hosts {
    let host = host.to_owned().into();
    match &host {
      host if host.starts_with("unix://") => {
        let path = host.trim_start_matches("unix://");
        srv = srv.bind_uds(path).map_err(|err| {
          MetrsError::Error(format!("Unable to bind server: {err}"))
        })?;
        log::info!("Listening on: {host}")
      }
      host if host.starts_with("tcp://") => {
        let addr = host.trim_start_matches("tcp://");
        srv = srv.bind(addr).map_err(|err| {
          MetrsError::Error(format!("Unable to bind server: {err}"))
        })?;
        log::info!("Listening on: {host}")
      }
      _ => {
        return Err(MetrsError::Error(format!(
          "Invalid host scheme must be [tcp,unix] got: {host}"
        )))
      }
    }
  }

  Ok(srv.run())
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::time::Duration;

  use ntex::web;
  use ntex::time::interval;
  use futures::{TryStreamExt, StreamExt};

  use crate::metrix;

  pub fn before() {
    // Build a test env logger
    if std::env::var("LOG_LEVEL").is_err() {
      std::env::set_var("LOG_LEVEL", "metrsd=info,warn,error,metrsd=debug");
    }
    let _ = env_logger::Builder::new()
      .parse_env("LOG_LEVEL")
      .is_test(true)
      .try_init();
  }

  pub fn generate_server(event_emitter: EventEmitter) -> web::test::TestServer {
    before();
    // Create test server
    web::test::server(move || {
      web::App::new()
        .state(event_emitter.clone())
        .service(subscribe)
        .default_service(web::route().to(unhandled_route))
    })
  }

  #[ntex::test]
  async fn test_gen_srv() {
    let event_emitter = EventEmitter::new();
    let hosts = vec!["unix:///tmp/metrsd.sock"];
    let srv = gen_srv(&hosts, event_emitter.clone());
    assert!(srv.is_ok());
    let hosts = vec!["tcp://0.0.0.0:1245"];
    let srv = gen_srv(&hosts, event_emitter.clone());
    assert!(srv.is_ok());
    let hosts = vec!["wrong_scheme://dsadas"];
    let srv = gen_srv(&hosts, event_emitter);
    assert!(srv.is_err());
  }

  #[ntex::test]
  async fn test_subscribe() {
    let event_emitter = EventEmitter::new();
    metrix::spawn_metrics(event_emitter.clone());
    let srv = generate_server(event_emitter.clone());
    let req = srv.get("/subscribe").send();
    let resp = req.await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let mut stream = resp.into_stream();
    let mut count = 0;
    const MAX_COUNT: usize = 50;
    while let Some(item) = stream.next().await {
      let _ =
        item.unwrap_or_else(|_| panic!("Expect to receive {count} event"));
      count += 1;
      if count == MAX_COUNT {
        break;
      }
    }
    assert_eq!(count, MAX_COUNT);
    // That will close the connection
    drop(stream);
    // Wait 15 seconds to trigger cleanup
    interval(Duration::from_secs(15)).tick().await;
  }

  #[ntex::test]
  async fn test_unhandled_route() {
    let event_emitter = EventEmitter::new();
    let srv = generate_server(event_emitter.clone());
    let req = srv.get("/unhandled").send();
    let resp = req.await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
  }
}
