## About

You can use this repo as a template for OAuth authentication using Axum and Google OAuth.

The underling database used is SQLite using SQLx.

Askama is also used as the HTML template system. Moreover, a deployment example with GitHub Actions is provided.

## Live Demo

A live demo of this template is available at:

TODO

## Conventional setup

* Get an OAuth Client ID and key at https://console.cloud.google.com/apis/credentials, setup `http://localhost:3000/oauth_return` as an authorised redirect URI.

* Create file named `.env` at the root of the repository (same folder as the README.md), containing:

      DATABASE_URL=postgres://postgres:postgrespw@localhost:5432
      POSTGRES_PASSWORD=postgrespw
      GOOGLE_CLIENT_ID=your_google_oauth_id
      GOOGLE_CLIENT_SECRET=your_google_oauth_secret

* If you don't have `Rust` installed, see `https://rustup.rs`.

* Run the database `docker-compose up db`

* In seperate terminal Deploy with `cargo run --release`, then just browse your website at `http://localhost:3000`.

## Setup with Docker Compose

* Get an OAuth Client ID and key at https://console.cloud.google.com/apis/credentials, setup `http://localhost:3000/oauth_return` as an authorised redirect URI.

* Create file named `.env` at the root of the repository (same folder as the README.md), containing:

      DATABASE_URL=postgres://postgres:postgrespw@localhost:5432
      POSTGRES_PASSWORD=postgrespw
      GOOGLE_CLIENT_ID=your_google_oauth_id
      GOOGLE_CLIENT_SECRET=your_google_oauth_secret

* Build your OCI (Docker image) with `docker build -t ghcr.io/joseburgosguntin/axum-oauth-docker .`.

* Deploy with `docker-compose up`, then just browse your website at `http://localhost:3000`.
