use super::UserData;
use crate::auth::ReturnUrl;
use askama::Template;
use axum::{
    extract::{Extension, Query},
    http::Uri,
};

#[derive(Template)]
#[template(path = "layout.html")]
struct Layout {
    login_return_url: Uri,
    maybe_user_data: Option<UserData>,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    login_return_url: Uri,
    maybe_user_data: Option<UserData>,
}

pub async fn index(
    login_return_url: Uri,
    Extension(maybe_user_data): Extension<Option<UserData>>,
) -> Index {
    Index {
        login_return_url,
        maybe_user_data,
    }
}

#[derive(Template)]
#[template(path = "about.html")]
pub struct About {
    login_return_url: Uri,
    maybe_user_data: Option<UserData>,
}

pub async fn about(
    login_return_url: Uri,
    Extension(maybe_user_data): Extension<Option<UserData>>,
) -> About {
    About {
        login_return_url,
        maybe_user_data,
    }
}

#[derive(Template)]
#[template(path = "profile.html")]
pub struct Profile {
    login_return_url: Uri,
    maybe_user_data: Option<UserData>,
    user_data: UserData,
}

#[rustfmt::skip]
pub async fn profile(
    login_return_url: Uri, 
    Extension(user_data): Extension<UserData>
) -> Profile {
    Profile {
        login_return_url,
        maybe_user_data: Some(user_data.clone()),
        user_data,
    }
}

#[derive(Template)]
#[template(path = "login_cookie.html")]
pub struct LoginCookie {
    return_url: Box<str>,
}

#[rustfmt::skip]
pub async fn login_cookie(
    Query(ReturnUrl { return_url }): Query<ReturnUrl>
) -> LoginCookie {
    LoginCookie { return_url }
}
