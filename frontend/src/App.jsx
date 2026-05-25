import { useState, useEffect, useCallback } from 'react'
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts'
import './App.css'

const FARBEN = ['#38bdf8', '#4ade80', '#f87171', '#facc15', '#a78bfa', '#fb923c']

const ZEITRAUM_OPTIONEN = [
  { label: '24h', tage: 1 },
  { label: '7 Tage', tage: 7 },
  { label: '30 Tage', tage: 30 },
  { label: '3 Monate', tage: 90 },
]

const utcZuLokal = (zeitstempel) => {
  if (!zeitstempel) return 'unbekannt'
  const datum = new Date(zeitstempel + 'Z') // 'Z' = UTC
  return datum.toLocaleString('de-DE', {
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit'
  })
}

const utcZuLokalKurz = (zeitstempel) => {
  if (!zeitstempel) return 'unbekannt'
  const datum = new Date(zeitstempel + 'Z')
  return datum.toLocaleString('de-DE', {
    month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit'
  })
}

function App() {
  const [sensoren, setSensoren] = useState([])
  const [messwerte, setMesswerte] = useState([])
  const [alarmConfigs, setAlarmConfigs] = useState([])
  const [alarme, setAlarme] = useState([])
  const [editSensor, setEditSensor] = useState(null)
  const [editAlarm, setEditAlarm] = useState(null)
  const [firmwareInfo, setFirmwareInfo] = useState(null)
  const [neuerSensor, setNeuerSensor] = useState(null)
  const [user, setUser] = useState(null)
  const [meldung, setMeldung] = useState(null)
  const [diagrammTage, setDiagrammTage] = useState(90)
  const [diagrammSensorId, setDiagrammSensorId] = useState(null)

  const [quittierenBestaetigung, setQuittierenBestaetigung] = useState(null)

  const zeigeMeldung = (text, typ = 'erfolg') => {
    setMeldung({ text, typ })
    setTimeout(() => setMeldung(null), 4000)
  }

  const ladeMesswerte = useCallback(async (tage, sensorId) => {
    try {
      let url = `/api/v1/messwerte?tage=${tage}`
      if (sensorId !== null) url += `&sensor_id=${sensorId}`
      const antwort = await fetch(url)
      if (!antwort.ok) throw new Error('Messwerte konnten nicht geladen werden')
      setMesswerte(await antwort.json())
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }
  }, [])

  const ladeDaten = useCallback(async () => {
    try {
      const sensorAntwort = await fetch('/api/v1/sensoren')
      if (!sensorAntwort.ok) throw new Error('Sensoren konnten nicht geladen werden')
      setSensoren(await sensorAntwort.json())
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }

    await ladeMesswerte(diagrammTage, diagrammSensorId)

    try {
      const alarmAntwort = await fetch('/api/v1/alarm-config')
      if (!alarmAntwort.ok) throw new Error('Alarm-Konfiguration konnte nicht geladen werden')
      setAlarmConfigs(await alarmAntwort.json())
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }

    try {
      const historieAntwort = await fetch('/api/v1/alarm-historie')
      if (!historieAntwort.ok) throw new Error('Alarm-Historie konnte nicht geladen werden')
      setAlarme(await historieAntwort.json())
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }

    try {
      const firmwareAntwort = await fetch('/api/v1/firmware/version')
      if (firmwareAntwort.ok) setFirmwareInfo(await firmwareAntwort.json())
    } catch (e) {}

    try {
      const userAntwort = await fetch('/api/v1/auth/me')
      const userData = await userAntwort.json()
      setUser(userData.eingeloggt ? userData : null)
    } catch (e) {}
  }, [diagrammTage, diagrammSensorId, ladeMesswerte])

  useEffect(() => {
    ladeDaten()
    const intervall = setInterval(ladeDaten, 30000)
    return () => clearInterval(intervall)
  }, [ladeDaten])

  // Diagramm-Filter geändert
  useEffect(() => {
    ladeMesswerte(diagrammTage, diagrammSensorId)
  }, [diagrammTage, diagrammSensorId, ladeMesswerte])

  const sensorSpeichern = async () => {
    try {
      const antwort = await fetch(`/api/v1/sensoren/${editSensor.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: editSensor.name, standort: editSensor.standort })
      })
      if (!antwort.ok) throw new Error('Sensor konnte nicht gespeichert werden')
      setEditSensor(null)
      await ladeDaten()
      zeigeMeldung('Sensor erfolgreich gespeichert')
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }
  }

  const alarmSpeichern = async () => {
    try {
      const config = alarmConfigs.find(c => c.sensor_id === editAlarm.sensor_id)
      const method = config ? 'PUT' : 'POST'
      const url = config
        ? `/api/v1/alarm-config/${editAlarm.sensor_id}`
        : '/api/v1/alarm-config'

      const antwort = await fetch(url, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(editAlarm)
      })
      if (!antwort.ok) throw new Error('Alarm-Konfiguration konnte nicht gespeichert werden')
      setEditAlarm(null)
      await ladeDaten()
      zeigeMeldung('Alarm-Schwellenwerte gespeichert')
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }
  }

  const firmwareHochladen = async (e) => {
    const datei = e.target.files[0]
    if (!datei) return

    const formData = new FormData()
    formData.append('firmware', datei)

    try {
      const antwort = await fetch('/api/v1/firmware/upload', {
        method: 'POST',
        body: formData
      })
      if (!antwort.ok) throw new Error('Firmware konnte nicht hochgeladen werden')
      const info = await antwort.json()
      setFirmwareInfo(info)
      zeigeMeldung(`Firmware hochgeladen! Version: ${info.version}, Größe: ${(info.groesse / 1024).toFixed(1)} KB`)
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }
  }

  const sensorHinzufuegen = async () => {
    try {
      const antwort = await fetch('/api/v1/sensoren', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: neuerSensor.name, standort: neuerSensor.standort })
      })
      if (!antwort.ok) throw new Error('Sensor konnte nicht hinzugefügt werden')
      setNeuerSensor(null)
      await ladeDaten()
      zeigeMeldung('Neue Truhe erfolgreich hinzugefügt')
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }
  }

  const sensorLoeschen = async (id, name) => {
    if (!window.confirm(`Truhe "${name}" wirklich löschen? Alle Messwerte und Alarme werden gelöscht!`)) return

    try {
      const antwort = await fetch(`/api/v1/sensoren/${id}`, { method: 'DELETE' })
      if (!antwort.ok) throw new Error('Sensor konnte nicht gelöscht werden')
      await ladeDaten()
      zeigeMeldung(`Truhe "${name}" erfolgreich gelöscht`)
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }
  }

  const sleepAendern = async (sensorId, sleepMinuten) => {
    const wert = parseInt(sleepMinuten)
    if (!wert || wert < 1) {
      zeigeMeldung('Ungültiger Wert — bitte eine Zahl größer als 0 eingeben', 'fehler')
      return
    }
    try {
      const antwort = await fetch(`/api/v1/sensoren/${sensorId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sleep_minuten: wert })
      })
      if (!antwort.ok) throw new Error('Sleep-Intervall konnte nicht gespeichert werden')
      await ladeDaten()
      zeigeMeldung('Sleep-Intervall gespeichert')
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }
  }

  const alarmQuittieren = async (sensorId) => {
    if (quittierenBestaetigung !== sensorId) {
      setQuittierenBestaetigung(sensorId)
      setTimeout(() => setQuittierenBestaetigung(null), 3000)
      return
    }
    setQuittierenBestaetigung(null)
    try {
      const antwort = await fetch(`/api/v1/alarm-config/${sensorId}/quittieren`, {
        method: 'POST'
      })
      if (!antwort.ok) throw new Error('Alarm konnte nicht quittiert werden')
      await ladeDaten()
      zeigeMeldung('Alarm quittiert — keine weiteren Meldungen bis zur nächsten Messung')
    } catch (e) {
      zeigeMeldung(e.message, 'fehler')
    }
  }

