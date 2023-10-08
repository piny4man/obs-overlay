use std::{collections::HashMap, path::PathBuf};

use anyhow::{Error, Result};
use askama::Template;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use octocrab::models::Repository;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

#[derive(Debug, Deserialize)]
struct RepoRequest {
    owner: String,
    repo: String,
}

#[derive(Debug, Serialize)]
struct RepoResponse {
    repo: Repository,
    languages: HashMap<String, u64>,
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

async fn hello_world() -> impl IntoResponse {
    let template = HelloTemplate { name: "world" };
    HtmlTemplate(template)
}

async fn hello_from_the_server() -> &'static str {
    "Hello!"
}

async fn get_repository_languages(url: Url) -> Result<HashMap<String, u64>, AppError> {
    let response = Client::new()
        .get(url)
        .header("User-Agent", "repos-toolbox-api")
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !response.status().is_success() {
        let error_message = format!(
            "Error fetching language data. Status code: {}",
            response.status()
        );
        return Err(Error::msg(error_message));
    }

    let response_text = response.text().await.map_err(|e| Error::msg(e))?;

    let languages: HashMap<String, u64> =
        serde_json::from_str(&response_text).map_err(|e| Error::msg(e))?;

    Ok(languages)
}

async fn get_repository(
    Path((owner, repo)): Path<(String, String)>,
) -> Result<RepoResponse, Error> {
    let repo = octocrab::instance()
        .repos(owner, repo)
        .get()
        .await
        .map_err(|e| Error::msg(e))?;

    let url = repo.clone().languages_url.unwrap();
    let languages: HashMap<String, u64> = get_repository_languages(url).await?;

    Ok(RepoResponse { repo, languages })
}

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let api_router = Router::new()
        .route("/hello", get(hello_from_the_server))
        .route("/repo/:owner/:repo", get(get_repository));
    let router = Router::new()
        .nest("/api", api_router)
        .route("/", get(hello_world))
        .nest_service("/assets", ServeDir::new(PathBuf::from("assets")));

    Ok(router.into())
}
