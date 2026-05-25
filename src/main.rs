mod auth;
mod errors;
mod handlers;
mod models;
mod notifications;

use axum::{routing::{get, put, post}, Router, middleware};
use sqlx::sqlite::SqlitePoolOptions;
use tower_http::services::{ServeDir, ServeFile};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use utoipa::OpenApi;

use crate::models::*;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::sensoren_laden,
        handlers::sensor_erstellen,
        handlers::sensor_aktualisieren,
        handlers::sensor_loeschen,
        handlers::messwerte_laden,
        handlers::messwert_erstellen,
        handlers::alarm_config_laden,
        handlers::alarm_config_erstellen,
        handlers::alarm_config_aktualisieren,
        handlers::alarm_historie_laden,
        handlers::firmware_version,
    ),
    components(schemas(
        Sensor, NeuerSensor, SensorUpdate,
        Messwert, NeuerMesswert,
        AlarmConfig, NeueAlarmConfig, AlarmConfigUpdate,
        AlarmHistorie, FirmwareInfo,
    )),
    tags(
        (name = "Sensoren", description = "Sensoren verwalten"),
        (name = "Messwerte", description = "Temperatur- und Akkudaten"),
        (name = "Alarm", description = "Alarm-Konfiguration und Historie"),
        (name = "Firmware", description = "OTA Firmware-Updates"),
    ),
    info(
        title = "Freezer Monitor API",
        version = "1.0.0",
        description = "REST API zur Überwachung von Tiefkühltruhen"
    )
)]
struct ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub ntfy_kanal: String,
    pub firmware_version: std::sync::Arc<tokio::sync::RwLock<String>>,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub erlaubte_emails: Vec<String>,
    pub jwt_secret: String,
    pub api_key: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let ntfy_kanal = std::env::var("NTFY_KANAL")
        .expect("NTFY_KANAL muss in .env gesetzt sein");

    let server_port = std::env::var("SERVER_PORT")
        .unwrap_or(String::from("3000"));

    let firmware_version_str = std::fs::read_to_string("firmware/version.txt")
    .unwrap_or_else(|_| std::env::var("FIRMWARE_VERSION")
        .unwrap_or(String::from("1.0.0")));

    let firmware_version = std::sync::Arc::new(
        tokio::sync::RwLock::new(firmware_version_str)
    );

    let google_client_id = std::env::var("GOOGLE_CLIENT_ID")
        .expect("GOOGLE_CLIENT_ID muss in .env gesetzt sein");

    let google_client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .expect("GOOGLE_CLIENT_SECRET muss in .env gesetzt sein");

    let google_redirect_uri = std::env::var("GOOGLE_REDIRECT_URI")
        .expect("GOOGLE_REDIRECT_URI muss in .env gesetzt sein");

    let erlaubte_emails: Vec<String> = std::env::var("ERLAUBTE_EMAILS")
        .expect("ERLAUBTE_EMAILS muss in .env gesetzt sein")
        .split(',')
        .map(|e| e.trim().to_string())
        .collect();

    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET muss in .env gesetzt sein");

    let api_key = std::env::var("API_KEY")
    .expect("API_KEY muss in .env gesetzt sein");

    let db = SqlitePoolOptions::new()
        .connect("sqlite:freezer.db?mode=rwc")
        .await
        .expect("Datenbank-Verbindung fehlgeschlagen");

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Migration fehlgeschlagen");

    println!("Datenbank bereit!");

    tokio::spawn(notifications::starte_offline_pruefung(
        db.clone(),
        ntfy_kanal.clone(),
    ));

    tokio::spawn(notifications::starte_daten_bereinigung(
    db.clone(),
    ));

    let state = AppState {
        db,
        ntfy_kanal,
        firmware_version,
        google_client_id,
        google_client_secret,
        google_redirect_uri,
        erlaubte_emails,
        jwt_secret,
        api_key,
    };

    // Rate-Limiting: 1 Request/Sekunde nachfüllen, Burst bis 60
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(60)
        .finish()
        .unwrap();

    // Hintergrund-Task: alte Rate-Limit-Einträge aufräumen
    let governor_limiter = governor_conf.limiter().clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            governor_limiter.retain_recent();
        }
    });

    let app = Router::new()
        .route("/api/v1/sensoren", get(handlers::sensoren_laden).post(handlers::sensor_erstellen))
        .route("/api/v1/sensoren/{id}", put(handlers::sensor_aktualisieren).delete(handlers::sensor_loeschen))
        .route("/api/v1/messwerte", get(handlers::messwerte_laden).post(handlers::messwert_erstellen))
        .route("/api/v1/alarm-config", get(handlers::alarm_config_laden).post(handlers::alarm_config_erstellen))
        .route("/api/v1/alarm-config/{sensor_id}", put(handlers::alarm_config_aktualisieren))
        .route("/api/v1/alarm-config/{sensor_id}/quittieren", post(handlers::alarm_quittieren))
        .route("/api/v1/alarm-historie", get(handlers::alarm_historie_laden))
        .route("/api/v1/firmware/version", get(handlers::firmware_version))
        .route("/api/v1/firmware/download", get(handlers::firmware_download))
        .route("/api/v1/firmware/upload", post(handlers::firmware_upload))
        .route("/api/v1/auth/login", get(auth::login))
        .route("/api/v1/auth/callback", get(auth::callback))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/auth/logout", get(auth::logout))
        .route("/api-docs/openapi.json", get(api_docs))
        .route("/swagger", get(swagger_ui))
        .with_state(state)
        .layer(middleware::from_fn(security_headers))
        .layer(GovernorLayer::new(governor_conf))
        .fallback_service(ServeDir::new("frontend/dist").fallback(ServeFile::new("frontend/dist/index.html")));

    let adresse = format!("0.0.0.0:{}", server_port);
    let listener = tokio::net::TcpListener::bind(&adresse)
        .await
        .unwrap();

    println!("Server läuft auf http://localhost:{}", server_port);
    println!("Swagger UI: http://localhost:{}/swagger", server_port);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    ).await.unwrap();
}

async fn security_headers(
    request: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline' https://unpkg.com; style-src 'self' 'unsafe-inline' https://unpkg.com; img-src 'self' data:; connect-src 'self'"
            .parse()
            .unwrap(),
    );
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    headers.insert(
        "Permissions-Policy",
        "camera=(), microphone=(), geolocation=()".parse().unwrap(),
    );

    response
}

async fn api_docs() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(ApiDoc::openapi())
}

async fn swagger_ui() -> axum::response::Html<String> {
    axum::response::Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Freezer Monitor API</title>
        <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
    </head>
    <body>
        <div id="swagger-ui"></div>
        <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
        <script>
            SwaggerUIBundle({
                url: '/api-docs/openapi.json',
                dom_id: '#swagger-ui',
            })
        </script>
    </body>
    </html>
    "#.to_string())
}