# FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
FROM rust:1.73-alpine as chef
RUN apk add openssl-dev musl-dev
# FROM messense/rust-musl-cross:x86_64-musl as chef
# FROM rust:1 AS chef 
ENV SQLX_OFFLINE=true
RUN cargo install cargo-chef
WORKDIR /axum-oauth-docker

FROM chef AS planner
# Copy source code from previous stage
COPY . .
# Generate info for caching dependencies
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /axum-oauth-docker/recipe.json recipe.json
# Build & cache dependencies
RUN cargo chef cook --release --recipe-path recipe.json
# RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Copy source code from previous stage
COPY . .
# RUN apt update
# RUN apt-get install -y \
#     curl \
#     clang \
#     gcc \
#     g++ \
#     zlib1g-dev \
#     libmpc-dev \
#     libmpfr-dev \
#     libgmp-dev \
#     git \
#     cmake \
#     pkg-config \
#     libssl-dev \
#     build-essential
# Build application
RUN cargo build --release 
# RUN cargo build --release --target x86_64-unknown-linux-musl

# Create a new stage with a minimal image
FROM scratch
COPY --from=builder /axum-oauth-docker/target/release/axum-oauth-docker /axum-oauth-docker
# COPY --from=builder /axum-oauth-docker/target/x86_64-unknown-linux-musl/release/axum-oauth-docker /axum-oauth-docker
ENTRYPOINT ["/axum-oauth-docker"]
EXPOSE 3000
