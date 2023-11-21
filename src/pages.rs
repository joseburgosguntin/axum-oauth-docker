use std::collections::HashMap;

use super::UserData;
use askama::Template;
use axum::{
    extract::{Extension, Host, Query},
    http::Request,
};

#[derive(Template)]
#[template(path = "layout.html")]
struct Layout {
    login_return_url: String,
    maybe_user_data: Option<UserData>,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    login_return_url: String,
    maybe_user_data: Option<UserData>,
}

pub async fn index<T: std::fmt::Debug>(
    Extension(maybe_user_data): Extension<Option<UserData>>,
    request: Request<T>,
) -> Index {
    let login_return_url = "?return_url=".to_owned() + &*request.uri().to_string();
    Index {
        login_return_url,
        maybe_user_data,
    }
}

#[derive(Template)]
#[template(path = "about.html")]
pub struct About {
    login_return_url: String,
    maybe_user_data: Option<UserData>,
}

pub async fn about<T>(
    Extension(maybe_user_data): Extension<Option<UserData>>,
    request: Request<T>,
) -> About {
    let login_return_url = "?return_url=".to_owned() + &*request.uri().to_string();
    About {
        login_return_url,
        maybe_user_data,
    }
}

#[derive(Template)]
#[template(path = "profile.html")]
pub struct Profile {
    login_return_url: String,
    maybe_user_data: Option<UserData>,
    user_data: UserData,
}

pub async fn profile<T>(Extension(user_data): Extension<UserData>, request: Request<T>) -> Profile {
    let login_return_url = "?return_url=".to_owned() + &*request.uri().to_string();
    Profile {
        login_return_url,
        maybe_user_data: Some(user_data.clone()),
        user_data,
    }
}

enum CookiesUrl {
    Success(String),
    Fail(String),
}

#[derive(Template)]
#[template(path = "cookies.html")]
pub struct Cookies {
    return_url: CookiesUrl,
}

pub async fn cookies(
    Host(hostname): Host,
    Query(params): Query<HashMap<String, String>>,
) -> Cookies {
    let protocol = if hostname.starts_with("localhost") || hostname.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };
    Cookies {
        return_url: params
            .get("return_url")
            .map(|x| CookiesUrl::Success(x.to_string()))
            .unwrap_or(CookiesUrl::Fail(format!("{protocol}://{hostname}/"))),
    }
}
