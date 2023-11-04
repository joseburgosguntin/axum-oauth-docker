use super::UserData;
use askama::Template;
use axum::{extract::Extension, http::Request};

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

pub async fn index<T>(
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
#[template(path = "about.html")]
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
