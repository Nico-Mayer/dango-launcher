import { expect, test } from "@playwright/test";
import { detailTree } from "../fixtures/data";
import type {} from "../fixtures/main";

for (const scene of ["root", "detail"]) {
  test(`development indicator: ${scene} marks a development build`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}&dev=1`);
    await expect(page.getByText("Dango", { exact: true })).toBeVisible();
    await expect(page.locator("main")).toHaveAttribute("data-development", "");
    await expect(page.getByText("dev", { exact: true })).toBeVisible();
    await expect(page.locator("main")).toHaveCSS("width", "720px");
    await expect(page.locator("main")).toHaveCSS("height", "400px");
    await expect(page.locator("main")).toHaveCSS("border-radius", "14px");
    await expect(page.locator("footer")).toHaveCSS("height", "40px");
    expect((await page.locator("footer").boundingBox())?.y).toBe(359);
  });

  test(`development indicator: ${scene} shows nothing in a release build`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    await expect(page.getByText("Dango", { exact: true })).toBeVisible();
    await expect(page.locator("main")).not.toHaveAttribute("data-development");
    await expect(page.getByText("dev", { exact: true })).toBeHidden();
  });
}

test("development indicator: protocol error screen keeps the edge", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=detail&dev=1");
  await expect(page.getByText("Dango", { exact: true })).toBeVisible();
  await page.evaluate((tree) => window.dangoFixture.replace({ ...tree, protocolVersion: 99 }), detailTree);
  await expect(page.getByText("This view needs a newer version of Dango.")).toBeVisible();
  await expect(page.locator("main")).toHaveAttribute("data-development", "");
});

test("development indicator: edge colour follows the token in both palettes", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=root&dev=1");
  await expect(page.getByText("Dango", { exact: true })).toBeVisible();
  const main = page.locator("main");
  const edge = () => main.evaluate((el) => getComputedStyle(el).borderTopColor);
  const dark = await edge();
  await page.screenshot({ path: "test-results/development-indicator-dark.png" });
  await page.evaluate(() => document.documentElement.classList.remove("dark"));
  const light = await edge();
  await page.screenshot({ path: "test-results/development-indicator-light.png" });
  expect(dark).not.toBe(light);
  await page.evaluate(() => document.documentElement.style.setProperty("--dango-dev-edge", "rgb(1, 2, 3)"));
  await expect(main).toHaveCSS("border-top-color", "rgb(1, 2, 3)");
});
