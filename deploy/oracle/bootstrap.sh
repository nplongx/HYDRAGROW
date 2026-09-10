#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

if [[ ! -f .env ]]; then
  cp .env.example .env
  echo "Created deploy/oracle/.env. Fill it before continuing."
  exit 1
fi

required=(MQTT_USER MQTT_PASSWORD INFLUX_INIT_USERNAME INFLUX_INIT_PASSWORD INFLUX_ORG INFLUX_BUCKET INFLUX_TOKEN DIAGNOSTIC_BACKEND_URL DIAGNOSTIC_API_KEY OPENROUTER_API_KEY)
for key in "${required[@]}"; do
  value="$(grep -E "^${key}=" .env | tail -n1 | cut -d= -f2- || true)"
  if [[ -z "$value" || "$value" == replace_* || "$value" == https://YOUR-BACKEND.onrender.com ]]; then
    echo "Missing or placeholder value for $key in $ROOT_DIR/.env" >&2
    exit 1
  fi
done

mkdir -p mosquitto

# Mosquitto's passwd file is generated on the VM and is intentionally not committed.
# Re-create it from the deployment secret on every bootstrap/update.
docker run --rm -v "$ROOT_DIR/mosquitto:/mosquitto" eclipse-mosquitto:2 \
  sh -c 'mosquitto_passwd -b -c /mosquitto/passwd "$1" "$2"' \
  sh "$(grep '^MQTT_USER=' .env | cut -d= -f2-)" "$(grep '^MQTT_PASSWORD=' .env | cut -d= -f2-)"
chmod 600 mosquitto/passwd

# Oracle Ubuntu images normally include UFW. Keep SSH open, expose MQTT and InfluxDB,
# and keep all other services private to Docker.
if command -v ufw >/dev/null 2>&1; then
  sudo ufw allow 22/tcp
  sudo ufw allow 1883/tcp
  sudo ufw allow 8086/tcp
  sudo ufw --force enable
fi

docker compose pull
docker compose up -d --build

docker compose ps

echo
echo "Oracle stack is up."
echo "MQTT:   tcp://$(hostname -I | awk '{print $1}'):1883"
echo "Influx: http://$(hostname -I | awk '{print $1}'):8086"
echo "Worker logs: docker compose logs -f diagnostic-worker"
