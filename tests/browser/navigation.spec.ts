import { expect, test, type Page } from "@playwright/test";
import type {} from "../fixtures/main";

const selected = (page: Page) => page.getByRole("option", { selected: true });

for (const scene of ["root", "list"]) {
  test(`${scene}: non-wrapping selection and first-row scrolling`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    const rows = page.getByRole("option");
    const list = page.getByRole("listbox");
    await expect(rows).toHaveCount(18);
    await expect(selected(page)).toHaveAttribute("data-value", "result-1");
    await page.keyboard.press("ArrowUp");
    await expect(selected(page)).toHaveAttribute("data-value", "result-1");
    for (let i = 2; i <= 18; i++) {
      await page.keyboard.press("ArrowDown");
      await expect(selected(page)).toHaveAttribute("data-value", `result-${i}`);
    }
    await page.keyboard.press("ArrowDown");
    await expect(selected(page)).toHaveAttribute("data-value", "result-18");
    expect(await list.evaluate((node) => node.scrollTop)).toBeGreaterThan(0);
    const last = await selected(page).boundingBox();
    const viewport = await list.boundingBox();
    expect(last!.y + last!.height).toBeLessThanOrEqual(viewport!.y + viewport!.height + 1);
    for (let i = 17; i >= 1; i--) {
      await page.keyboard.press("ArrowUp");
      await expect(selected(page)).toHaveAttribute("data-value", `result-${i}`);
    }
    await expect.poll(() => list.evaluate((node) => node.scrollTop)).toBe(0);
    if (scene === "root") await expect(page.getByText("Suggestions", { exact: true })).toBeInViewport();
    await expect(page.getByRole("combobox")).toBeFocused();
  });

  test(`${scene}: pointer hover never selects and keyboard suppresses resting hover`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    const row = page.getByRole("option").nth(2);
    const list = page.getByRole("listbox");
    await expect(row).toBeVisible();
    const box = await row.boundingBox();
    const point = { x: box!.x + 20, y: box!.y + 20 };
    await page.mouse.move(point.x, point.y);
    await expect(list).toHaveAttribute("data-pointer", "");
    await expect(selected(page)).toHaveAttribute("data-value", "result-1");
    const hoverColor = await row.evaluate((node) => getComputedStyle(node).backgroundColor);
    await page.keyboard.press("ArrowDown");
    await expect(list).not.toHaveAttribute("data-pointer");
    await expect(selected(page)).toHaveAttribute("data-value", "result-2");
    expect(await row.evaluate((node) => getComputedStyle(node).backgroundColor)).not.toBe(hoverColor);
    await page.evaluate(({ x, y }) => window.dispatchEvent(new PointerEvent("pointermove", {
      clientX: x, clientY: y,
    })), point);
    await expect(list).not.toHaveAttribute("data-pointer");
    await page.mouse.move(point.x + 1, point.y);
    await expect(list).toHaveAttribute("data-pointer", "");
    await expect(selected(page)).toHaveAttribute("data-value", "result-2");
  });

  test(`${scene}: clicking keeps typed-query focus`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    await page.getByRole("option").nth(1).click();
    const input = page.getByRole("combobox");
    await expect(input).toBeFocused();
    await page.keyboard.type("Result 12");
    await expect(input).toHaveValue("Result 12");
    await expect(page.getByRole("option")).toHaveCount(1);
    await expect(selected(page)).toHaveAttribute("data-value", "result-12");
    await expect(input).toBeFocused();
  });

}

test("root: the first row of a later group is scrolled fully into view", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=root");
  await expect(page.getByRole("option")).toHaveCount(18);
  const inView = () => page.evaluate(() => {
    const list = document.querySelector('[role="listbox"]')!.getBoundingClientRect();
    const row = document.querySelector('[role="option"][aria-selected="true"]')!.getBoundingClientRect();
    return row.top >= list.top && row.bottom <= list.bottom;
  });
  for (let i = 2; i <= 6; i++) {
    await page.keyboard.press("ArrowDown");
    await expect(selected(page)).toHaveAttribute("data-value", `result-${i}`);
    await expect.poll(inView, `result-${i} inside the scroller`).toBe(true);
  }
  await expect(page.getByText("Everything else")).toBeInViewport();
  for (let i = 5; i >= 1; i--) {
    await page.keyboard.press("ArrowUp");
    await expect(selected(page)).toHaveAttribute("data-value", `result-${i}`);
    await expect.poll(inView, `result-${i} inside the scroller`).toBe(true);
  }
  await expect(page.getByText("Suggestions")).toBeInViewport();
});
