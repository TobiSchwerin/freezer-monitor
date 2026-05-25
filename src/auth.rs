use axum::{
    extract::{FromRequestParts, Query, State},
    http::{header, request::Parts, StatusCode},
    response::{Html, IntoResponse, Redirect},
    Json,
};

use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub email: String,
    pub exp: usize,
}

#[derive(Deserialize)]
pub struct AuthCallback {
    pub code: String,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub email: String,
    pub eingeloggt: bool,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    email: String,
}

pub async fn login(State(state): State<AppState>) -> Redirect {
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=email&access_type=offline",
        state.google_client_id,
        state.google_redirect_uri
    );
    Redirect::temporary(&url)
}

pub async fn callback(
    State(state): State<AppState>,
    Query(params): Query<AuthCallback>,
) -> impl IntoResponse {
    // Code gegen Token tauschen
    let client = reqwest::Client::new();
    let token_response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", params.code.as_str()),
            ("client_id", state.google_client_id.as_str()),
            ("client_secret", state.google_client_secret.as_str()),
            ("redirect_uri", state.google_redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await;

    let token_response = match token_response {
        Ok(r) => r,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Token-Anfrage fehlgeschlagen").into_response(),
    };

    let google_token: GoogleTokenResponse = match token_response.json().await {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Token-Parsing fehlgeschlagen").into_response(),
    };

    // E-Mail von Google holen
    let user_response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .header("Authorization", format!("Bearer {}", google_token.access_token))
        .send()
        .await;

    let user_info: GoogleUserInfo = match user_response {
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "User-Info fehlgeschlagen").into_response(),
        },
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "User-Anfrage fehlgeschlagen").into_response(),
    };

    // Prüfen ob E-Mail erlaubt ist
    if !state.erlaubte_emails.contains(&user_info.email) {
        let html = format!(r#"
<!DOCTYPE html>
<html lang="de">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Zugriff verweigert – Freezer Monitor</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            background: #0f172a;
            color: #e2e8f0;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .karte {{
            background: #1e293b;
            border: 1px solid #2d3a4f;
            border-radius: 16px;
            padding: 48px 40px;
            max-width: 460px;
            width: 90%;
            text-align: center;
        }}
        .icon {{ font-size: 64px; margin-bottom: 24px; }}
        h1 {{
            font-size: 24px;
            font-weight: 700;
            color: #f1f5f9;
            margin-bottom: 12px;
        }}
        .email {{
            background: #0f172a;
            border: 1px solid #334155;
            border-radius: 8px;
            padding: 10px 16px;
            margin: 16px 0 24px;
            font-size: 14px;
            color: #94a3b8;
            word-break: break-all;
        }}
        p {{
            color: #94a3b8;
            font-size: 15px;
            line-height: 1.6;
            margin-bottom: 32px;
        }}
        .buttons {{
            display: flex;
            flex-direction: column;
            gap: 12px;
        }}
        .btn {{
            display: block;
            padding: 12px 24px;
            border-radius: 8px;
            font-size: 15px;
            font-weight: 600;
            text-decoration: none;
            transition: opacity 0.2s;
        }}
        .btn:hover {{ opacity: 0.85; }}
        .btn-primary {{
            background: #3b82f6;
            color: white;
        }}
        .btn-secondary {{
            background: #1e293b;
            color: #94a3b8;
            border: 1px solid #334155;
        }}
    </style>
</head>
<body>
    <div class="karte">
        <div class="icon">🔒</div>
        <h1>Zugriff nicht erlaubt</h1>
        <div class="email">{}</div>
        <p>Dieses Google-Konto ist nicht für den Freezer Monitor freigeschaltet. Bitte wende dich an den Administrator.</p>
        <div class="buttons">
            <a href="/api/v1/auth/login" class="btn btn-primary">🔄 Mit anderem Konto anmelden</a>
            <a href="/" class="btn btn-secondary">← Zurück zur Hauptseite</a>
        </div>
    </div>
</body>
</html>
        "#, user_info.email);

        return (StatusCode::FORBIDDEN, Html(html)).into_response();
    }

    // JWT erstellen
    let claims = Claims {
        email: user_info.email,
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ).unwrap();

    // Cookie setzen und zum Dashboard weiterleiten
    let cookie = format!("token={}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=86400", token);

    (
        [(header::SET_COOKIE, cookie), (header::LOCATION, "/".to_string())],
        StatusCode::FOUND,
    ).into_response()
}

pub async fn me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Json<UserInfo> {
    let email = email_aus_cookie(&headers, &state.jwt_secret);

    match email {
        Some(email) => Json(UserInfo { email, eingeloggt: true }),
        None => Json(UserInfo { email: String::new(), eingeloggt: false }),
    }
}

pub async fn logout() -> impl IntoResponse {
    let cookie = "token=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0";

    (
        [(header::SET_COOKIE, cookie.to_string()), (header::LOCATION, "/".to_string())],
        StatusCode::FOUND,
    ).into_response()
}

pub fn email_aus_cookie(headers: &axum::http::HeaderMap, jwt_secret: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;

    let token = cookie_header
        .split(';')
        .find(|c| c.trim().starts_with("token="))?
        .trim()
        .strip_prefix("token=")?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    ).ok()?;

    Some(token_data.claims.email)
}

// --- AuthGuard: prüft JWT-Cookie (für Browser/Frontend) ---
pub struct AuthGuard;

impl FromRequestParts<AppState> for AuthGuard {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match email_aus_cookie(&parts.headers, &state.jwt_secret) {
            Some(_) => Ok(AuthGuard),
            None => Err((StatusCode::UNAUTHORIZED, "Nicht eingeloggt")),
        }
    }
}

// --- ApiKeyGuard: prüft X-Api-Key Header (für ESP32) ---
pub struct ApiKeyGuard;

impl FromRequestParts<AppState> for ApiKeyGuard {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let key = parts.headers
            .get("X-Api-Key")
            .and_then(|v| v.to_str().ok());

        match key {
            Some(k) if k == state.api_key => Ok(ApiKeyGuard),
            _ => Err((StatusCode::UNAUTHORIZED, "Ungültiger API-Key")),
        }
    }
}

// --- FirmwareGuard: akzeptiert JWT-Cookie ODER API-Key (für OTA + Browser) ---
pub struct FirmwareGuard;

impl FromRequestParts<AppState> for FirmwareGuard {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // Erst JWT prüfen (Browser-Login)
        if email_aus_cookie(&parts.headers, &state.jwt_secret).is_some() {
            return Ok(FirmwareGuard);
        }

        // Dann API-Key prüfen (ESP32 OTA)
        let key = parts.headers
            .get("X-Api-Key")
            .and_then(|v| v.to_str().ok());

        match key {
            Some(k) if k == state.api_key => Ok(FirmwareGuard),
            _ => Err((StatusCode::UNAUTHORIZED, "Nicht autorisiert")),
        }
    }
}