const chartDaten = () => {
    const gefilterteSensoren = diagrammSensorId
      ? sensoren.filter(s => s.id === diagrammSensorId)
      : sensoren

    const zeitMap = {}
    messwerte.forEach(m => {
      const zeit = m.zeitstempel || 'unbekannt'
      if (!zeitMap[zeit]) {
        zeitMap[zeit] = { zeit: utcZuLokalKurz(zeit) }
      }
      const sensor = gefilterteSensoren.find(s => s.id === m.sensor_id)
      if (sensor) zeitMap[zeit][sensor.name] = m.temperatur
    })

    return Object.entries(zeitMap).sort((a, b) => a[0].localeCompare(b[0])).map(([, v]) => v)
  }

  const alarmTypIcon = (typ) => {
    switch (typ) {
      case 'temperatur': return '🚨'
      case 'akku': return '🔋'
      case 'offline': return '📡'
      default: return '⚠️'
    }
  }

  const alarmTypKlasse = (typ) => {
    switch (typ) {
      case 'temperatur': return 'alarm-temperatur'
      case 'akku': return 'alarm-akku'
      case 'offline': return 'alarm-offline'
      default: return ''
    }
  }

  const diagrammSensoren = diagrammSensorId
    ? sensoren.filter(s => s.id === diagrammSensorId)
    : sensoren

  const istEingeloggt = user !== null

  return (
    <div className="app">

      {meldung && (
        <div className={`meldung meldung-${meldung.typ}`}>
          {meldung.typ === 'erfolg' ? '✅' : '❌'} {meldung.text}
        </div>
      )}

      <div className="header">
        <h1>Freezer Monitor</h1>
        <p>{sensoren.length} Sensor{sensoren.length !== 1 ? 'en' : ''} verbunden</p>
        <div className="auth-bereich">
          {istEingeloggt ? (
            <>
              <span className="user-email">{user.email}</span>
              <a href="/api/v1/auth/logout" className="auth-btn logout-btn">Abmelden</a>
            </>
          ) : (
            <a href="/api/v1/auth/login" className="auth-btn login-btn">🔐 Mit Google anmelden</a>
          )}
        </div>
      </div>

      <div className="section-header">
        <h2 className="section-title">Sensoren</h2>
        {istEingeloggt && (
          <button className="hinzufuegen-btn" onClick={() => setNeuerSensor({ name: '', standort: '' })}>+ Neue Truhe</button>
        )}
      </div>

      {sensoren.length === 0 ? (
        <div className="keine-daten">Keine Sensoren gefunden</div>
      ) : (
        <div className="sensor-grid">
          {sensoren.map(sensor => {
            const letzterMesswert = messwerte
              .filter(m => m.sensor_id === sensor.id)
              .sort((a, b) => b.id - a.id)[0]

            const alarmConfig = alarmConfigs.find(c => c.sensor_id === sensor.id)

            return (
              <div key={sensor.id} className="sensor-karte">
                <div className="sensor-karte-header">
                  <h3>{sensor.name}</h3>
                  <div className="header-buttons">
                    {sensor.standort && (
                      <span className="standort-badge">{sensor.standort}</span>
                    )}
                    {istEingeloggt && (
                      <>
                        <button className="edit-btn" onClick={() => setEditSensor({ ...sensor })}>✏️</button>
                        <button className="edit-btn" onClick={() => sensorLoeschen(sensor.id, sensor.name)}>🗑️</button>
                      </>
                    )}
                  </div>
                </div>

                {letzterMesswert ? (
                  <>
                    <div className="messwerte-grid">
                      <div className="messwert-box">
                        <div className="messwert-label">Temperatur</div>
                        <div className="messwert-wert temperatur">
                          {letzterMesswert.temperatur}°C
                        </div>
                      </div>
                      <div className="messwert-box">
                        <div className="messwert-label">Akku</div>
                        <div className={`messwert-wert akku ${letzterMesswert.akku_prozent < 20 ? 'niedrig' : ''}`}>
                          {letzterMesswert.akku_prozent}%
                        </div>
                      </div>
                    </div>
                    <div className="letztes-update">
                      Letztes Update: {utcZuLokal(letzterMesswert.zeitstempel)}
                    </div>
                  </>
                ) : (
                  <div className="keine-daten">Noch keine Messwerte</div>
                )}

                <div className="alarm-bereich">
                  {istEingeloggt && (
                    <>
                      <button className="alarm-btn" onClick={() => setEditAlarm(
                        alarmConfig
                          ? { ...alarmConfig }
                          : { sensor_id: sensor.id, temperatur_max: -15.0, akku_min: 20, offline_minuten: 120 }
                      )}>
                        ⚙️ Alarm-Schwellenwerte
                      </button>

                      {alarmConfig && alarmConfig.alarm_quittiert === 0 && (
                        <button
                          className={`alarm-btn quittieren-btn ${quittierenBestaetigung === sensor.id ? 'bestaetigung' : ''}`}
                          onClick={() => alarmQuittieren(sensor.id)}
                        >
                          {quittierenBestaetigung === sensor.id ? '⚠️ Nochmal tippen zum Bestätigen' : '🔕 Alarm quittieren'}
                        </button>
                      )}

                      <div className="sleep-bereich">
                        <span className="sleep-label">Sleep-Intervall</span>
                        <div className="sleep-input-gruppe">
                          <input
                            type="number"
                            min="1"
                            defaultValue={sensor.sleep_minuten}
                            key={sensor.sleep_minuten}
                            onBlur={(e) => sleepAendern(sensor.id, e.target.value)}
                            className="sleep-input"
                          />
                          <span className="sleep-einheit">Min</span>
                        </div>
                      </div>
                    </>
                  )}
                  {alarmConfig && (
                    <div className="alarm-info">
                      Max: {alarmConfig.temperatur_max}°C | Akku min: {alarmConfig.akku_min}% | Offline: {alarmConfig.offline_minuten}min
                      {alarmConfig.alarm_quittiert === 1 && <span className="quittiert-badge"> | 🔕 Quittiert</span>}
                    </div>
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}

      <h2 className="section-title">Temperaturverlauf</h2>
      <div className="chart-container">
        <div className="diagramm-filter">
          <div className="filter-gruppe">
            {ZEITRAUM_OPTIONEN.map(option => (
              <button
                key={option.tage}
                className={`filter-btn ${diagrammTage === option.tage ? 'aktiv' : ''}`}
                onClick={() => setDiagrammTage(option.tage)}
              >
                {option.label}
              </button>
            ))}
          </div>
          <div className="filter-gruppe">
            <button
              className={`filter-btn ${diagrammSensorId === null ? 'aktiv' : ''}`}
              onClick={() => setDiagrammSensorId(null)}
            >
              Alle
            </button>
            {sensoren.map(sensor => (
              <button
                key={sensor.id}
                className={`filter-btn ${diagrammSensorId === sensor.id ? 'aktiv' : ''}`}
                onClick={() => setDiagrammSensorId(sensor.id)}
              >
                {sensor.name}
              </button>
            ))}
          </div>
        </div>

        {messwerte.length === 0 ? (
          <div className="keine-daten">Noch keine Messwerte vorhanden</div>
        ) : (
          <ResponsiveContainer width="100%" height={350}>
            <LineChart data={chartDaten()}>
              <CartesianGrid strokeDasharray="3 3" stroke="#2d3a4f" />
              <XAxis dataKey="zeit" stroke="#64748b" />
              <YAxis stroke="#64748b" unit="°C" />
              <Tooltip
                contentStyle={{ background: '#1e293b', border: '1px solid #2d3a4f', borderRadius: '8px' }}
                labelStyle={{ color: '#94a3b8' }}
              />
              <Legend />
              {diagrammSensoren.map((sensor, index) => (
                <Line
                  key={sensor.id}
                  type="monotone"
                  dataKey={sensor.name}
                  stroke={FARBEN[index % FARBEN.length]}
                  strokeWidth={2}
                  dot={{ r: 4 }}
                  connectNulls
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        )}
      </div>

      <h2 className="section-title">Alarm-Historie</h2>
      <div className="alarm-historie">
        {alarme.length === 0 ? (
          <div className="keine-daten">Keine Alarme vorhanden</div>
        ) : (
          alarme.map(alarm => {
            const sensor = sensoren.find(s => s.id === alarm.sensor_id)
            const sensorName = sensor ? sensor.name : `Sensor ${alarm.sensor_id}`

            return (
              <div key={alarm.id} className={`alarm-eintrag ${alarmTypKlasse(alarm.alarm_typ)}`}>
                <div className="alarm-eintrag-header">
                  <span className="alarm-icon">{alarmTypIcon(alarm.alarm_typ)}</span>
                  <span className="alarm-sensor">{sensorName}</span>
                  <span className="alarm-zeit">{utcZuLokal(alarm.zeitstempel)}</span>
                </div>
                <div className="alarm-nachricht">{alarm.nachricht}</div>
              </div>
            )
          })
        )}
      </div>

      {istEingeloggt && (
        <>
          <h2 className="section-title">Firmware</h2>
          <div className="firmware-bereich">
            <div className="firmware-info">
              <p>Aktuelle Version: {firmwareInfo ? firmwareInfo.version : 'Unbekannt'}</p>
              {firmwareInfo && <p>Größe: {(firmwareInfo.groesse / 1024).toFixed(1)} KB</p>}
            </div>
            <label className="firmware-upload-btn">
              📦 Neue Firmware hochladen
              <input type="file" accept=".bin" onChange={firmwareHochladen} hidden />
            </label>
          </div>
        </>
      )}

      {editSensor && (
        <div className="modal-overlay" onClick={() => setEditSensor(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>Sensor bearbeiten</h3>
            <label>Name</label>
            <input value={editSensor.name} onChange={e => setEditSensor({ ...editSensor, name: e.target.value })} />
            <label>Standort</label>
            <input value={editSensor.standort || ''} onChange={e => setEditSensor({ ...editSensor, standort: e.target.value })} />
            <div className="modal-buttons">
              <button className="btn-speichern" onClick={sensorSpeichern}>Speichern</button>
              <button className="btn-abbrechen" onClick={() => setEditSensor(null)}>Abbrechen</button>
            </div>
          </div>
        </div>
      )}

      {editAlarm && (
        <div className="modal-overlay" onClick={() => setEditAlarm(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>Alarm-Schwellenwerte</h3>
            <label>Max. Temperatur (°C)</label>
            <input type="number" step="0.5" value={editAlarm.temperatur_max} onChange={e => setEditAlarm({ ...editAlarm, temperatur_max: parseFloat(e.target.value) })} />
            <label>Min. Akku (%)</label>
            <input type="number" value={editAlarm.akku_min} onChange={e => setEditAlarm({ ...editAlarm, akku_min: parseInt(e.target.value) })} />
            <label>Offline nach (Minuten)</label>
            <input type="number" value={editAlarm.offline_minuten} onChange={e => setEditAlarm({ ...editAlarm, offline_minuten: parseInt(e.target.value) })} />
            <div className="modal-buttons">
              <button className="btn-speichern" onClick={alarmSpeichern}>Speichern</button>
              <button className="btn-abbrechen" onClick={() => setEditAlarm(null)}>Abbrechen</button>
            </div>
          </div>
        </div>
      )}

      {neuerSensor && (
        <div className="modal-overlay" onClick={() => setNeuerSensor(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>Neue Truhe hinzufügen</h3>
            <label>Name</label>
            <input value={neuerSensor.name} onChange={e => setNeuerSensor({ ...neuerSensor, name: e.target.value })} />
            <label>Standort</label>
            <input value={neuerSensor.standort} onChange={e => setNeuerSensor({ ...neuerSensor, standort: e.target.value })} />
            <div className="modal-buttons">
              <button className="btn-speichern" onClick={sensorHinzufuegen}>Hinzufügen</button>
              <button className="btn-abbrechen" onClick={() => setNeuerSensor(null)}>Abbrechen</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export default App