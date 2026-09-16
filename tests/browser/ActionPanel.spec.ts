import { expect, test } from "@playwright/test";

test.use({ reducedMotion: "reduce" });

test("ActionPanel: isolated portaled Command proof", async ({ page }) => {
  await page.goto("/tests/fixtures/actions.html");
  const parent = page.locator("[data-command-root]").first();
  await page.getByRole("button", { name: "Actions", exact: true }).click();
  const panel = page.locator("[data-popover-content]");
  const scope = panel.locator("[data-command-root]");
  const options = panel.getByRole("option");
  await expect(scope).toBeFocused();
  await expect(options.first()).toHaveAttribute("data-selected");
  await expect(parent.getByRole("option")).toHaveCount(2);
  await expect(panel.getByRole("combobox")).toHaveCount(0);
  expect(await panel.evaluate(node => !node.parentElement?.closest("[data-command-root]"))).toBe(true);
  await page.keyboard.press("ArrowUp");
  await expect(options.first()).toHaveAttribute("data-selected");
  await options.nth(2).hover();
  await expect(options.first()).toHaveAttribute("data-selected");
  await page.keyboard.press("ArrowDown");
  await expect(options.nth(1)).toHaveAttribute("data-selected");
  await expect(scope).toHaveAttribute("aria-activedescendant", await options.nth(1).getAttribute("id") as string);
  await page.keyboard.press("End");
  await page.keyboard.press("ArrowDown");
  await expect(options.last()).toHaveAttribute("data-selected");
  await expect(options.last()).toBeInViewport();
  await expect(parent.getByRole("option").first()).toHaveAttribute("data-selected");
  await expect(page.locator("output")).toHaveAttribute("data-parent-keys", "0");
  await page.keyboard.press("Enter");
  await expect(panel).toHaveCount(0);
  await expect(page.locator("output")).toHaveAttribute("data-runs", "1");
  await expect(page.getByRole("combobox")).toBeFocused();
});
