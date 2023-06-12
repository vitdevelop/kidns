FROM rust:latest as build

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

COPY . /kidns
WORKDIR /kidns

# RUN cargo build
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine

WORKDIR /kidns

# Copy our build
COPY --from=build /kidns/target/x86_64-unknown-linux-musl/release/kidns ./app

ENTRYPOINT ["/kidns/app"]
