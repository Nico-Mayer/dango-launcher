import { expect, test } from "@playwright/test";
import type {} from "../fixtures/main";

const panelSelector = "[data-popover-content]";

test("refinement: motion keeps navigation immediate and exit owns fresh input", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/tests/fixtures/?scene=root");
  const rows = page.locator("main [data-command-item]");
  await expect(rows).toHaveCount(18);
  await rows.nth(2).hover();
  await expect(rows.nth(2)).toHaveCSS("transition-duration", "0.1s");
  await page.keyboard.press("ArrowDown");
  await expect(rows.nth(1)).toHaveAttribute("data-selected");
  await expect(rows.nth(2)).toHaveCSS("transition-duration", "0s");
  await expect(rows.nth(2)).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  await expect(rows.nth(1)).toHaveCSS("transition-duration", "0s");
  await page.keyboard.press("Control+k");
  const panel = page.locator(panelSelector);
  await expect(panel).toHaveCSS("animation-duration", "0.14s");
  await page.keyboard.press("ArrowDown");
  await expect(panel.getByRole("option").nth(1)).toHaveAttribute("data-selected");
  await page.evaluate(() => {
    const panel = document.querySelector<HTMLElement>("[data-popover-content]")!;
    const observer = new MutationObserver(() => {
      if (panel.dataset.state === "closed") for (const animation of panel.getAnimations()) animation.pause();
    });
    observer.observe(panel, { attributes: true, attributeFilter: ["data-state"] });
    (window as unknown as { exitObserver: MutationObserver }).exitObserver = observer;
  });
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCSS("animation-duration", "0.09s");
  await expect(page.getByRole("combobox")).toBeFocused();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Escape");
  expect(await page.evaluate(() => window.dangoFixture.calls.filter(c => ["run_action", "dismiss"].includes(c.command)))).toHaveLength(0);
  await page.evaluate(() => {
    (window as unknown as { exitObserver: MutationObserver }).exitObserver.disconnect();
    for (const animation of document.querySelector("[data-popover-content]")!.getAnimations()) animation.finish();
  });
  await expect(panel).toHaveCount(0);
  expect(await page.locator("main").evaluate(node => node.getAnimations({ subtree: true }).length)).toBe(0);
});
