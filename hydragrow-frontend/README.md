# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Scripts

- `npm run dev:web`: Run Vite dev server for the web target.
- `npm run build:web`: Build SPA artifacts for web deployment.
- `npm run preview:web`: Preview the web build locally.
- `npm run dev:tauri`: Run Tauri desktop app in development mode.
- `npm run build:tauri`: Build Tauri desktop bundles.

## Deploying the SPA Build

Build the web bundle first:

```bash
npm run build:web
```

Deploy the generated `dist/` directory to your host and make sure all application routes are rewritten to `index.html` for React Router.

### Netlify

Use a `_redirects` file in your publish directory with:

```txt
/* /index.html 200
```

### Vercel

Add a `vercel.json` file:

```json
{
  "rewrites": [{ "source": "/(.*)", "destination": "/index.html" }]
}
```

### Nginx

Inside your site block:

```nginx
location / {
  try_files $uri $uri/ /index.html;
}
```

## Runtime secrets and deployment configuration

Do not commit production credentials to this repository. Values that have been exposed in git history (WiFi password, MQTT password, backend API key, Firebase/FCM/VAPID keys) must be rotated in the external systems before redeploying.

### Web/Tauri frontend

- `public/config.json` is committed with placeholders only. For a deployed web build, replace it at deploy time (or serve an environment-specific file from your hosting platform) with:
  - `backend_url`: HTTPS URL for the Hydragrow backend.
  - `api_key`: backend API key generated outside source control.
  - `device_id`: the device identifier to control.
- Copy `.env.example` to an untracked `.env.local` for Firebase browser messaging values (`VITE_FIREBASE_*` and `VITE_FIREBASE_VAPID_KEY`).
- For Android/Tauri Firebase messaging, generate `google-services.json` from Firebase Console during the build/deploy process. The checked-in file contains placeholders only.

### ESP32-C3 sensor node

Copy `ESP32-C3-SENSOR-NODE/include/secrets.example.h` to `ESP32-C3-SENSOR-NODE/include/secrets.h` and fill in WiFi, MQTT, and device-specific values locally. `secrets.h` is ignored by git.

### ESP32-C3 controller node

Provide controller credentials via compile-time environment variables instead of editing Rust source:

```bash
export HYDRAGROW_WIFI_SSID="your-wifi-ssid"
export HYDRAGROW_WIFI_PASSWORD="your-wifi-password"
export HYDRAGROW_MQTT_URL="mqtt://your-mqtt-host:1883"
export HYDRAGROW_MQTT_USER="your-mqtt-user"
export HYDRAGROW_MQTT_PASSWORD="your-mqtt-password"
export HYDRAGROW_DEVICE_ID="your-device-id"
```
