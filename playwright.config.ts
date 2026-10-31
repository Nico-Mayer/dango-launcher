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
  webServer: [
    {
      command: "cargo run --quiet --manifest-path src-tauri/Cargo.toml --example ui_fixture -- 1423",
      url: "http://127.0.0.1:1423/health",
      reuseExistingServer: false,
      // The first run compiles the app crate.
      timeout: 600_000,
    },
    {
      command: "npm run dev -- --port 1422",
      url: "http://127.0.0.1:1422/tests/fixtures/",
      reuseExistingServer: false,
    },
  ],
});
