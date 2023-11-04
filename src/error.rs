#[derive(thiserror::Error, Debug, askama::Template)]
#[template(path = "error.html")]
pub enum Error {
    Oauth2(#[from] oauth2::url::ParseError),
    Sqlx(#[from] sqlx::Error),
    Tokio(#[from] tokio::task::JoinError),
    Reqwest(#[from] reqwest::Error),
    Anyhow(#[from] anyhow::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
