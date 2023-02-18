/*
* metrsdd is a metrics server
* It is a simple server that can be use to receive metrics from an host
* You can subscribe to the server to get the metrics in real time.
*/

mod cli;
mod error;
mod server;
mod metrix;
mod event_emitter;

use clap::Parser;

use metrix::*;
use event_emitter::EventEmitter;

#[ntex::main]
async fn main() -> std::io::Result<()> {
  let cli = cli::Cli::parse();
  // Build env logger
  if std::env::var("LOG_LEVEL").is_err() {
    std::env::set_var("LOG_LEVEL", "metrsd=info,warn,error,metrsd=debug");
  }
  env_logger::Builder::new()
    .parse_env("LOG_LEVEL")
    .format_target(false)
    .init();

  sysinfo::set_open_files_limit(0);
  let event_emitter = EventEmitter::new();

  spawn_metrics(event_emitter.clone());

  log::info!("Starting server");
  let srv = match server::gen_srv(&cli.hosts, event_emitter) {
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
