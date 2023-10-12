use std::{collections::HashMap, path::PathBuf};

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

mod utils;

use utils::language::{get_language_color, get_language_size};

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

#[derive(Debug, Serialize, Deserialize)]
struct Language {
    name: String,
    size: f64,
    color: String,
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    code: StatusCode,
    message: String,
}

#[derive(Template)]
#[template(path = "repo.html")]
struct RepoTemplate {
    name: String,
    owner: String,
    html_url: String,
    avatar_url: String,
    open_issues_count: u32,
    stargazers_count: u32,
    watchers_count: u32,
    forks_count: u32,
    languages: Vec<Language>,
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

async fn hello_world() -> impl IntoResponse {
    let template = HelloTemplate { name: "world" };
    HtmlTemplate(template)
}

async fn hello_from_the_server() -> &'static str {
    "Hello!"
}

async fn get_repository_languages(url: Url) -> Result<Vec<Language>, (StatusCode, String)> {
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
        return Err((StatusCode::INTERNAL_SERVER_ERROR, error_message));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let languages_response: HashMap<String, u64> = serde_json::from_str(&response_text)
        // .map(|lang: u64| )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let languages_total = languages_response.values().sum::<u64>();
    let mut languages: Vec<Language> = Vec::new();

    for (name, value) in languages_response {
        let size = get_language_size(&value, &languages_total);
        let color = get_language_color(&name).trim_matches('"').to_string();
        let language = Language { name, size, color };
        languages.push(language);
    }

    Ok(languages)
}

async fn get_repository(
    Path((owner, repo)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let response = octocrab::instance()
        .repos(owner, repo)
        .get()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let repo = response.clone();
    let url = response.clone().languages_url.unwrap();
    let languages: Vec<Language> = get_repository_languages(url).await?;
    let template = RepoTemplate {
        name: repo.name,
        owner: match repo.owner.clone() {
            Some(owner) => owner.login,
            None => "unknown".to_string(),
        },
        html_url: match repo.html_url {
            Some(url) => url.to_string(),
            None => "/".to_string(),
        },
        avatar_url: match repo.owner.clone() {
            Some(owner) => owner.avatar_url.to_string(),
            None => "/".to_string(),
        },
        open_issues_count: repo.open_issues_count.unwrap_or(0),
        stargazers_count: repo.stargazers_count.unwrap_or(0),
        watchers_count: repo.watchers_count.unwrap_or(0),
        forks_count: repo.forks_count.unwrap_or(0),
        languages,
    };

    Ok(HtmlTemplate(template))
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
