use sqlx::SqlitePool;

pub async fn pruefe_alarm(
    db: &SqlitePool,
    sensor_id: i64,
    temperatur: f64,
    akku_prozent: i64,
    ntfy_kanal: &str,
) {
    let config = sqlx::query!(
        "SELECT temperatur_max, akku_min, alarm_quittiert FROM alarm_config WHERE sensor_id = ?",
        sensor_id
    )
    .fetch_optional(db)
    .await;

    let config = match config {
        Ok(Some(c)) => c,
        _ => return,
    };

    // Wenn quittiert, keine Alarme senden
    if config.alarm_quittiert == 1 {
        return;
    }

    let name = hole_sensorname(db, sensor_id).await;

    if temperatur > config.temperatur_max {
        let titel = format!("🚨 TEMPERATUR-ALARM: {}", name);
        let nachricht = format!("Temperatur {:.1}°C überschreitet Grenzwert {:.1}°C!", temperatur, config.temperatur_max);
        speichere_alarm(db, sensor_id, "temperatur", &nachricht).await;
        sende_ntfy(ntfy_kanal, &titel, &nachricht).await;
    }

    if akku_prozent < config.akku_min {
        let titel = format!("🔋 AKKU-WARNUNG: {}", name);
        let nachricht = format!("Akkustand {}% unter Minimum {}%!", akku_prozent, config.akku_min);
        speichere_alarm(db, sensor_id, "akku", &nachricht).await;
        sende_ntfy(ntfy_kanal, &titel, &nachricht).await;
    }
}

pub async fn starte_offline_pruefung(db: SqlitePool, ntfy_kanal: String) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1800)).await;

        println!("Prüfe Offline-Status...");

        let configs = sqlx::query!(
            "SELECT sensor_id, offline_minuten, alarm_quittiert FROM alarm_config"
        )
        .fetch_all(&db)
        .await;

        let configs = match configs {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Fehler beim Laden der Alarm-Config: {}", e);
                continue;
            }
        };

        for config in configs {
            if config.alarm_quittiert == 1 {
                continue;
            }
            let letzter = sqlx::query!(
                r#"SELECT zeitstempel as "zeitstempel: String" FROM messwerte WHERE sensor_id = ? ORDER BY zeitstempel DESC LIMIT 1"#,
                config.sensor_id
            )
            .fetch_optional(&db)
            .await;

            let zeitstempel = match letzter {
                Ok(Some(row)) => row.zeitstempel,
                _ => continue,
            };

            let name = hole_sensorname(&db, config.sensor_id).await;

            if let Some(ts) = zeitstempel {
                let alter_minuten = berechne_alter_minuten(&ts);
                if alter_minuten > config.offline_minuten {
                    let titel = format!("📡 OFFLINE-ALARM: {}", name);
                    let nachricht = format!("{} hat seit {} Minuten keine Daten gesendet!", name, alter_minuten);
                    speichere_alarm(&db, config.sensor_id, "offline", &nachricht).await;
                    sende_ntfy(&ntfy_kanal, &titel, &nachricht).await;
                }
            }
        }
    }
}

async fn hole_sensorname(db: &SqlitePool, sensor_id: i64) -> String {
    sqlx::query!("SELECT name FROM sensoren WHERE id = ?", sensor_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|r| r.name)
        .unwrap_or(format!("Sensor {}", sensor_id))
}

async fn speichere_alarm(db: &SqlitePool, sensor_id: i64, alarm_typ: &str, nachricht: &str) {
    let _ = sqlx::query!(
        "INSERT INTO alarm_historie (sensor_id, alarm_typ, nachricht) VALUES (?, ?, ?)",
        sensor_id,
        alarm_typ,
        nachricht
    )
    .execute(db)
    .await;
}

async fn sende_ntfy(kanal: &str, titel: &str, nachricht: &str) {
    let url = format!("https://ntfy.sh/{}", kanal);

    let ergebnis = reqwest::Client::new()
        .post(&url)
        .header("Title", titel)
        .body(nachricht.to_string())
        .send()
        .await;

    match ergebnis {
        Ok(_) => println!("Alarm gesendet: {}", titel),
        Err(e) => eprintln!("Fehler beim Senden: {}", e),
    }
}

fn berechne_alter_minuten(zeitstempel: &str) -> i64 {
    let now = chrono::Utc::now().naive_utc();
    let parsed = chrono::NaiveDateTime::parse_from_str(zeitstempel, "%Y-%m-%d %H:%M:%S");
    match parsed {
        Ok(ts) => (now - ts).num_minutes(),
        Err(_) => 0,
    }
}

pub async fn starte_daten_bereinigung(db: sqlx::SqlitePool) {
    loop {
        // Wöchentlich prüfen (Retention: 3 Wochen)
        tokio::time::sleep(tokio::time::Duration::from_secs(7 * 24 * 60 * 60)).await;

        println!("Starte wöchentliche Datenbereinigung...");

        match sqlx::query(
            "DELETE FROM messwerte WHERE zeitstempel < datetime('now', '-21 days')"
        )
        .execute(&db)
        .await {
            Ok(r) => println!("Bereinigung: {} Messwerte gelöscht", r.rows_affected()),
            Err(e) => eprintln!("Fehler bei Messwert-Bereinigung: {}", e),
        }

        match sqlx::query(
            "DELETE FROM alarm_historie WHERE zeitstempel < datetime('now', '-21 days')"
        )
        .execute(&db)
        .await {
            Ok(r) => println!("Bereinigung: {} Alarm-Einträge gelöscht", r.rows_affected()),
            Err(e) => eprintln!("Fehler bei Alarm-Bereinigung: {}", e),
        }

        println!("Datenbereinigung abgeschlossen.");
    }
}