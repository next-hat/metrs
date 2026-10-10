use ntex::{web, http::StatusCode};

use crate::event_emitter::EventEmitter;
use crate::error::{MetrsError, HttpError};

async fn subscribe(
  event_emitter: &web::AppState<EventEmitter>,
  (): (),
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
  let mut srv = web::HttpServer::new({
    let event_emitter = event_emitter.clone();
    async move |_| {
      web::App::<web::AppState<EventEmitter>>::new()
        .route("/subscribe", web::get().to_with_state(subscribe))
        .default_service(web::route().to(unhandled_route))
        .build_with::<()>(web::AppState::new(event_emitter.clone()))
    }
  });

  for host in hosts {
    let host = host.to_owned().into();
    match &host {
      host if host.starts_with("unix://") => {
        let path = host.trim_start_matches("unix://");
        srv =
          srv
            .bind_uds(path, ntex::SharedCfg::default())
            .map_err(|err| {
              MetrsError::Error(format!("Unable to bind server: {err}"))
            })?;
        log::info!("Listening on: {host}")
      }
      host if host.starts_with("tcp://") => {
        let addr = host.trim_start_matches("tcp://");
        srv = srv.bind(addr, ntex::SharedCfg::default()).map_err(|err| {
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

  use std::time::{Duration, SystemTime, UNIX_EPOCH};

  use ntex::web;
  use ntex::time::sleep;
  use futures::{TryStreamExt, StreamExt};

  use crate::metrics;

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

  pub async fn generate_server(
    event_emitter: EventEmitter,
  ) -> web::test::TestServer {
    before();
    // Create test server
    web::test::server({
      let event_emitter = event_emitter.clone();
      async move |_| {
        web::App::<web::AppState<EventEmitter>>::new()
          .route("/subscribe", web::get().to_with_state(subscribe))
          .default_service(web::route().to(unhandled_route))
          .build_with::<()>(web::AppState::new(event_emitter.clone()))
      }
    })
  }

  #[ntex::test]
  async fn test_gen_srv() {
    let event_emitter = EventEmitter::new();
    let socket = std::env::temp_dir().join(format!(
      "metrsd-{}-{}.sock",
      std::process::id(),
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    let hosts = vec![format!("unix://{}", socket.display())];
    let srv = gen_srv(&hosts, event_emitter.clone());
    assert!(srv.is_ok());
    srv.unwrap().stop(false).await;
    assert!(!socket.exists());
    let hosts = vec!["tcp://127.0.0.1:0"];
    let srv = gen_srv(&hosts, event_emitter.clone());
    assert!(srv.is_ok());
    srv.unwrap().stop(false).await;
    let hosts = vec!["wrong_scheme://dsadas"];
    let srv = gen_srv(&hosts, event_emitter);
    assert!(srv.is_err());
    let err = srv.unwrap_err();
    println!("{err}");
    println!("{err:?}");
  }

  #[ntex::test]
  async fn test_subscribe() {
    let event_emitter = EventEmitter::new();
    metrics::spawn_metrics(event_emitter.clone(), 10);
    let srv = generate_server(event_emitter.clone()).await;
    let req = srv.get("/subscribe").send();
    let resp = req.await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
      resp.headers().get("content-type").unwrap(),
      "text/event-stream"
    );
    let mut stream = resp.into_stream();
    let mut count = 0;
    const MAX_COUNT: usize = 5;
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
    sleep(Duration::from_secs(15)).await;
  }

  #[ntex::test]
  async fn test_unhandled_route() {
    let event_emitter = EventEmitter::new();
    let srv = generate_server(event_emitter.clone()).await;
    let req = srv.get("/unhandled").send();
    let resp = req.await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(
      resp.json::<serde_json::Value>().await.unwrap(),
      serde_json::json!({ "msg": "Unhandled route" })
    );
  }
}
