import { expect, test, type Page } from "@playwright/test";
import type {} from "../fixtures/main";

const searchesFor = (page: Page, query: string) => page.evaluate((wanted) =>
  window.dangoFixture.calls.filter((call) =>
    call.command === "search" && (call.args as { query: string }).query === wanted).length, query);

test("held activation keeps the prompt and selection and refreshes the results", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=root");
  const input = page.getByRole("combobox");
  await page.keyboard.type("Result 1");
  await expect(input).toHaveValue("Result 1");
  await page.keyboard.press("ArrowDown");
  const selected = page.getByRole("option", { selected: true });
  const picked = await selected.getAttribute("data-value");
  const before = await searchesFor(page, "Result 1");
  await page.evaluate(() => window.dangoFixture.reactivate());
  await expect(input).toHaveValue("Result 1");
  await expect(input).toBeFocused();
  await expect(selected).toHaveAttribute("data-value", picked!);
  await expect.poll(() => searchesFor(page, "Result 1")).toBe(before + 1);
});

test("held activation leaves the cursor in a pushed view's field", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=form");
  const name = page.getByLabel("Name", { exact: true });
  await expect(name).toBeFocused();
  await page.evaluate(() => window.dangoFixture.reactivate());
  await expect(name).toBeFocused();
  await expect(page.getByRole("combobox")).toHaveCount(0);
});

test("plain activation after a reset starts at an empty, focused prompt", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=root");
  await page.keyboard.type("Result 1");
  await page.evaluate(() => window.dangoFixture.show("root"));
  const input = page.getByRole("combobox");
  await expect(input).toHaveValue("");
  await expect(input).toBeFocused();
  await expect(page.getByRole("option")).toHaveCount(18);
});
