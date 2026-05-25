use axum::{extract::{Path, State}, Json};

use crate::auth::AuthGuard;
use crate::errors::AppError;
use crate::models::{AlarmConfig, AlarmConfigUpdate, AlarmHistorie, NeueAlarmConfig};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/alarm-config",
    responses((status = 200, description = "Liste aller Alarm-Konfigurationen", body = Vec<AlarmConfig>)),
    tag = "Alarm"
)]
pub async fn alarm_config_laden(State(state): State<AppState>) -> Result<Json<Vec<AlarmConfig>>, AppError> {
    let configs = sqlx::query_as!(
        AlarmConfig,
        r#"SELECT id, sensor_id, temperatur_max, akku_min, offline_minuten, alarm_quittiert as "alarm_quittiert!: i64" FROM alarm_config"#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(configs))
}

#[utoipa::path(
    post,
    path = "/api/v1/alarm-config",
    request_body = NeueAlarmConfig,
    responses((status = 200, description = "Alarm-Konfiguration erstellt", body = AlarmConfig)),
    tag = "Alarm"
)]
pub async fn alarm_config_erstellen(
    _guard: AuthGuard,
    State(state): State<AppState>,
    Json(neu): Json<NeueAlarmConfig>,
) -> Result<Json<AlarmConfig>, AppError> {
    let temp_max = neu.temperatur_max.unwrap_or(-15.0);
    let akku = neu.akku_min.unwrap_or(20);
    let offline = neu.offline_minuten.unwrap_or(270);

    let ergebnis = sqlx::query!(
        "INSERT INTO alarm_config (sensor_id, temperatur_max, akku_min, offline_minuten) VALUES (?, ?, ?, ?)",
        neu.sensor_id,
        temp_max,
        akku,
        offline
    )
    .execute(&state.db)
    .await?;

    Ok(Json(AlarmConfig {
        id: ergebnis.last_insert_rowid(),
        sensor_id: neu.sensor_id,
        temperatur_max: temp_max,
        akku_min: akku,
        offline_minuten: offline,
        alarm_quittiert: 0,
    }))
}

#[utoipa::path(
    put,
    path = "/api/v1/alarm-config/{sensor_id}",
    params(("sensor_id" = i64, Path, description = "Sensor ID")),
    request_body = AlarmConfigUpdate,
    responses((status = 200, description = "Alarm-Konfiguration aktualisiert", body = AlarmConfig)),
    tag = "Alarm"
)]
pub async fn alarm_config_aktualisieren(
    _guard: AuthGuard,
    State(state): State<AppState>,
    Path(sensor_id): Path<i64>,
    Json(update): Json<AlarmConfigUpdate>,
) -> Result<Json<AlarmConfig>, AppError> {
    let aktuell = sqlx::query_as!(
        AlarmConfig,
        r#"SELECT id as "id!: i64", sensor_id as "sensor_id!: i64", temperatur_max as "temperatur_max!: f64", akku_min as "akku_min!: i64", offline_minuten as "offline_minuten!: i64", alarm_quittiert as "alarm_quittiert!: i64" FROM alarm_config WHERE sensor_id = ?"#,
        sensor_id
    )
    .fetch_one(&state.db)
    .await?;

    let temp = update.temperatur_max.unwrap_or(aktuell.temperatur_max);
    let akku = update.akku_min.unwrap_or(aktuell.akku_min);
    let offline = update.offline_minuten.unwrap_or(aktuell.offline_minuten);
    let quittiert = update.alarm_quittiert.unwrap_or(aktuell.alarm_quittiert);

    sqlx::query!(
        "UPDATE alarm_config SET temperatur_max = ?, akku_min = ?, offline_minuten = ?, alarm_quittiert = ? WHERE sensor_id = ?",
        temp,
        akku,
        offline,
        quittiert,
        sensor_id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(AlarmConfig {
        id: aktuell.id,
        sensor_id,
        temperatur_max: temp,
        akku_min: akku,
        offline_minuten: offline,
        alarm_quittiert: quittiert,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/alarm-historie",
    responses((status = 200, description = "Liste der letzten 50 Alarme", body = Vec<AlarmHistorie>)),
    tag = "Alarm"
)]
pub async fn alarm_historie_laden(State(state): State<AppState>) -> Result<Json<Vec<AlarmHistorie>>, AppError> {
    let alarme = sqlx::query_as!(
        AlarmHistorie,
        r#"SELECT id as "id!: i64", sensor_id as "sensor_id!: i64", 
        alarm_typ as "alarm_typ!: String", nachricht as "nachricht!: String", 
        zeitstempel as "zeitstempel: String" 
        FROM alarm_historie ORDER BY zeitstempel DESC LIMIT 10"#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(alarme))
}

pub async fn alarm_quittieren(
    _guard: AuthGuard,
    State(state): State<AppState>,
    Path(sensor_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query!(
        "UPDATE alarm_config SET alarm_quittiert = 1 WHERE sensor_id = ?",
        sensor_id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"quittiert": true})))
}