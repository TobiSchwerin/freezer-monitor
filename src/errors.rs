use axum::http::StatusCode;
use axum::response::IntoResponse;

pub enum AppError {
    DatenbankFehler(sqlx::Error),
    NichtGefunden(String),
    UploadFehler(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::DatenbankFehler(e) => {
                eprintln!("Datenbankfehler: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Datenbankfehler: {}", e),
                ).into_response()
            }
            AppError::NichtGefunden(msg) => {
                (
                    StatusCode::NOT_FOUND,
                    msg,
                ).into_response()
            }
            AppError::UploadFehler(msg) => {
                (
                    StatusCode::BAD_REQUEST,
                    msg,
                ).into_response()
            }
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::DatenbankFehler(e)
    }
}

impl From<axum::extract::multipart::MultipartError> for AppError {
    fn from(e: axum::extract::multipart::MultipartError) -> Self {
        AppError::UploadFehler(e.to_string())
    }
}