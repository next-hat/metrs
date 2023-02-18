FROM rust:1.67.0-alpine3.17

RUN apk add musl-dev make
RUN cargo install cargo-watch
RUN apk add openssl-dev

RUN mkdir -p /project
WORKDIR /project

ENTRYPOINT [ "cargo", "watch"]
