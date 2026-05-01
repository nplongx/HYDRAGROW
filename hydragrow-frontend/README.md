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
