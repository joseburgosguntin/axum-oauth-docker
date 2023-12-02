mod auth;
mod error;
mod middlewares;
mod pages;

use crate::auth::{login, logout, oauth_return};
use crate::middlewares::{check_auth, inject_user_data};
use crate::pages::{about, index, login_cookie, profile};
use axum::{extract::FromRef, middleware, routing::get, Extension, Router};
use sqlx::PgPool;
use tower_http::trace::{self, TraceLayer};
use tracing::{info, Level};

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
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(Level::DEBUG)
        .pretty()
        .init();

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
        .route("/login_cookie", get(login_cookie))
        .route("/logout", get(logout))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            inject_user_data,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(app_state)
        .layer(Extension(user_data));
    let bind_addr = &"0.0.0.0:3000".parse()?;
    info!("listening on {}", bind_addr);
    axum::Server::bind(bind_addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
