use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
  /// Hosts to listen on
  #[clap(
    short = 'H',
    long,
    value_parser,
    value_delimiter = ' ',
    required = true
  )]
  pub hosts: Vec<String>,
  /// Interval between two metrics publications
  #[clap(short, long, default_value = "10")]
  pub tick_interval: u64,
}

/// Cli arguments unit test
#[cfg(test)]
mod tests {
  use super::*;

  /// Test cli arguments with custom values
  #[test]
  fn test_cli() {
    let args = Cli::parse_from([
      "metrsd",
      "-H",
      "unix:///run/toto.sock",
      "-H",
      "tcp://0.0.0.0:1245",
    ]);

    assert_eq!(args.hosts.len(), 2);
    assert_eq!(args.hosts[0], "unix:///run/toto.sock");
    assert_eq!(args.hosts[1], "tcp://0.0.0.0:1245");

    let args = Cli::parse_from([
      "metrsd",
      "--hosts",
      "unix:///run/toto.sock",
      "--hosts",
      "tcp://0.0.0.0:1245",
    ]);

    assert_eq!(args.hosts.len(), 2);
    assert_eq!(args.hosts[0], "unix:///run/toto.sock");
    assert_eq!(args.hosts[1], "tcp://0.0.0.0:1245");
  }
}
