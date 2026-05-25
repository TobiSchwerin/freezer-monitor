use axum::{extract::{Multipart, State}, Json};
use axum::response::IntoResponse;
use axum::http::header;

use crate::auth::{AuthGuard, FirmwareGuard};
use crate::errors::AppError;
use crate::models::FirmwareInfo;
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/firmware/version",
    responses((status = 200, description = "Aktuelle Firmware-Version", body = FirmwareInfo)),
    tag = "Firmware"
)]
pub async fn firmware_version(State(state): State<AppState>) -> Result<Json<FirmwareInfo>, AppError> {
    let pfad = std::path::Path::new("firmware/firmware.bin");

    if !pfad.exists() {
        return Err(AppError::NichtGefunden(String::from("Keine Firmware vorhanden")));
    }

    let metadata = std::fs::metadata(pfad)
        .map_err(|_| AppError::NichtGefunden(String::from("Firmware-Datei nicht lesbar")))?;

    let version = state.firmware_version.read().await.clone();

    Ok(Json(FirmwareInfo {
        version,
        groesse: metadata.len(),
    }))
}

pub async fn firmware_download(_guard: FirmwareGuard) -> impl IntoResponse {
    let pfad = "firmware/firmware.bin";

    match std::fs::read(pfad) {
        Ok(daten) => (
            [
                (header::CONTENT_TYPE, "application/octet-stream"),
                (header::CONTENT_DISPOSITION, "attachment; filename=\"firmware.bin\""),
            ],
            daten,
        ).into_response(),
        Err(_) => (
            axum::http::StatusCode::NOT_FOUND,
            "Keine Firmware vorhanden",
        ).into_response(),
    }
}

fn version_aus_dateiname(dateiname: &str) -> Option<String> {
    // firmware_1.0.1.bin -> "1.0.1"
    let ohne_extension = dateiname.trim_end_matches(".bin");
    let teile: Vec<&str> = ohne_extension.splitn(2, '_').collect();
    if teile.len() == 2 {
        let version = teile[1].to_string();
        // Prüfen ob es ein gültiges Versionsformat ist (x.y.z)
        let segmente: Vec<&str> = version.split('.').collect();
        if segmente.len() >= 2 && segmente.iter().all(|s| s.parse::<u32>().is_ok()) {
            return Some(version);
        }
    }
    None
}

pub async fn firmware_upload(
    _guard: AuthGuard,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<FirmwareInfo>, AppError> {
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("").to_string();

        if name == "firmware" {
            // Dateiname auslesen
            let dateiname = field
                .file_name()
                .unwrap_or("firmware.bin")
                .to_string();

            let daten = field.bytes().await?;

            std::fs::create_dir_all("firmware")
                .map_err(|_| AppError::NichtGefunden(String::from("Firmware-Ordner konnte nicht erstellt werden")))?;

            std::fs::write("firmware/firmware.bin", &daten)
                .map_err(|_| AppError::NichtGefunden(String::from("Firmware konnte nicht gespeichert werden")))?;

            // Version aus Dateiname extrahieren
            let neue_version = version_aus_dateiname(&dateiname)
                .unwrap_or_else(|| state.firmware_version.blocking_read().clone());

            // Version in Datei speichern (bleibt nach Neustart erhalten)
            let _ = std::fs::write("firmware/version.txt", &neue_version);

            // Version im AppState aktualisieren
            *state.firmware_version.write().await = neue_version.clone();

            println!("Firmware hochgeladen: {} ({} bytes)", neue_version, daten.len());

            return Ok(Json(FirmwareInfo {
                version: neue_version,
                groesse: daten.len() as u64,
            }));
        }
    }

    Err(AppError::NichtGefunden(String::from("Keine Firmware-Datei im Upload gefunden")))
}