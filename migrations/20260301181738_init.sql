-- Add migration script here
CREATE TABLE IF NOT EXISTS sensoren (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    standort TEXT
);

CREATE TABLE IF NOT EXISTS messwerte (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sensor_id INTEGER NOT NULL,
    temperatur REAL NOT NULL,
    akku_prozent INTEGER NOT NULL,
    zeitstempel DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (sensor_id) REFERENCES sensoren(id)
);