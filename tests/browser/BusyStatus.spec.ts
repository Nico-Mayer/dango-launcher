import { expect, test, type Page } from "@playwright/test";
import { detailTree, listTree, rootItems } from "../fixtures/data";
import type {} from "../fixtures/main";

const edge = (page: Page) => page.locator(".dango-busy-edge");
const status = (page: Page) => page.getByRole("status");

for (const scene of ["loading-list", "refreshing-list", "detail"] as const) {
  test(`BusyStatus: ${scene} preserves content, geometry and declared lifecycle`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    await expect(status(page)).toHaveText("Loading…");
    await expect(edge(page)).toHaveCount(1);
    await expect(page.locator('[aria-busy="true"]')).toHaveCount(1);
    expect(await status(page).evaluate(node => !node.closest('[aria-busy="true"]'))).toBe(true);
    await expect(status(page)).toHaveAttribute("aria-live", "polite");
    await expect(page.locator('[aria-valuenow], [role="progressbar"]')).toHaveCount(0);
    const footerBox = await page.locator("footer").boundingBox();
    const content = page.locator('[aria-busy="true"]');
    const contentBox = await content.boundingBox();
    const empty = scene === "loading-list";
    if (empty) {
      await expect(status(page).locator("span")).toHaveClass(/\bsr-only\b/);
      await expect(page.locator("[data-command-viewport]")).toContainText("Loading…");
    } else if (scene === "refreshing-list") {
      await expect(page.getByRole("option")).toHaveCount(18);
      await page.keyboard.press("ArrowDown");
      await expect(page.getByRole("option").nth(1)).toHaveAttribute("data-selected");
    } else await expect(page.getByText("Working…", { exact: true })).toBeVisible();
    const tree = structuredClone(scene === "detail" ? detailTree : listTree);
    if (tree.view.kind === "list") tree.view.items = empty ? [] : tree.view.items;
    if (tree.view.kind !== "form") tree.view.loading = false;
    await page.evaluate(tree => window.dangoFixture.replace(tree), tree);
    await expect(status(page)).toBeEmpty();
    await expect(edge(page)).toHaveCount(0);
    await expect(page.locator('[aria-busy="true"]')).toHaveCount(0);
    expect(await page.locator("footer").boundingBox()).toEqual(footerBox);
    expect(await page.locator('[aria-busy="false"]').boundingBox()).toEqual(contentBox);
    if (scene === "refreshing-list") await expect(page.getByRole("option").nth(1)).toHaveAttribute("data-selected");
    if (tree.view.kind !== "form") tree.view.loading = true;
    await page.evaluate(tree => window.dangoFixture.replace(tree), tree);
    await expect(edge(page)).toHaveCount(1);
    await expect(status(page)).toHaveText("Loading…");
  });
}

test("BusyStatus: root pending uses footer, failure/reset stop work, searches invent no progress", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=root");
  await expect(page.getByRole("option")).toHaveCount(18);
  const viewport = page.locator("[data-command-list]");
  const before = await viewport.boundingBox();
  await page.evaluate(item => window.dangoFixture.results("", [{ ...item, actions: [] }]), rootItems[0]);
  await page.keyboard.press("Enter");
  await expect(status(page)).toHaveText("Running Clipboard History…");
  await expect(edge(page)).toHaveCount(1);
  expect(await viewport.boundingBox()).toEqual(before);
  await expect(page.getByRole("option")).toHaveCount(1);
  await page.evaluate(() => window.dangoFixture.emit("dango://failed", "Couldn't paste. Try again."));
  await expect(status(page)).toBeEmpty();
  await expect(edge(page)).toHaveCount(0);
  await expect(page.getByText("Couldn't paste. Try again.")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(edge(page)).toHaveCount(1);
  await page.evaluate(() => window.dangoFixture.emit("dango://reset"));
  await expect(status(page)).toBeEmpty();
  await expect(edge(page)).toHaveCount(0);
  await page.getByRole("combobox").fill("Result");
  await expect(page.getByRole("option")).toHaveCount(17);
  await expect(edge(page)).toHaveCount(0);
  await expect.poll(() => page.locator("main").evaluate(node => node.getAnimations({ subtree: true }).length)).toBe(0);
});

