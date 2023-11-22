FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /axum-oauth-docker

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /axum-oauth-docker/recipe.json recipe.json
# Build & cache dependencies
RUN cargo chef cook --release --recipe-path recipe.json
# Copy source code from previous stage
COPY . .
RUN cargo build --release --bin axum-oauth-docker  

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
WORKDIR /axum-oauth-docker
COPY --from=builder /axum-oauth-docker/target/release/axum-oauth-docker /usr/local/bin
RUN apt-get update && apt install -y openssl
ENTRYPOINT ["/usr/local/bin/axum-oauth-docker"]
EXPOSE 3000
