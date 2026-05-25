use axum::{extract::{Path, State}, Json};

use crate::auth::AuthGuard;
use crate::errors::AppError;
use crate::models::{NeuerSensor, Sensor, SensorUpdate};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/sensoren",
    responses((status = 200, description = "Liste aller Sensoren", body = Vec<Sensor>)),
    tag = "Sensoren"
)]
pub async fn sensoren_laden(State(state): State<AppState>) -> Result<Json<Vec<Sensor>>, AppError> {
    let sensoren = sqlx::query_as!(
        Sensor,
        r#"SELECT id as "id!: i64", name, standort, sleep_minuten as "sleep_minuten!: i64" FROM sensoren"#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(sensoren))
}

#[utoipa::path(
    post,
    path = "/api/v1/sensoren",
    request_body = NeuerSensor,
    responses((status = 200, description = "Sensor erstellt", body = Sensor)),
    tag = "Sensoren"
)]
pub async fn sensor_erstellen(
    _guard: AuthGuard,
    State(state): State<AppState>,
    Json(neu): Json<NeuerSensor>,
) -> Result<Json<Sensor>, AppError> {
    let ergebnis = sqlx::query!(
        "INSERT INTO sensoren (name, standort, sleep_minuten) VALUES (?, ?, 120)",
        neu.name,
        neu.standort
    )
    .execute(&state.db)
    .await?;

    Ok(Json(Sensor {
        id: ergebnis.last_insert_rowid(),
        name: neu.name,
        standort: neu.standort,
        sleep_minuten: 120,  // <- NEU
    }))
}

#[utoipa::path(
    put,
    path = "/api/v1/sensoren/{id}",
    params(("id" = i64, Path, description = "Sensor ID")),
    request_body = SensorUpdate,
    responses((status = 200, description = "Sensor aktualisiert", body = Sensor)),
    tag = "Sensoren"
)]
pub async fn sensor_aktualisieren(
    _guard: AuthGuard,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(update): Json<SensorUpdate>,
) -> Result<Json<Sensor>, AppError> {
    let aktuell = sqlx::query_as!(
        Sensor,
        r#"SELECT id as "id!: i64", name, standort, sleep_minuten as "sleep_minuten!: i64" FROM sensoren WHERE id = ?"#,
        id
    )
    .fetch_one(&state.db)
    .await?;

    let neuer_name = update.name.unwrap_or(aktuell.name);
    let neuer_standort = update.standort.or(aktuell.standort);
    let neues_sleep = update.sleep_minuten.unwrap_or(aktuell.sleep_minuten);  // <- NEU

    sqlx::query!(
        "UPDATE sensoren SET name = ?, standort = ?, sleep_minuten = ? WHERE id = ?",
        neuer_name,
        neuer_standort,
        neues_sleep,  // <- NEU
        id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(Sensor {
        id,
        name: neuer_name,
        standort: neuer_standort,
        sleep_minuten: neues_sleep,  // <- NEU
    }))
}

#[utoipa::path(
    delete,
    path = "/api/v1/sensoren/{id}",
    params(("id" = i64, Path, description = "Sensor ID")),
    responses((status = 200, description = "Sensor gelöscht")),
    tag = "Sensoren"
)]
pub async fn sensor_loeschen(
    _guard: AuthGuard,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query!("DELETE FROM messwerte WHERE sensor_id = ?", id)
        .execute(&state.db)
        .await?;

    sqlx::query!("DELETE FROM alarm_config WHERE sensor_id = ?", id)
        .execute(&state.db)
        .await?;

    sqlx::query!("DELETE FROM alarm_historie WHERE sensor_id = ?", id)
        .execute(&state.db)
        .await?;

    sqlx::query!("DELETE FROM sensoren WHERE id = ?", id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"geloescht": true})))
}
