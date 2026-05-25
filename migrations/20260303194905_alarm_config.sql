-- Add migration script here
CREATE TABLE IF NOT EXISTS alarm_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sensor_id INTEGER NOT NULL UNIQUE,
    temperatur_max REAL NOT NULL DEFAULT -15.0,
    akku_min INTEGER NOT NULL DEFAULT 20,
    offline_minuten INTEGER NOT NULL DEFAULT 120,
    FOREIGN KEY (sensor_id) REFERENCES sensoren(id)
);