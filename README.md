<div align="center">
  <h1>Metrs</h1>
  <h3>A Metrics Service</h3>
  <p>

  [![Stars](https://img.shields.io/github/stars/anonkey/metrs?label=%E2%AD%90%20stars%20%E2%AD%90)](https://github.com/anonkey/metrs)
  [![Build With](https://img.shields.io/badge/built_with-Rust-dca282.svg?style=flat)](https://github.com/anonkey/metrs)
  [![Chat on Discord](https://img.shields.io/discord/1011267493114949693?label=chat&logo=discord&style=flat)](https://discord.gg/WV4Aac8uZg)

  </p>

  <p>

  [![Tests](https://github.com/anonkey/metrs/actions/workflows/tests.yml/badge.svg)](https://github.com/anonkey/metrs/actions/workflows/tests.yml)
  [![Clippy](https://github.com/anonkey/metrs/actions/workflows/clippy.yml/badge.svg)](https://github.com/anonkey/metrs/actions/workflows/clippy.yml)

  </p>


  <p>

[![codecov](https://codecov.io/gh/anonkey/metrs/branch/master/graph/badge.svg?token=N1P1BL5RWH)](https://codecov.io/gh/anonkey/metrs)

  </p>

</div>

## Overview

Metrs is a lightweight and efficient service that provides real-time metrics information about a host's CPU, memory, disk, and network usage.<br/>
Unlike traditional services, Metrs doesn't store data in a database; its sole purpose is to emit information.

## The daemon

To use the Metrs daemon, run the following command:

```console
Usage: metrsd --hosts <HOSTS>

Options:
  -H, --hosts <HOSTS>  Hosts to listen on
  -h, --help           Print help
```

Example:

```sh
metrsd --hosts tcp://127.0.0.1:8080
```

## The client

Metrs provides a Rust client that you can use with [ntex](https://github.com/ntex-rs/ntex). To install the client, run the following command:

```sh
cargo add metrsd_client
```

You can then call the subscribe event using the following code:

```rust
use metrsd_client::MetrsdClient;

#[ntex::main]
async fn main() -> std::io::Result<()> {
  let client = MetrsdClient::connect("http://localhost:8080");

  let stream = client.subscribe().await.unwrap();

  while let Some(ev) = stream.next().await {
    println!("{ev:#?}");
  }
  Ok(())
}
```

## The cli

There is no CLI available for Metrs at the moment, but it's planned for future releases.
