# Build Stage
FROM rust:latest AS builder
WORKDIR /usr/src/
RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

RUN USER=root cargo new logger
WORKDIR /usr/src/logger
COPY ./logger/Cargo.toml ./Cargo.lock ./
RUN echo "openssl = { version = \"0.10\", features = [\"vendored\"] }" >> ./Cargo.toml
COPY ./api ../api
RUN cargo build --release

COPY ./logger/src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .

# Bundle Stage
FROM scratch

COPY --from=builder /usr/local/cargo/bin/logger .
USER 1000
CMD ["./logger"]