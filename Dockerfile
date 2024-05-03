# stage 1 - Setup cargo-chef
FROM --platform=$BUILDPLATFORM rust:1.78.0-alpine3.19 as planner

WORKDIR /app
RUN apk add gcc g++ make
RUN cargo install cargo-chef --locked
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./crates/metrs_stubs ./crates/metrs_stubs
COPY ./crates/metrsd_client ./crates/metrsd_client
COPY ./bin/metrs/Cargo.toml ./bin/metrs/Cargo.toml
COPY ./bin/metrsd/Cargo.toml ./bin/metrsd/Cargo.toml
RUN cargo chef prepare --recipe-path recipe.json --bin ./bin/metrsd

# stage 2 - Cook our dependencies
FROM --platform=$BUILDPLATFORM rust:1.78.0-alpine3.19 as cacher

WORKDIR /app
COPY --from=planner /usr/local/cargo/bin/cargo-chef /usr/local/cargo/bin/cargo-chef
COPY --from=planner /app .
RUN apk add musl-dev libpq-dev openssl-dev
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN export ARCH=$(uname -m) \
  && cargo chef cook --release --target=$ARCH-unknown-linux-musl --recipe-path recipe.json --bin metrsd

# stage 3 - Build our project
FROM --platform=$BUILDPLATFORM rust:1.78.0-alpine3.19 as builder

## Build our metrs daemon binary
WORKDIR /app
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY --from=cacher /app .
COPY ./bin/metrsd/src ./bin/metrsd/src
COPY ./crates/metrs_stubs/src ./crates/metrs_stubs/src
COPY .git ./.git
RUN apk add musl-dev libpq-dev openssl-dev git upx
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN export ARCH=$(uname -m) \
  && cargo build --release --target=$ARCH-unknown-linux-musl --bin metrsd

## Compress the binary
RUN export ARCH=$(uname -m) \
  && cp /app/target/$ARCH-unknown-linux-musl/release/metrsd /bin/metrsd

# stage 4 - Create runtime image
FROM --platform=$BUILDPLATFORM scratch

## Copy the binary
COPY --from=builder /bin/metrsd /bin/metrsd

LABEL org.opencontainers.image.source https://github.com/nxthat/metrs
LABEL org.opencontainers.image.description Metrics Emitter

## Set entrypoint
ENTRYPOINT ["/bin/metrsd"]
