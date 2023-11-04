// Code adapted from https://github.com/ramosbugs/oauth2-rs/blob/main/examples/google.rs
//
// Must set the enviroment variables:
// GOOGLE_CLIENT_ID=xxx
// GOOGLE_CLIENT_SECRET=yyy

use super::UserData;
use crate::error::Result;
use anyhow::anyhow;
use axum::{
    extract::{Extension, Host, Query, State, TypedHeader},
    headers::Cookie,
    http::Request,
    response::{IntoResponse, Redirect},
};
use chrono::Utc;
use dotenvy::var;
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, AuthorizationCode, ClientId, ClientSecret,
    CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope,
    TokenResponse, TokenUrl,
};
use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

fn get_client(hostname: String) -> Result<BasicClient> {
    const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
    const TOKEN_URL: &str = "https://www.googleapis.com/oauth2/v3/token";
    let protocol = if hostname.starts_with("localhost") || hostname.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };

    let redirect_url = format!("{}://{}/oauth_return", protocol, hostname);
    const REVOCATION_URL: &str = "https://oauth2.googleapis.com/revoke";

    // Set up the config for the Google OAuth2 process.
    let client = BasicClient::new(
        ClientId::new(var("GOOGLE_CLIENT_ID").unwrap()),
        Some(ClientSecret::new(var("GOOGLE_CLIENT_SECRET").unwrap())),
        AuthUrl::new(AUTH_URL.to_string())?,
        TokenUrl::new(TOKEN_URL.to_string()).ok(),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url)?)
    .set_revocation_uri(RevocationUrl::new(REVOCATION_URL.to_string())?);
    Ok(client)
}

pub async fn login(
    Extension(user_data): Extension<Option<UserData>>,
    Query(mut params): Query<HashMap<String, String>>,
    State(db_pool): State<SqlitePool>,
    Host(hostname): Host,
) -> Result<Redirect> {
    if user_data.is_some() {
        // check if already authenticated
        return Ok(Redirect::to("/"));
    }

    let return_url = params
        .remove("return_url")
        .unwrap_or_else(|| "/".to_string());
    // TODO: check if return_url is valid

    let client = get_client(hostname)?;

    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    const SCOPE: &str = "https://www.googleapis.com/auth/userinfo.email";
    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(SCOPE.to_string()))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    sqlx::query(
        r#"
        INSERT INTO oauth2_state_storage 
        (csrf_state, pkce_code_verifier, return_url) 
        VALUES (?, ?, ?);"#,
    )
    .bind(csrf_state.secret())
    .bind(pkce_code_verifier.secret())
    .bind(return_url)
    .execute(&db_pool)
    .await?;

    Ok(Redirect::to(authorize_url.as_str()))
}

