use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use dotenvy::dotenv;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool; 
use std::env;
use url::Url;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};


async fn redirect_handler(
    State(pool): State<PgPool>,
    Path(alias): Path<String>,
) -> Result<Redirect, StatusCode> {
    info!("Versuche Weiterleitung für: {}", alias);

    let result: Result<Option<(String,)>, sqlx::Error> = sqlx::query_as("SELECT original_url FROM urls WHERE short_code = $1")
        .bind(&alias)
        .fetch_optional(&pool) 
        .await;

    match result {
        Ok(Some((original_url,))) => {
            info!("Leite '{}' weiter nach: {}", alias, original_url);
            Ok(Redirect::permanent(&original_url)) 
        }
        Ok(None) => {
            info!("Kein Mapping gefunden für: {}", alias);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            eprintln!("Datenbankfehler beim Suchen von {}: {}", alias, e); 
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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
    State(pool): State<PgPool>,
    Json(payload): Json<ShortenRequest>,
) -> Result<impl IntoResponse, StatusCode> { 

    // URL validieren
    if Url::parse(&payload.url).is_err() {
        info!("Ungültige URL empfangen: {}", payload.url);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Generiere einen zufälligen Alias (ggf. wiederholen bei Kollision)
    let alias: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6) 
        .map(char::from)
        .collect();

    info!("Versuche '{}' zu kürzen auf Alias: {}", payload.url, alias);

    let insert_result = sqlx::query(
        "INSERT INTO urls (short_code, original_url) VALUES ($1, $2)"
    )
    .bind(&alias)
    .bind(&payload.url)
    .execute(&pool)
    .await;

    match insert_result {
        Ok(_) => {
            // TODO: Basis-URL aus Konfiguration lesen oder dynamisch ermitteln
            let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
            let short_url = format!("{}/{}", base_url, alias);
            let response = ShortenResponse { short_url };
            info!("URL erfolgreich gekürzt: {} -> {}", payload.url, response.short_url);
            Ok((StatusCode::CREATED, Json(response))) 
        }
        Err(e) => {
             if let Some(db_err) = e.as_database_error() {
                 if db_err.code().as_deref() == Some("23505") {
                    eprintln!("Kollision beim Alias '{}'. Versuche es erneut oder implementiere Wiederholungslogik.", alias);
                    return Err(StatusCode::CONFLICT);
                 }
            }
            eprintln!("Datenbankfehler beim Einfügen: {}", e); 
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> { 
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "url_shortener=info,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    
    dotenv().ok();
    info!("Environment Variablen geladen.");

   
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL muss gesetzt sein");
    info!("Verbinde mit Datenbank...");

    
    let pool = PgPoolOptions::new()
        .max_connections(5) 
        .connect(&database_url)
        .await?; 
    info!("Datenbankverbindungspool erfolgreich erstellt.");

    // --- Datenbankmigrationen ausführen ---
    info!("Führe Datenbankmigrationen aus...");
    sqlx::migrate!("./migrations") 
        .run(&pool) 
        .await?; 
    info!("Datenbankmigrationen erfolgreich abgeschlossen.");

    // --- Axum App Router erstellen ---.
    let app = Router::new()
        .route("/:alias", get(redirect_handler))
        .route("/shorten", post(shorten_handler))
        .with_state(pool);

    // --- Server starten ---
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?; 
    info!("Server lauscht auf: {}", listener.local_addr()?);
    axum::serve(listener, app.into_make_service()).await?; 

    Ok(())
}