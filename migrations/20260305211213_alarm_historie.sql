-- Add migration script here
CREATE TABLE IF NOT EXISTS alarm_historie (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sensor_id INTEGER NOT NULL,
    alarm_typ TEXT NOT NULL,
    nachricht TEXT NOT NULL,
    zeitstempel DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (sensor_id) REFERENCES sensoren(id)
);