pub async fn oauth_return<T>(
    Query(mut params): Query<HashMap<String, String>>,
    State(db_pool): State<SqlitePool>,
    Host(hostname): Host,
    mut request: Request<T>,
) -> Result<impl IntoResponse> {
    let state = CsrfToken::new(
        params
            .remove("state")
            .ok_or(anyhow!("OAuth: without state"))?,
    );
    let code = AuthorizationCode::new(
        params
            .remove("code")
            .ok_or(anyhow!("OAuth: without code"))?,
    );

    let query: (String, String) = sqlx::query_as(
        r#"DELETE FROM oauth2_state_storage WHERE csrf_state = ? RETURNING pkce_code_verifier,return_url"#,
    )
    .bind(state.secret())
    .fetch_one(&db_pool)
    .await?;

    // Alternative:
    // let query: (String, String) = sqlx::query_as(
    //     r#"SELECT pkce_code_verifier,return_url FROM oauth2_state_storage WHERE csrf_state = ?"#,
    // )
    // .bind(state.secret())
    // .fetch_one(&db_pool)
    // .await?;
    // let _ = sqlx::query("DELETE FROM oauth2_state_storage WHERE csrf_state = ?")
    //     .bind(state.secret())
    //     .execute(&db_pool)
    //     .await;

    let pkce_code_verifier = query.0;
    let return_url = query.1;
    let pkce_code_verifier = PkceCodeVerifier::new(pkce_code_verifier);

    // Exchange the code with a token.
    let client = get_client(hostname)?;
    let token_response = tokio::task::spawn_blocking(move || {
        client
            .exchange_code(code)
            .set_pkce_verifier(pkce_code_verifier)
            .request(http_client)
    })
    .await?
    .map_err(|x| anyhow::anyhow!(x))?;
    let access_token = token_response.access_token().secret();

    // Get user info from Google
    let url =
        "https://www.googleapis.com/oauth2/v2/userinfo?oauth_token=".to_owned() + access_token;
    let body = reqwest::get(url)
        .await
        .map_err(|_| anyhow!("OAuth: reqwest failed to query userinfo"))?
        .text()
        .await
        .map_err(|_| anyhow!("OAuth: reqwest received invalid userinfo"))?;
    let mut body: serde_json::Value = serde_json::from_str(body.as_str())
        .map_err(|_| anyhow!("OAuth: Serde failed to parse userinfo"))?;
    let email = body["email"]
        .take()
        .as_str()
        .ok_or(anyhow!("OAuth: Serde failed to parse email address"))?
        .to_owned();
    let picture = body["picture"]
        .take()
        .as_str()
        .ok_or(anyhow!("OAuth: Serde failed to parse picture"))?
        .to_owned();
    let verified_email = body["verified_email"]
        .take()
        .as_bool()
        .ok_or(anyhow!("OAuth: Serde failed to parse verified_email"))?;
    if !verified_email {
        return Err(anyhow::anyhow!("OAuth: email address is not verified").into());
    }

    // Check if user exists in database
    // If not, create a new user
    let query: sqlx::Result<(i64,), _> = sqlx::query_as(r#"SELECT id FROM users WHERE email=?"#)
        .bind(email.as_str())
        .fetch_one(&db_pool)
        .await;
    let user_id = if let Ok(query) = query {
        query.0
    } else {
        let query: (i64,) =
            sqlx::query_as("INSERT INTO users (email, picture) VALUES (?1, ?2) RETURNING id")
                .bind(email.clone())
                .bind(picture.clone())
                .fetch_one(&db_pool)
                .await?;
        query.0
    };

    // Create a session for the user
    let session_token_p1 = Uuid::new_v4().to_string();
    let session_token_p2 = Uuid::new_v4().to_string();
    let session_token = [session_token_p1.as_str(), "_", session_token_p2.as_str()].concat();
    let headers = axum::response::AppendHeaders([(
        axum::http::header::SET_COOKIE,
        "session_token=".to_owned()
            + &*session_token
            + "; path=/; httponly; secure; samesite=strict",
    )]);
    let now = Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO user_sessions
        (session_token_p1, session_token_p2, user_id, created_at, expires_at)
        VALUES (?, ?, ?, ?, ?);",
    )
    .bind(session_token_p1)
    .bind(session_token_p2)
    .bind(user_id)
    .bind(now)
    .bind(now + 60 * 60 * 24)
    .execute(&db_pool)
    .await?;

    let user_data = UserData {
        user_id,
        user_email: email,
        user_picture: picture,
    };
    request.extensions_mut().insert(Some(user_data.clone()));
    request.extensions_mut().insert(user_data);

    dbg!(request.extensions());

    Ok((headers, Redirect::to(return_url.as_str())))
}

pub async fn logout(
    cookie: TypedHeader<Cookie>,
    State(db_pool): State<SqlitePool>,
) -> Result<impl IntoResponse> {
    if let Some(session_token) = cookie.get("session_token") {
        let session_token_1 = session_token.split('_').nth(0);
        let _ = sqlx::query("DELETE FROM user_sessions WHERE session_token_1 = ?")
            .bind(session_token_1)
            .execute(&db_pool)
            .await;
    }
    let headers = axum::response::AppendHeaders([(
        axum::http::header::SET_COOKIE,
        "session_token=deleted; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT",
    )]);
    Ok((headers, Redirect::to("/")))
}
