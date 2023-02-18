# stage 1 - Setup cargo-chef
FROM rust:1.67.0-alpine3.17 as planner

WORKDIR /app
RUN apk add gcc g++ make
RUN cargo install cargo-chef --locked
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./crates/metrs_stubs/Cargo.toml ./crates/metrs_stubs/Cargo.toml
COPY ./crates/metrsd_client/Cargo.toml ./crates/metrsd_client/Cargo.toml
COPY ./bin/metrs/Cargo.toml ./bin/metrs/Cargo.toml
COPY ./bin/metrsd/Cargo.toml ./bin/metrsd/Cargo.toml
RUN cargo chef prepare --recipe-path recipe.json --bin ./bin/metrsd

# state 2 - Cook our dependencies
FROM rust:1.67.0-alpine3.17 as cacher

WORKDIR /app
COPY --from=planner /usr/local/cargo/bin/cargo-chef /usr/local/cargo/bin/cargo-chef
COPY --from=planner /app .
RUN apk add musl-dev libpq-dev openssl-dev
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo chef cook --release --target=x86_64-unknown-linux-musl --recipe-path recipe.json --bin metrsd

# stage 3 - Build our project
FROM rust:1.67.0-alpine3.17 as builder

## Build our metrs daemon binary
WORKDIR /app
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY --from=cacher /app .
COPY ./bin/metrsd/src ./bin/metrsd/src
COPY ./crates/metrs_stubs/src ./crates/metrs_stubs/src
COPY .git ./.git
RUN apk add musl-dev libpq-dev openssl-dev git upx
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo build --release --target=x86_64-unknown-linux-musl --bin metrsd

## Strip and compress the binary
RUN strip /app/target/x86_64-unknown-linux-musl/release/metrsd
RUN upx /app/target/x86_64-unknown-linux-musl/release/metrsd

# stage 4 - Create runtime image
FROM scratch

## Copy the binary
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/metrsd /usr/local/bin/metrsd

## Set entrypoint
ENTRYPOINT ["/usr/local/bin/metrsd"]
