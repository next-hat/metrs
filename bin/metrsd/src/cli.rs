use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
  /// Hosts to listen on
  #[clap(short, long)]
  hosts: Vec<String>,
}
