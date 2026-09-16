import { expect, test } from "@playwright/test";
import { fixtureNames } from "../fixtures/data";
import type {} from "../fixtures/main";

for (const name of fixtureNames) {
  test(`fixture ${name} loads without native IPC`, async ({ page }, testInfo) => {
    const errors: string[] = [];
    const externalRequests: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("request", (request) => {
      if (new URL(request.url()).hostname !== "127.0.0.1") externalRequests.push(request.url());
    });
    await page.goto(`/tests/fixtures/?scene=${name}`);
    await expect(page.getByText("Dango", { exact: true })).toBeVisible();
    if (["root", "list", "refreshing-list", "failure", "long-actions"].includes(name))
      await expect(page.getByRole("option")).toHaveCount(18);
    if (name === "empty-list") await expect(page.getByText("Nothing to show")).toBeVisible();
    if (name === "loading-list") await expect(page.getByRole("listbox").getByText("Loading…")).toBeVisible();
    if (name === "detail") await expect(page.getByText("Working…")).toBeVisible();
    if (name === "form") await expect(page.getByLabel("Name", { exact: true })).toHaveValue("Morning note");
    if (name === "template") await expect(page.getByLabel("Template")).toHaveValue("Hello {{name}}");
    if (name === "failure") await expect(page.getByText("Couldn't paste. Try again.")).toBeVisible();
    if (name === "long-actions") {
      await page.keyboard.press("Control+k");
      await expect(page.locator("[data-popover-content]").getByRole("option")).toHaveCount(24);
    }
    await page.screenshot({ path: testInfo.outputPath(`${name}.png`) });
    if (name === "detail") {
      await page.evaluate(() => window.dangoFixture.stream(3, 20));
      await expect(page.getByText(/Streaming text 3/)).toBeVisible();
    }
    expect(errors).toEqual([]);
    expect(externalRequests).toEqual([]);
    const commands = await page.evaluate(() => window.dangoFixture.calls.map((call) => call.command));
    expect(commands).toContain("warmup_done");
    expect(commands).toContain("search");
  });
}
