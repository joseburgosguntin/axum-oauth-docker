mod auth;
mod error;
mod middlewares;
mod pages;

use crate::auth::{login, logout, oauth_return};
use crate::middlewares::{check_auth, inject_user_data};
use crate::pages::{about, index, profile};
use axum::{extract::FromRef, middleware, routing::get, Extension, Router};
use sqlx::SqlitePool;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub db_pool: SqlitePool,
}

#[derive(Clone, Debug)]
pub struct UserData {
    pub user_id: i64,
    pub user_email: String,
    pub user_picture: String,
}

// # create 2 thing that from request (or request parts not sure)
// * one of them does
// 1. user_data
// 2. check auth (middleware but without using from fn)
// * the other does
// 1. optional user_data
// 2. check auth (middleware but without using from fn)
//
// in reality don't do above
// create layouts that work like this
// handler that ask all the data it needs
// maybe pass all data (optional) (hard)
// and then have sub routes that don't have to directly take that data
// how big should user data be
// probably just minimal and query the rest on the go
// should the rest get cached there (hard perserve consitency)

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = dotenvy::var("DATABASE_URL")?;
    let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;
    sqlx::migrate!().run(&db_pool).await?;

    let app_state = AppState { db_pool };
    let user_data: Option<UserData> = None;
    let app = Router::new()
        .route("/profile", get(profile))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            check_auth,
        ))
        .route("/", get(index))
        .route("/about", get(about))
        .route("/login", get(login))
        .route("/oauth_return", get(oauth_return))
        .route("/logout", get(logout))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            inject_user_data,
        ))
        .with_state(app_state)
        .layer(Extension(user_data));
    let bind_addr = &"0.0.0.0:3011".parse()?;
    axum::Server::bind(bind_addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
