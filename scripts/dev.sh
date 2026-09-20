#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
COMPOSE=(docker compose -f docker-compose.yml -f docker-compose.dev.yml)
PIDS=()
cleanup() {
  trap - INT TERM EXIT
  for pid in "${PIDS[@]:-}"; do
    kill "$pid" 2>/dev/null || true
  done
  wait 2>/dev/null || true
}
trap cleanup INT TERM EXIT
need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: missing command: $1" >&2
    exit 1
  }
}
need_cmd docker
need_cmd cargo
need_cmd npm
need_cmd curl
need_cmd mosquitto_pub
echo "==> Starting dev infrastructure"
"${COMPOSE[@]}" up -d postgres mosquitto influxdb
echo "==> Waiting for infrastructure"
for _ in {1..30}; do
  if docker exec hydragrow-postgres-1 pg_isready -U hydragrow -d hydragrow >/dev/null 2>&1 &&
     curl -fsS http://127.0.0.1:8086/health >/dev/null 2>&1 &&
     mosquitto_pub -h 127.0.0.1 -p 1883 -t hydragrow/dev/health -m ok >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
echo "==> Starting backend"
(
  cd "$ROOT/hydragrow-backend"
  ENVIRONMENT=development \
  DEV_AUTH_TOKEN="${DEV_AUTH_TOKEN:-local-dev-auth-token}" \
  DEV_AUTH_USER_ID="${DEV_AUTH_USER_ID:-1}" \
  PRIVILEGED_CONTROL_SECRET="${PRIVILEGED_CONTROL_SECRET:-local-dev-control-secret}" \
  API_KEY="${API_KEY:-local-dev-api-key}" \
  ALLOWED_ORIGINS="${ALLOWED_ORIGINS:-http://localhost:1420,http://127.0.0.1:1420,http://localhost:1421,http://127.0.0.1:1421}" \
  DATABASE_URL="postgres://hydragrow:dev_only_password@127.0.0.1:55432/hydragrow" \
  MQTT_HOST=127.0.0.1 \
  MQTT_PORT=1883 \
  INFLUX_URL=http://127.0.0.1:8086 \
  INFLUX_ORG=hydragrow \
  INFLUX_BUCKET=sensors \
  INFLUX_TOKEN=dev_only_token \
  cargo run
) &
PIDS+=("$!")
echo "==> Starting frontend"
(
  cd "$ROOT/hydragrow-frontend"
  npm run dev:web -- --host 127.0.0.1
) &
PIDS+=("$!")
echo
echo "Dev environment is running."
echo "  Frontend: http://127.0.0.1:1420"
echo "  Backend:  http://localhost:8080"
echo "  MQTT:     mqtt://localhost:1883"
echo "  InfluxDB: http://localhost:8086"
echo "  Postgres: localhost:55432"
echo
echo "Mock auth token: ${DEV_AUTH_TOKEN:-local-dev-auth-token}"
echo "Press Ctrl+C to stop backend/frontend. Docker infra stays running."
wait
