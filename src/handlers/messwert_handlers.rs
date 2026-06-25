use axum::extract::{Query, State};
use axum::Json;

use crate::auth::ApiKeyGuard;
use crate::errors::AppError;
use crate::models::{Messwert, MesswertAntwort, MesswertFilter, NeuerMesswert};
use crate::notifications::pruefe_alarm;
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/messwerte",
    params(
        ("tage" = Option<i64>, Query, description = "Zeitraum in Tagen"),
        ("sensor_id" = Option<i64>, Query, description = "Filter nach Sensor ID")
    ),
    responses((status = 200, description = "Liste der Messwerte", body = Vec<Messwert>)),
    tag = "Messwerte"
)]
pub async fn messwerte_laden(
    State(state): State<AppState>,
    Query(filter): Query<MesswertFilter>,
) -> Result<Json<Vec<Messwert>>, AppError> {
    let tage = filter.tage.unwrap_or(21);

    let messwerte = match filter.sensor_id {
        Some(sensor_id) => sqlx::query_as!(
            Messwert,
            r#"SELECT id as "id!: i64", sensor_id as "sensor_id!: i64", 
            temperatur as "temperatur!: f64", akku_prozent as "akku_prozent!: i64", 
            zeitstempel as "zeitstempel: String"
            FROM messwerte
            WHERE zeitstempel >= datetime('now', '-' || ? || ' days')
            AND sensor_id = ?
            ORDER BY zeitstempel ASC"#,
            tage,
            sensor_id
        )
        .fetch_all(&state.db)
        .await?,

        None => sqlx::query_as!(
            Messwert,
            r#"SELECT id as "id!: i64", sensor_id as "sensor_id!: i64", 
            temperatur as "temperatur!: f64", akku_prozent as "akku_prozent!: i64", 
            zeitstempel as "zeitstempel: String"
            FROM messwerte
            WHERE zeitstempel >= datetime('now', '-' || ? || ' days')
            ORDER BY zeitstempel ASC"#,
            tage
        )
        .fetch_all(&state.db)
        .await?,
    };

    Ok(Json(messwerte))
}

#[utoipa::path(
    post,
    path = "/api/v1/messwerte",
    request_body = NeuerMesswert,
    responses((status = 200, description = "Messwert erstellt", body = Messwert)),
    tag = "Messwerte"
)]
pub async fn messwert_erstellen(
    _guard: ApiKeyGuard,
    State(state): State<AppState>,
    Json(neu): Json<NeuerMesswert>,
) -> Result<Json<MesswertAntwort>, AppError> {
    // Sensor automatisch anlegen falls noch nicht vorhanden
    let sensor_existiert = sqlx::query!(
        "SELECT id FROM sensoren WHERE id = ?",
        neu.sensor_id
    )
    .fetch_optional(&state.db)
    .await?;

    if sensor_existiert.is_none() {
        let sensor_name = format!("Sensor {}", neu.sensor_id);
        sqlx::query!(
            "INSERT INTO sensoren (id, name, standort, sleep_minuten) VALUES (?, ?, ?, 120)",
            neu.sensor_id,
            sensor_name,
            Option::<String>::None
        )
        .execute(&state.db)
        .await?;
    }

    let ergebnis = sqlx::query!(
        "INSERT INTO messwerte (sensor_id, temperatur, akku_prozent) VALUES (?, ?, ?)",
        neu.sensor_id,
        neu.temperatur,
        neu.akku_prozent
    )
    .execute(&state.db)
    .await?;

    pruefe_alarm(
        &state.db,
        neu.sensor_id,
        neu.temperatur,
        neu.akku_prozent,
        &state.ntfy_kanal,
    ).await;

    // Quittierung zurücksetzen - neuer Messwert = frischer Alarm-Zustand
    let _ = sqlx::query!(
        "UPDATE alarm_config SET alarm_quittiert = 0 WHERE sensor_id = ?",
        neu.sensor_id
    )
    .execute(&state.db)
    .await;
    // sleep_minuten für diesen Sensor aus DB lesen
    let sensor = sqlx::query!(
        r#"SELECT sleep_minuten as "sleep_minuten!: i64" FROM sensoren WHERE id = ?"#,
        neu.sensor_id
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(MesswertAntwort {
        id: ergebnis.last_insert_rowid(),
        sleep_minuten: sensor.sleep_minuten,
    }))
}