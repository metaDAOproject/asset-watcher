FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /asset-watcher

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /asset-watcher/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .

RUN cargo build --release --bin asset-watcher

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
RUN mkdir /run/sshd
RUN apt-get update
RUN apt-get install ca-certificates -y
RUN update-ca-certificates
EXPOSE 3000
WORKDIR /asset-watcher
COPY --from=builder /asset-watcher/target/release/asset-watcher /usr/local/bin

ENTRYPOINT ["/usr/local/bin/asset-watcher"]