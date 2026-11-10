import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  use: {
    baseURL: process.env.SKILLBRIDGE_TEST_URL ?? "http://127.0.0.1:8080",
    ...(process.env.SKILLBRIDGE_CHROME_CHANNEL ? { channel: process.env.SKILLBRIDGE_CHROME_CHANNEL } : {}),
  },
  workers: 1,
  timeout: 45_000,
});
