use askama::Template;
use axum::{routing::get, Router};

#[derive(Template)]
#[template(path = "hello.html")]

struct HelloTemplate<'a> {
    name: &'a str,
}

async fn hello_world() -> HelloTemplate<'static> {
    HelloTemplate { name: "world" }
}

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let router = Router::new().route("/", get(hello_world));

    Ok(router.into())
}
