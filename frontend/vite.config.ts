import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const proxy = { "/api": "http://127.0.0.1:3000" };

export default defineConfig({
  plugins: [react()],
  server: { host: "127.0.0.1", proxy },
  preview: { host: "127.0.0.1", proxy },
});
