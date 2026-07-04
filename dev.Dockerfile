FROM rust:1.96.1-alpine3.23

RUN apk add musl-dev make
RUN cargo install cargo-watch
RUN apk add openssl-dev

RUN mkdir -p /project
WORKDIR /project

ENTRYPOINT [ "cargo", "watch"]
