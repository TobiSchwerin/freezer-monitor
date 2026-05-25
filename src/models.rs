use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct Sensor {
    pub id: i64,
    pub name: String,
    pub standort: Option<String>,
    pub sleep_minuten: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct NeuerSensor {
    pub name: String,
    pub standort: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct Messwert {
    pub id: i64,
    pub sensor_id: i64,
    pub temperatur: f64,
    pub akku_prozent: i64,
    pub zeitstempel: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct NeuerMesswert {
    pub sensor_id: i64,
    pub temperatur: f64,
    pub akku_prozent: i64,
}

#[derive(Serialize, ToSchema)]
pub struct AlarmConfig {
    pub id: i64,
    pub sensor_id: i64,
    pub temperatur_max: f64,
    pub akku_min: i64,
    pub offline_minuten: i64,
    pub alarm_quittiert: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct NeueAlarmConfig {
    pub sensor_id: i64,
    pub temperatur_max: Option<f64>,
    pub akku_min: Option<i64>,
    pub offline_minuten: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct SensorUpdate {
    pub name: Option<String>,
    pub standort: Option<String>,
    pub sleep_minuten: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct AlarmConfigUpdate {
    pub temperatur_max: Option<f64>,
    pub akku_min: Option<i64>,
    pub offline_minuten: Option<i64>,
    pub alarm_quittiert: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct AlarmHistorie {
    pub id: i64,
    pub sensor_id: i64,
    pub alarm_typ: String,
    pub nachricht: String,
    pub zeitstempel: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct FirmwareInfo {
    pub version: String,
    pub groesse: u64,
}

#[derive(Deserialize, ToSchema)]
pub struct MesswertFilter {
    pub tage: Option<i64>,
    pub sensor_id: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct MesswertAntwort {
    pub id: i64,
    pub sleep_minuten: i64,
}