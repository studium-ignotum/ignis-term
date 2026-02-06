import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "../assets", // Output to relay-server/assets/ for embedding
    emptyOutDir: true, // Clean before build
    assetsDir: "assets", // Nest CSS/JS in assets/ subdirectory
  },
  server: {
    allowedHosts: true, // Allows the server to respond to requests from any host
  },
});
