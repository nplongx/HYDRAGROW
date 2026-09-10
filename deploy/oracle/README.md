# HYDRAGROW on Oracle Cloud Always Free

This stack is designed for the current deployment model:

- **Render:** existing HYDRAGROW backend + PostgreSQL.
- **Oracle Cloud Always Free VM:** Mosquitto MQTT, InfluxDB, and `hydragrow-diagnostic-worker`.
- **ESP32:** connects to the Oracle VM MQTT endpoint.
- **Frontend:** can stay local during development or be deployed separately.

Oracle's Always Free compute is suitable for this small deployment, but availability depends on the selected region/capacity. The stack is intentionally Docker-based so it can run on Oracle's Arm Ampere VM.

## 1. Create the Oracle VM

Create an Always Free Ampere A1 VM with Ubuntu. Use the smallest practical shape that is available in your tenancy; the worker + Mosquitto + InfluxDB stack is light enough for a small demo workload.

Reserve a public IPv4 address if the ESP32 devices need to reach MQTT directly. Prefer a DNS name such as `mqtt.example.com` for production.

Install Docker and Compose on the VM using Docker's official Ubuntu instructions.

## 2. Get the repository

```bash
git clone https://github.com/nplongx/HYDRAGROW.git
cd HYDRAGROW/deploy/oracle
cp .env.example .env
```

Edit `.env` and set:

- `DIAGNOSTIC_BACKEND_URL` to the existing Render backend URL.
- `DIAGNOSTIC_API_KEY` to the backend `API_KEY`.
- `OPENROUTER_API_KEY` to the OpenRouter key.
- `MQTT_USER` / `MQTT_PASSWORD` to a long random credential.
- `INFLUX_INIT_*` and `INFLUX_TOKEN` to long random values.

Do **not** commit `.env`.

## 3. Start the Oracle services

```bash
bash bootstrap.sh
```

The script creates the Mosquitto password file locally, enables the required firewall ports, builds the diagnostic worker image, and starts all services.

Check:

```bash
docker compose ps
docker compose logs -f diagnostic-worker
```

## 4. Point Render backend at Oracle

In the Render backend environment, change the infrastructure endpoints to the Oracle VM:

```env
MQTT_HOST=mqtt.example.com
MQTT_PORT=1883
MQTT_USER=<same MQTT_USER as Oracle>
MQTT_PASSWORD=<same MQTT_PASSWORD as Oracle>
MQTT_TLS=false

INFLUX_URL=http://<ORACLE_PUBLIC_IP>:8086
INFLUX_ORG=hydragrow
INFLUX_BUCKET=sensors
INFLUX_TOKEN=<same INFLUX_TOKEN as Oracle>
```

Restart/redeploy the Render backend after changing these values.

### Production security note

Port `1883` is plain MQTT. It is acceptable for an initial lab/demo deployment only when protected by a strong username/password and a restricted network path. For a public production deployment, put MQTT behind TLS on `8883` with a publicly trusted certificate and set:

```env
MQTT_PORT=8883
MQTT_TLS=true
```

The backend already supports `MQTT_TLS=true` using native platform certificates.

## 5. ESP32 configuration

Configure both ESP32 firmware targets to use the Oracle MQTT hostname/IP and the same MQTT credentials. Do not put the credentials into Git. The sensor firmware is the MQTT publisher; the controller uses the same broker for its control/data path.

For a first end-to-end smoke test, the repository simulator can publish telemetry to the same broker before flashing physical hardware.

## 6. InfluxDB

The initial InfluxDB admin token is created from `.env` on first startup. Keep the token stable after initialization. If the persistent volume already exists, changing `DOCKER_INFLUXDB_INIT_*` values will not reinitialize the database.

The backend should use the same organization/bucket/token values as the Oracle InfluxDB service.

## 7. Updating the worker

From the Oracle VM:

```bash
git pull
cd deploy/oracle
bash bootstrap.sh
```

The worker image is rebuilt from the repository root and restarted by Compose.

## 8. Useful commands

```bash
# Worker logs
docker compose logs -f diagnostic-worker

# MQTT logs
docker compose logs -f mosquitto

# InfluxDB logs
docker compose logs -f influxdb

# Restart only the worker
docker compose up -d --build diagnostic-worker

# Stop the stack without deleting data
docker compose down

# Stop and remove persistent data (DESTRUCTIVE)
docker compose down -v
```

## Architecture

```text
ESP32 sensor/controller
        |
        | MQTT
        v
+---------------------------+
| Oracle Always Free VM     |
|                           |
| Mosquitto :1883           |
| InfluxDB   :8086          |
| Diagnostic Worker         |
+-------------+-------------+
              |
              | HTTPS
              v
      Render Backend
              |
        PostgreSQL
```
