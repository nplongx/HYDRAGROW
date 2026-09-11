import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from 'path';

const tauriDevHost = process.env.TAURI_DEV_HOST;
const isTauriDevRuntime = Boolean(tauriDevHost);
const isTauriTarget = process.env.TAURI_ENV_PLATFORM !== undefined;

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const enableSourceMap = mode !== "production";

  return {
    // 👇 SỬA Ở ĐÂY: Thêm VitePWA vào bên trong mảng plugins
    plugins: [
      react(), 
      tailwindcss(),
    ],

    define: {
      __TAURI_BUILD__: JSON.stringify(isTauriTarget),
    },

    // Prevent Vite from obscuring Rust errors in Tauri workflows.
    clearScreen: false,

    // Output and source maps tuned for web/desktop targets.
    build: {
      outDir: isTauriTarget ? "dist-tauri" : "dist",
      sourcemap: isTauriTarget ? true : enableSourceMap,
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (id.includes('node_modules')) {
              if (id.includes('@xyflow')) return 'vendor-flow';
              if (id.includes('html5-qrcode') || id.includes('react-qr-code')) return 'vendor-qr';
              if (id.includes('firebase')) return 'vendor-firebase';
              if (id.includes('lucide-react')) return 'vendor-icons';
              if (id.includes('@tanstack') || id.includes('react-router-dom') || id.includes('zustand')) return 'vendor-framework';
            }
          },
        },
      },
    },

    server: {
      // Tauri expects a fixed port, fail if that port is not available.
      port: 1420,
      strictPort: true,
      host: true,
      proxy: { '/api': { target: 'http://localhost:8080', changeOrigin: true, ws: true } },
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

    test: {
      environment: 'jsdom',
      globals: true
    }
  };
});
