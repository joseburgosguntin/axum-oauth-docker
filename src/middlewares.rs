use crate::error::Result;

use super::UserData;
use anyhow::anyhow;
use axum::{
    extract::{State, TypedHeader},
    headers::Cookie,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Redirect},
};
use chrono::Utc;
use sqlx::PgPool;
use std::fmt::Debug;
use tracing::{debug, info, instrument};

#[instrument]
pub async fn inject_user_data<T: Debug>(
    State(db_pool): State<PgPool>,
    cookie: Option<TypedHeader<Cookie>>,
    mut request: Request<T>,
    next: Next<T>,
) -> Result<impl IntoResponse> {
    if let Some((cookie_p1, cookie_p2)) = cookie
        .as_ref()
        .and_then(|cookie| cookie.get("session_token").map(|s| s.split('_')))
        .and_then(|mut session_token| session_token.next().zip(session_token.next()))
    {
        let query: sqlx::Result<(i32, chrono::DateTime<Utc>, String)> = sqlx::query_as(
                r#"SELECT user_id,expires_at,session_token_p2 FROM user_sessions WHERE session_token_p1=$1"#,
            )
            .bind(cookie_p1)
            .fetch_one(&db_pool)
            .await;
        tracing::debug!(?query);

        if let Some(user_id) = query
            .as_ref()
            .ok()
            .and_then(|(user_id, expires_at, db_p2)| {
                db_p2
                    .as_bytes()
                    .try_into()
                    .ok()
                    .map(|array_db_p2| (user_id, expires_at, array_db_p2))
            })
            .zip(cookie_p2.as_bytes().try_into().ok())
            .and_then(|((user_id, expires_at, array_p2_db), array_p2_cookie)| {
                constant_time_eq::constant_time_eq_n::<36>(array_p2_cookie, array_p2_db)
                    .then_some((*user_id, expires_at))
            })
            .and_then(|(user_id, expires_at)| (expires_at > &Utc::now()).then_some(user_id))
        {
            let query: sqlx::Result<(String, String)> =
                sqlx::query_as(r#"SELECT email, picture FROM users WHERE id=$1"#)
                    .bind(user_id)
                    .fetch_one(&db_pool)
                    .await;
            if let Ok((user_email, user_picture)) = query {
                let user_data = UserData {
                    user_id,
                    user_email,
                    user_picture,
                };
                debug!(?user_data);
                request.extensions_mut().insert(Some(user_data.clone()));
                request.extensions_mut().insert(user_data);
            }
        }
    }
    let maybe_user_data = request.extensions().get::<Option<UserData>>();
    debug!(?maybe_user_data);

    Ok(next.run(request).await)
}

#[instrument]
pub async fn check_auth<T: Debug>(request: Request<T>, next: Next<T>) -> Result<impl IntoResponse> {
    if request
        .extensions()
        .get::<Option<UserData>>()
        .ok_or(anyhow!("check_auth: extensions have no UserData"))?
        .is_some()
    {
        info!("Authorized");
        Ok(next.run(request).await)
    } else {
        info!("Redirect to login");
        let login_url = "/login?return_url=".to_owned() + &*request.uri().to_string();
        Ok(Redirect::to(login_url.as_str()).into_response())
    }
}
