mod auth;
mod error;
mod middlewares;
mod pages;

use crate::auth::{login, logout, oauth_return};
use crate::middlewares::{check_auth, inject_user_data};
use crate::pages::{about, cookies, index, profile};
use axum::{extract::FromRef, middleware, routing::get, Extension, Router};
use sqlx::PgPool;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub db_pool: PgPool,
}

#[derive(Clone, Debug)]
pub struct UserData {
    pub user_id: i32,
    pub user_email: String,
    pub user_picture: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = dotenvy::var("DATABASE_URL")?;
    let db_pool = sqlx::postgres::PgPoolOptions::new()
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
        .route("/cookies", get(cookies))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            inject_user_data,
        ))
        .with_state(app_state)
        .layer(Extension(user_data));
    let bind_addr = &"0.0.0.0:3000".parse()?;
    axum::Server::bind(bind_addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