test("BusyStatus: streaming keeps one sweep/status node and does not announce chunks", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/tests/fixtures/?scene=detail");
  await expect(status(page)).toHaveText("Loading…");
  const nodes = await page.evaluateHandle(() => {
    const status = document.querySelector('[role="status"]')!;
    const edge = document.querySelector(".dango-busy-edge")!;
    const changes = { count: 0 };
    const observer = new MutationObserver(records => { changes.count += records.length; });
    observer.observe(status, { subtree: true, characterData: true, childList: true });
    return { status, edge, animation: edge.getAnimations({ subtree: true })[0], changes, observer };
  });
  for (let i = 0; i < 30; i++) {
    await page.evaluate(i => window.dangoFixture.replace({ protocolVersion: 1, view: {
      kind: "detail", markdown: `Chunk ${i}`, loading: true, actions: [],
    } }), i);
  }
  expect(await nodes.evaluate(({ status, edge, animation, changes }) => ({
    statusSame: status === document.querySelector('[role="status"]'),
    edgeSame: edge === document.querySelector(".dango-busy-edge"),
    animationSame: animation === edge.getAnimations({ subtree: true })[0], changes: changes.count,
  }))).toEqual({ statusSame: true, edgeSame: true, animationSame: true, changes: 0 });
  await expect(page.getByText("Chunk 29", { exact: true })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(edge(page)).toHaveCount(0);
  await page.evaluate(() => window.dangoFixture.replace({ protocolVersion: 1, view: {
    kind: "detail", markdown: "Late chunk", loading: true, actions: [],
  } }));
  await expect(page.getByText("Late chunk")).toHaveCount(0);
  await expect(edge(page)).toHaveCount(0);
  await nodes.evaluate(({ observer }) => observer.disconnect());
});

test("BusyStatus: real surfaces inherit live independent busy token overrides", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=detail");
  await expect(edge(page)).toHaveCount(1);
  await page.evaluate(() => {
    document.documentElement.style.setProperty("--dango-busy-text", "rgb(201, 202, 203)");
    document.documentElement.style.setProperty("--dango-busy-indicator", "rgb(91, 171, 231)");
  });
  await expect(status(page)).toHaveCSS("color", "rgb(201, 202, 203)");
  expect(await edge(page).evaluate(node => getComputedStyle(node, "::after").backgroundImage)).toContain("rgb(91, 171, 231)");
  await expect(page.getByText("Working…", { exact: true })).toHaveCSS("color", "rgb(242, 243, 245)");
  await page.keyboard.press("Control+k");
  await expect(page.locator('[data-popover-content] [data-selected]')).toHaveCSS("background-color", "rgb(41, 61, 89)");
});

for (const change of ["reduce", "hidden"] as const) {
  test(`BusyStatus: live ${change} stops loops and panel exit releases ownership`, async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await page.goto("/tests/fixtures/?scene=detail");
    await expect(edge(page)).toHaveCount(1);
    expect(await edge(page).evaluate(node => getComputedStyle(node, "::after").animationName)).not.toBe("none");
    await page.keyboard.press("Control+k");
    const panel = page.locator("[data-popover-content]");
    await expect(panel.locator("[data-command-root]")).toBeFocused();
    await page.evaluate(() => {
      const panel = document.querySelector("[data-popover-content]")!;
      const observer = new MutationObserver(() => {
        for (const animation of panel.getAnimations()) animation.pause();
      });
      observer.observe(panel, { attributes: true, attributeFilter: ["data-state"] });
    });
    await page.keyboard.press("Escape");
    await expect(panel).toHaveAttribute("data-state", "closed");
    if (change === "reduce") await page.emulateMedia({ reducedMotion: "reduce" });
    else await page.evaluate(() => {
      Object.defineProperty(document, "hidden", { configurable: true, value: true });
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await expect(panel).toHaveCount(0);
    expect(await edge(page).evaluate(node => getComputedStyle(node, "::after").animationName)).toBe("none");
    await expect(edge(page)).toHaveCSS("background-color", "rgb(140, 185, 255)");
    await expect(status(page)).toHaveText("Loading…");
    await expect(page.getByText("Working…", { exact: true })).toBeFocused();
    await expect.poll(() => page.locator("main").evaluate(node => node.getAnimations({ subtree: true }).length)).toBe(0);
    await page.keyboard.press("Control+k");
    await expect(panel.locator("[data-command-root]")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(panel).toHaveCount(0);
    await page.evaluate(() => window.dangoFixture.emit("dango://reset"));
    await expect(edge(page)).toHaveCount(0);
    if (change === "reduce") await page.emulateMedia({ reducedMotion: "no-preference" });
    else await page.evaluate(() => {
      Object.defineProperty(document, "hidden", { configurable: true, value: false });
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await expect.poll(() => page.locator("main").evaluate(node => node.getAnimations({ subtree: true }).length)).toBe(0);
  });
}

test("BusyStatus: unmount removes busy loop and visibility listener", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=detail");
  await expect(edge(page)).toHaveCount(1);
  await page.evaluate(() => window.dangoFixture.destroy());
  await expect(page.locator("main")).toHaveCount(0);
  expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", { configurable: true, value: true });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await expect(page.locator("html")).not.toHaveAttribute("data-dango-hidden");
});

test("BusyStatus: live reduced motion removes pointer feedback while keeping selection", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=refreshing-list");
  await expect(page.getByRole("option")).toHaveCount(18);
  await page.getByRole("option").nth(1).hover();
  await expect(page.getByRole("option").nth(1)).toHaveCSS("transition-duration", "0.1s");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.getByRole("option").nth(1)).toHaveCSS("transition-duration", "0s");
  await expect(page.getByRole("option").first()).toHaveAttribute("data-selected");
  await expect(status(page)).toHaveText("Loading…");
});
