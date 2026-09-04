import path from "node:path"
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import tailwindcss from "@tailwindcss/vite"

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(__dirname, "./src") } },
  server: { host: "0.0.0.0", port: 4790, allowedHosts: [".tail82fec1.ts.net", "localhost"] },
  preview: { host: "0.0.0.0", port: 4790, allowedHosts: [".tail82fec1.ts.net", "localhost"] },
})
