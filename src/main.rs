use std::{collections::HashMap, sync::Arc};
use std::sync::{Mutex};
use axum::{routing::get, routing::post, Router, Json};
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use rand::{distributions::Alphanumeric, Rng};
use url::Url;

use tokio;


async fn redirect_handler(
    State(url_map): State<Arc<Mutex<HashMap<String, String>>>>,
    Path(alias): Path<String>,
) -> Result<Redirect, StatusCode> {
    let map = url_map.lock().unwrap();
    if let Some(long_url) = map.get(&alias) {
        println!("Redirecting '/{}' to {}", alias, long_url);
        Ok(Redirect::permanent(long_url.as_str()))
    } else {
        println!("No mapping found for '/{}'", alias);
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize)]
struct ShortenRequest {
    url: String,
}

#[derive(Serialize)]
struct ShortenResponse {
    short_url: String,
}

async fn shorten_handler(
    State(url_map_mutex): State<Arc<Mutex<HashMap<String, String>>>>,
    Json(payload): Json<ShortenRequest>, 
) -> impl IntoResponse { 

    let alias: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    // Hier wäre ein guter Ort für URL-Validierung mit dem `url`-Crate (optional)
    if Url::parse(&payload.url).is_err() {
        return Err(StatusCode::BAD_REQUEST); 
    }

    { // Neuer Scope, um die Mutex-Sperre schneller freizugeben
        // Map für Schreibzugriff sperren
        let mut map = url_map_mutex.lock().unwrap();
        println!("Shortening '{}' to '/{}'", payload.url, alias);
        map.insert(alias.clone(), payload.url);
        // Mutex wird hier freigegeben, wenn `map` den Scope verlässt
    }

    // TODO: Basis-URL aus Konfiguration lesen
    let short_url = format!("http://127.0.0.1:3000/{}", alias);
    let response = ShortenResponse { short_url };

    Ok((StatusCode::CREATED, Json(response)))
}

#[tokio::main]
async fn main() { 
    let url_map: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new([
        ("rust".to_string(), "https://www.rust-lang.org".to_string()),
        ("google".to_string(), "https://www.google.com".to_string()),
        ("github".to_string(), "https://www.github.com".to_string()),]
        .iter().cloned().collect()
    ));

    let app = Router::new()
        .route("/:alias", get(redirect_handler))
        .route("/shorten", post(shorten_handler)) 
        .with_state(url_map); 

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on: {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}