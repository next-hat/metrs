/*
* Metrsd is a metrics server
* It is a simple server that can be use to receive metrics from an host
* You can subscribe to the server to get the metrics in real time.
*/

use ntex::rt;
use ntex::web;
use ntex::web::middleware;
use ntex::web::{App, HttpServer};

mod cli;
mod metrix;
mod error;
mod event_emitter;

use metrix::*;
use event_emitter::EventEmitter;
use error::{HttpError, MetrsError};

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

fn conf_srv(config: &mut web::ServiceConfig) {
  config.service(subscribe);
}

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

fn spawn_background_loop(event_emitter: EventEmitter) {
  rt::Arbiter::new().exec_fn(move || {
    rt::spawn(async move {
      sync_cpu_info(event_emitter.clone());
      sync_network_info(event_emitter.clone());
      sync_disk_info(event_emitter.clone());
      sync_memory_info(event_emitter.clone());
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

  sysinfo::set_open_files_limit(0);
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
