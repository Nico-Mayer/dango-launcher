import { expect, test, type Page } from "@playwright/test";
import { listTree, rootItems } from "../fixtures/data";
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

  test(`${scene}: delayed replacement preserves selected identity and falls back if removed`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    await expect(page.getByRole("option")).toHaveCount(18);
    await page.keyboard.press("ArrowDown");
    await expect(selected(page)).toHaveAttribute("data-value", "result-2");
    const reordered = [rootItems[2], rootItems[1], rootItems[0], ...rootItems.slice(3)]
      .map((item) => ({ ...item, suggested: false }));
    if (scene === "root") {
      await page.evaluate(async (items) => {
        await new Promise((resolve) => setTimeout(resolve, 50));
        await window.dangoFixture.results("", items, false);
      }, reordered);
    } else {
      const tree = structuredClone(listTree);
      if (tree.view.kind === "list") tree.view.items = [tree.view.items[2], tree.view.items[1], tree.view.items[0], ...tree.view.items.slice(3)];
      await page.evaluate(async (tree) => {
        await new Promise((resolve) => setTimeout(resolve, 50));
        await window.dangoFixture.replace(tree);
      }, tree);
    }
    await expect(page.getByRole("option").first()).toHaveAttribute("data-value", "result-3");
    await expect(selected(page)).toHaveAttribute("data-value", "result-2");
    await expect(page.getByRole("combobox")).toBeFocused();
    if (scene === "root") {
      await page.evaluate((items) => window.dangoFixture.results("", items), reordered.filter((item) => item.id !== "result-2"));
    } else {
      const tree = structuredClone(listTree);
      if (tree.view.kind === "list") tree.view.items = tree.view.items.filter((item) => item.id !== "result-2");
      await page.evaluate((tree) => window.dangoFixture.replace(tree), tree);
    }
    await expect(selected(page)).toHaveAttribute("data-value", scene === "root" ? "result-3" : "result-1");
  });
}

test("root: stale results cannot replace typed-query results", async ({ page }) => {
  await page.goto("/tests/fixtures/");
  const input = page.getByRole("combobox");
  await input.fill("Result 12");
  await expect(page.getByRole("option")).toHaveCount(1);
  await page.evaluate((items) => window.dangoFixture.results("", items), rootItems);
  await expect(page.getByRole("option")).toHaveCount(1);
  await expect(selected(page)).toHaveAttribute("data-value", "result-12");
  await expect(input).toBeFocused();
});

for (const scene of ["root", "list"]) {
  test(`${scene}: same-task replacement cannot impersonate keyboard selection`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    await expect(page.getByRole("combobox")).toBeFocused();
    await expect(page.getByRole("option")).toHaveCount(18);
    await page.evaluate(async ({ scene, items, tree }) => {
      document.querySelector("input")!.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
      if (scene === "root") await window.dangoFixture.results("", items);
      else await window.dangoFixture.replace(tree);
    }, { scene, items: structuredClone(rootItems), tree: structuredClone(listTree) });
    await expect(selected(page)).toHaveAttribute("data-value", "result-2");
    await expect(page.getByRole("combobox")).toBeFocused();
  });
}

for (const scene of ["root", "list"]) {
  test(`${scene}: thirty same-view replacements per second keep selection, focus, and nodes`, async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    await expect(page.getByRole("option")).toHaveCount(18);
    await page.keyboard.press("ArrowDown");
    await expect(selected(page)).toHaveAttribute("data-value", "result-2");
    await expect.poll(() => page.locator("main").evaluate((node) => node.getAnimations({ subtree: true }).length)).toBe(0);
    const outcome = await page.evaluate(async ({ scene, items, tree }) => {
      const listbox = document.querySelector('[role="listbox"]')!;
      const row = document.querySelector('[data-value="result-2"]')!;
      const main = document.querySelector("main")!;
      let animations = 0;
      let selectionChanges = 0;
      const observer = new MutationObserver(() => {
        selectionChanges += 1;
      });
      observer.observe(row, { attributes: true, attributeFilter: ["data-selected", "aria-selected"] });
      const started = performance.now();
      for (let i = 0; i < 30; i++) {
        if (scene === "root") await window.dangoFixture.results("", items);
        else if (tree.view.kind === "list") await window.dangoFixture.replace({ ...tree, view: { ...tree.view, loading: i < 29 } });
        animations = Math.max(animations, main.getAnimations({ subtree: true })
          .filter((animation) => !(animation.effect as KeyframeEffect).target?.closest(".dango-busy-edge")).length);
        await new Promise((resolve) => setTimeout(resolve, 1000 / 30));
      }
      observer.disconnect();
      return {
        elapsed: performance.now() - started,
        listboxSame: listbox === document.querySelector('[role="listbox"]'),
        rowSame: row === document.querySelector('[data-value="result-2"]'),
        animations,
        selectionChanges,
      };
    }, { scene, items: structuredClone(rootItems), tree: structuredClone(listTree) });
    expect(outcome.elapsed).toBeLessThan(2000);
    expect(outcome).toMatchObject({ listboxSame: true, rowSame: true, animations: 0, selectionChanges: 0 });
    await expect(selected(page)).toHaveAttribute("data-value", "result-2");
    await expect(page.getByRole("combobox")).toBeFocused();
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
