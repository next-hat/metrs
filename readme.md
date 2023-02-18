<div align="center">
  <h1>Metrs</h1>
  <h3>Metrics microservice</h3>
  <p>

  [![Stars](https://img.shields.io/github/stars/nxthat/metrs?label=%E2%AD%90%20stars%20%E2%AD%90)](https://github.com/nxthat/metrs)
  [![Build With](https://img.shields.io/badge/built_with-Rust-dca282.svg?style=flat)](https://github.com/nxthat/metrs)
  [![Chat on Discord](https://img.shields.io/discord/1011267493114949693?label=chat&logo=discord&style=flat)](https://discord.gg/WV4Aac8uZg)

  </p>

  <p>

  [![Tests](https://github.com/nxthat/metrs/actions/workflows/tests.yml/badge.svg)](https://github.com/nxthat/metrs/actions/workflows/tests.yml)
  [![Clippy](https://github.com/nxthat/metrs/actions/workflows/clippy.yml/badge.svg)](https://github.com/nxthat/metrs/actions/workflows/clippy.yml)

  </p>

  <p>

[![codecov](https://codecov.io/gh/nxthat/metrs/branch/master/graph/badge.svg?token=N1P1BL5RWH)](https://codecov.io/gh/nxthat/metrs)

  </p>

</div>

## Overview

Metrs is a microservice to help you gather metrics information about an host
It will send in real time the Cpu, Memory, Disk and Network usage information.
This service don't store anything in database, it he designed to only emit information.

## The daemon

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

A rust client is available for you to use using [ntex](https://github.com/)

You can install it by running:

```sh
cargo add metrsd_client
```

And then call the subscribe event

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

There is not CLI for the moment but it's planned
