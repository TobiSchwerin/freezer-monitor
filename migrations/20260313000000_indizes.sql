-- Index für Messwerte: Zeitraum-Filter (häufigste Abfrage)
CREATE INDEX IF NOT EXISTS idx_messwerte_zeitstempel 
    ON messwerte(zeitstempel);

-- Index für Messwerte: Sensor + Zeitraum kombiniert
CREATE INDEX IF NOT EXISTS idx_messwerte_sensor_zeitstempel 
    ON messwerte(sensor_id, zeitstempel);

-- Index für Alarm-Historie: Zeitraum-Filter
CREATE INDEX IF NOT EXISTS idx_alarm_historie_zeitstempel 
    ON alarm_historie(zeitstempel);