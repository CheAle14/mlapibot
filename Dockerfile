FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y libleptonica-dev pkg-config libclang-dev libtesseract-dev
ENV RUSTUP_HOME=/.rustup
COPY rust-toolchain.toml rust-toolchain.toml
RUN rustup install

FROM chef AS planner

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build

# We do not need the Rust toolchain to run the binary!
FROM debian:trixie-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y libleptonica-dev pkg-config libtesseract-dev ca-certificates
COPY --from=builder /app/target/debug/mlapibot-bin /usr/local/bin
ENV RUST_BACKTRACE=1
ENTRYPOINT ["mlapibot-bin", "reddit", "--release", "--scratch-dir", "/data"]