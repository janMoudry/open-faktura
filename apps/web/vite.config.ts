import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { developmentApiOrigin } from "./dev/api-proxy.mjs";
export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    {
      name: "exclude-development-sandbox",
      apply: "build",
      generateBundle() {
        const forbidden =
          /(?:sandbox\.tsx|invoice-demo|DesignSystemPage|\/InvoicesPage\.tsx(?:$|\?)|(?:Atom|Chart|Foundation|Selection)Gallery)/;
        for (const id of this.getModuleIds()) {
          if (forbidden.test(id))
            this.error(`Development sandbox leaked into product: ${id}`);
        }
      },
    },
  ],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:3001",
        configure(proxy) {
          proxy.on("proxyReq", (outgoing, incoming) => {
            const origin = developmentApiOrigin(
              incoming.headers.origin,
              incoming.headers.host,
            );
            if (origin) outgoing.setHeader("Origin", origin);
          });
        },
      },
    },
  },
  build: { target: "es2022" },
});
