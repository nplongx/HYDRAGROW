import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from 'path';
import { VitePWA } from 'vite-plugin-pwa';

const tauriDevHost = process.env.TAURI_DEV_HOST;
const isTauriDevRuntime = Boolean(tauriDevHost);
const isTauriTarget = process.env.TAURI_ENV_PLATFORM !== undefined;

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const enableSourceMap = mode !== "production";

  return {
    plugins: [react(), tailwindcss()],

    define: {
      __TAURI_BUILD__: JSON.stringify(isTauriTarget),
    },

    // Prevent Vite from obscuring Rust errors in Tauri workflows.
    clearScreen: false,

    // Output and source maps tuned for web/desktop targets.
    build: {
      outDir: isTauriTarget ? "dist-tauri" : "dist",
      sourcemap: isTauriTarget ? true : enableSourceMap,
    },

    server: {
      // Tauri expects a fixed port, fail if that port is not available.
      port: 1420,
      strictPort: true,
      host: true,
      hmr: isTauriDevRuntime
        ? {
          protocol: "ws",
          host: tauriDevHost,
          port: 1421,
        }
        : undefined,
      watch: {
        // Ignore Tauri Rust sources when running Vite watcher.
        ignored: ["**/src-tauri/**"],
      },
    },

    resolve: {
      alias: {
        '@gleam': path.resolve(__dirname, './gleam_core/build/dev/javascript/gleam_core')
      }
    },

    VitePWA({
      strategies: 'injectManifest',
      srcDir: 'src', // Trỏ thư mục chứa SW vào src
      filename: 'firebase-messaging-sw.js', // Tên file SW của bạn
      injectManifest: {
        injectionPoint: undefined // Nếu bạn không dùng tính năng cache offline, chỉ dùng push notification
      }
    })
  }});
