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
    response::{AppendHeaders, IntoResponse, IntoResponseParts, Redirect},
};
use tracing::{info, instrument};

use axum_extra::extract::PrivateCookieJar;
use chrono::Utc;
use dotenvy::var;
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, AuthorizationCode, ClientId, ClientSecret,
    CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

fn base_url(Host(hostname): Host) -> String {
    let scheme = if hostname.starts_with("localhost") || hostname.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };
    format!("{scheme}://{hostname}")
}

#[instrument]
fn get_client(host: Host) -> Result<BasicClient> {
    const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
    const TOKEN_URL: &str = "https://www.googleapis.com/oauth2/v3/token";

    let redirect_url = format!("{}/oauth_return", base_url(host));
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

#[derive(Deserialize)]
pub struct ReturnUrl {
    pub return_url: Box<str>,
}

#[instrument]
pub async fn login(
    State(db_pool): State<PgPool>,
    host: Host,
    Query(ReturnUrl { return_url }): Query<ReturnUrl>,
    Extension(user_data): Extension<Option<UserData>>,
) -> Result<Redirect> {
    // check if already authenticated
    if user_data.is_some() {
        return Ok(Redirect::to("/"));
    }

    // TODO: check if return_url is valid

    let client = get_client(host)?;

    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    const SCOPE: &str = "https://www.googleapis.com/auth/userinfo.email";
    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(SCOPE.to_string()))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    _ = sqlx::query(
        r#"
        INSERT INTO oauth2_state_storage(csrf_state, pkce_code_verifier, return_url) 
        VALUES ($1, $2, $3)"#,
    )
    .bind(csrf_state.secret())
    .bind(pkce_code_verifier.secret())
    .bind(return_url)
    .execute(&db_pool)
    .await?;

    Ok(Redirect::to(authorize_url.as_str()))
}

#[derive(Deserialize)]
pub struct OAuthReturn {
    state: String,
    code: String,
}

#[instrument]
pub async fn oauth_return(
    State(db_pool): State<PgPool>,
    host: Host,
    Query(OAuthReturn { state, code }): Query<OAuthReturn>,
) -> Result<(impl IntoResponseParts, Redirect)> {
    let state = CsrfToken::new(state);
    let code = AuthorizationCode::new(code);

    let (pkce_code_verifier, return_url): (String, String) = sqlx::query_as(
        r#"DELETE FROM oauth2_state_storage WHERE csrf_state = $1 RETURNING pkce_code_verifier,return_url"#,
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

    info!("4b");
    let pkce_code_verifier = PkceCodeVerifier::new(pkce_code_verifier);

    // Exchange the code with a token.
    let client = get_client(host.clone())?;
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
    let query: sqlx::Result<(i32,), _> = sqlx::query_as(r#"SELECT id FROM users WHERE email=$1"#)
        .bind(email.as_str())
        .fetch_one(&db_pool)
        .await;
    let user_id = if let Ok(query) = query {
        query.0
    } else {
        let query: (i32,) =
            sqlx::query_as("INSERT INTO users (email, picture) VALUES ($1, $2) RETURNING id")
                .bind(email)
                .bind(picture)
                .fetch_one(&db_pool)
                .await?;
        query.0
    };

    // Create a session for the user
    let session_token_p1 = Uuid::new_v4().to_string();
    let session_token_p2 = Uuid::new_v4().to_string();
    let session_token = [session_token_p1.as_str(), "_", session_token_p2.as_str()].concat();
    let headers = AppendHeaders([(
        axum::http::header::SET_COOKIE,
        "session_token=".to_owned()
            + &*session_token
            + "; path=/; httponly; secure; SameSite=Strict",
    )]);
    let now = Utc::now();

    let x = sqlx::query(
        "INSERT INTO user_sessions
        (session_token_p1, session_token_p2, user_id, created_at, expires_at)
        VALUES ($1, $2, $3, $4, $5);",
    )
    .bind(session_token_p1)
    .bind(session_token_p2)
    .bind(user_id)
    .bind(now)
    .bind(now + chrono::Duration::days(1))
    .execute(&db_pool)
    .await?;
    println!("{x:?}");

    println!("{return_url}");

    Ok((
        headers,
        Redirect::temporary(&format!(
            "{}/login_cookie?return_url={return_url}",
            base_url(host)
        )),
    ))
}

#[instrument]
pub async fn logout(
    cookie: TypedHeader<Cookie>,
    State(db_pool): State<PgPool>,
) -> Result<impl IntoResponse> {
    if let Some(session_token) = cookie.get("session_token") {
        let session_token_1 = session_token.split('_').nth(0);
        let x = sqlx::query("DELETE FROM user_sessions WHERE session_token_p1 = $1")
            .bind(session_token_1)
            .execute(&db_pool)
            .await;
        println!("{x:?}")
    }
    let headers = axum::response::AppendHeaders([(
        axum::http::header::SET_COOKIE,
        "session_token=deleted; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT",
    )]);
    Ok((headers, Redirect::to("/")))
}
