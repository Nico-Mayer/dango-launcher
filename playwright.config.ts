import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/browser",
  outputDir: "./test-results",
  fullyParallel: true,
  use: {
    baseURL: "http://127.0.0.1:1422",
    viewport: { width: 720, height: 400 },
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
  webServer: {
    command: "npm run dev -- --port 1422",
    url: "http://127.0.0.1:1422/tests/fixtures/",
    reuseExistingServer: false,
  },
});
