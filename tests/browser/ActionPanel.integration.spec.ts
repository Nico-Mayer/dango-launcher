import { expect, test, type Page } from "@playwright/test";
import { detailTree, formTree, listTree, rootItems } from "../fixtures/data";
import type {} from "../fixtures/main";


const panel = (page: Page) => page.locator("[data-popover-content]");
const calls = (page: Page, command = "run_action") => page.evaluate(command => window.dangoFixture.calls.filter(c => c.command === command), command);

for (const reducedMotion of ["reduce", "no-preference"] as const) {
test.describe(reducedMotion, () => {
  test.use({ reducedMotion });
for (const scene of ["root", "list", "form", "detail"]) {
  for (const chord of ["Control+k", "Meta+k"]) {
    test(`ActionPanel: ${scene} ${chord} owns selection, confirmation and focus`, async ({ page }) => {
      await page.goto(`/tests/fixtures/?scene=${scene}`);
      const origin = scene === "form" ? page.getByLabel("Password", { exact: true }) : scene === "detail" ? page.getByText("Working…", { exact: true }) : page.getByRole("combobox");
      if (scene === "form") await origin.focus();
      else await expect(origin).toBeFocused();
      if (["root", "list"].includes(scene)) await page.keyboard.press("ArrowDown");
      const parentValue = ["root", "list"].includes(scene) ? await page.locator("main [data-command-item][data-selected]").getAttribute("data-value") : null;
      await page.keyboard.press(chord);
      const scope = panel(page).locator("[data-command-root]");
      await expect(scope).toBeFocused();
      const options = panel(page).getByRole("option");
      await expect(options.first()).toHaveAttribute("data-selected");
      await page.keyboard.press("ArrowUp");
      await expect(options.first()).toHaveAttribute("data-selected");
      await options.last().hover();
      await expect(options.first()).toHaveAttribute("data-selected");
      await page.keyboard.press("ArrowDown");
      await expect(options.nth(1)).toHaveAttribute("data-selected");
      const actionId = await options.nth(1).getAttribute("data-value");
      await page.keyboard.press("Enter");
      await expect(panel(page)).toHaveCount(0);
      await expect(origin).toBeFocused();
      const invoked = await calls(page);
      expect(invoked).toHaveLength(1);
      expect(invoked[0].args).toMatchObject({ extensionId: "fixture", actionId, itemId: scene === "form" ? "fixture-form" : scene === "detail" ? "" : "result-2" });
      expect(await calls(page, "dismiss")).toHaveLength(0);
      if (parentValue) await expect(page.locator("main [data-command-item][data-selected]")).toHaveAttribute("data-value", parentValue);
      await page.getByRole("button", { name: "Actions", exact: true }).click();
      await expect(options.first()).toHaveAttribute("data-selected");
      await page.keyboard.press("Escape");
      await expect(panel(page)).toHaveCount(0);
      await expect(origin).toBeFocused();
      expect(await calls(page, "dismiss")).toHaveLength(0);
      expect(await calls(page, "cancel_invocation")).toHaveLength(0);
    });
  }
}

for (const scene of ["root", "list", "form", "detail"]) {
  test(`ActionPanel: ${scene} empty actions disable trigger and chord`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    await expect(page.getByRole("button", { name: "Actions", exact: true })).toBeEnabled();
    if (scene === "root") await page.evaluate(item => window.dangoFixture.results("", [{ ...item, actions: [] }]), rootItems[0]);
    else {
      const tree = structuredClone(scene === "list" ? listTree : scene === "form" ? formTree : detailTree);
      if (tree.view.kind === "list") tree.view.items.forEach(item => item.actions = []);
      else tree.view.actions = [];
      await page.evaluate(tree => window.dangoFixture.replace(tree), tree);
    }
    await expect(page.getByRole("button", { name: "Actions", exact: true })).toBeDisabled();
    await page.keyboard.press("Control+k");
    await expect(panel(page)).toHaveCount(0);
    expect(await calls(page)).toHaveLength(0);
  });
}

test("ActionPanel: long actions scroll inside compact portal in both palettes", async ({ page }, testInfo) => {
  for (const palette of ["dark", "light"]) {
    for (const size of [{ width: 720, height: 400 }, { width: 480, height: 300 }]) {
      await page.setViewportSize(size);
      await page.goto("/tests/fixtures/?scene=long-actions");
      await expect(page.getByRole("combobox")).toBeFocused();
      await page.evaluate(palette => document.documentElement.classList.toggle("dark", palette === "dark"), palette);
      await page.keyboard.press("Control+k");
      const options = panel(page).getByRole("option");
      await expect(options).toHaveCount(24);
      for (let i = 0; i < 30; i++) await page.keyboard.press("ArrowDown");
      await expect(options.last()).toHaveAttribute("data-selected");
      await expect(options.last()).toBeInViewport();
      const rect = await panel(page).boundingBox();
      expect(rect!.x).toBeGreaterThanOrEqual(0);
      expect(rect!.y).toBeGreaterThanOrEqual(0);
      expect(rect!.y + rect!.height).toBeLessThanOrEqual(size.height);
      expect(rect!.x + rect!.width).toBeLessThanOrEqual(size.width);
      await page.screenshot({ path: testInfo.outputPath(`${palette}-${size.width}-actions.png`) });
      await options.last().click();
      await expect(panel(page)).toHaveCount(0);
      expect(await calls(page)).toHaveLength(1);
    }
  }
});

for (const scene of ["root", "list", "form", "detail"]) {
  for (const press of ["click", "held"]) {
    test(`ActionPanel: ${scene} outside ${press} preserves parent`, async ({ page }) => {
      await page.goto(`/tests/fixtures/?scene=${scene}`);
      const origin = scene === "form" ? page.getByLabel("Password", { exact: true }) : scene === "detail" ? page.getByText("Working…", { exact: true }) : page.getByRole("combobox");
      await origin.focus();
      await page.keyboard.press("Control+k");
      await expect(panel(page).locator("[data-command-root]")).toBeFocused();
      const target = ["root", "list"].includes(scene)
        ? page.locator("main").getByRole("option").nth(1)
        : scene === "form" ? page.getByLabel("Name", { exact: true }) : origin;
      const rect = await target.boundingBox();
      await page.mouse.move(rect!.x + 20, rect!.y + 15);
      await page.mouse.down();
      if (press === "held") await expect(panel(page)).toHaveCount(0);
      await page.mouse.up();
      await expect(panel(page)).toHaveCount(0);
      await expect(origin).toBeFocused();
      expect(await calls(page)).toHaveLength(0);
      expect(await calls(page, "dismiss")).toHaveLength(0);
      expect(await calls(page, "cancel_invocation")).toHaveLength(0);
      if (["root", "list"].includes(scene)) {
        await expect(page.locator("main [data-command-item][data-selected]")).toHaveAttribute("data-value", "result-1");
      }
      await page.keyboard.press("Control+k");
      await expect(panel(page)).toBeVisible();
      await page.getByRole("button", { name: "Actions", exact: true }).click();
      await expect(panel(page)).toHaveCount(0);
      await expect(origin).toBeFocused();
    });

  }

  for (const key of ["Escape", "Enter"]) {
    test(`ActionPanel: ${scene} immediate ${key} and held repeats stay isolated`, async ({ page }) => {
      await page.goto(`/tests/fixtures/?scene=${scene}`);
      await expect(page.getByRole("button", { name: "Actions", exact: true })).toBeEnabled();
      await page.keyboard.press("Control+k");
      await page.keyboard.down(key);
      await page.keyboard.down(key);
      await page.keyboard.down(key);
      await page.keyboard.up(key);
      await expect(panel(page)).toHaveCount(0);
      expect(await calls(page)).toHaveLength(key === "Enter" ? 1 : 0);
      expect(await calls(page, "dismiss")).toHaveLength(0);
      expect(await calls(page, "cancel_invocation")).toHaveLength(0);
      await page.keyboard.press("Control+k");
      await expect(panel(page).getByRole("option").first()).toHaveAttribute("data-selected");
      await page.keyboard.press("Escape");
      await expect(panel(page)).toHaveCount(0);
    });
  }
}

test("ActionPanel: trigger keyboard activation does not confirm parent", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=root");
  const trigger = page.getByRole("button", { name: "Actions", exact: true });
  for (const key of ["Enter", "Space", "Control+k"]) {
    await trigger.focus();
    await page.keyboard.press(key);
    await expect(panel(page).locator("[data-command-root]")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(panel(page)).toHaveCount(0);
    await expect(page.getByRole("combobox")).toBeFocused();
  }
  expect(await calls(page)).toHaveLength(0);
});

for (const closingKey of ["Escape", "Enter"]) {
  test(`ActionPanel: ${closingKey} closed-presence blocks fresh keys and clicks`, async ({ page }) => {
    await page.goto("/tests/fixtures/?scene=root");
    // Hold exit presence to exercise fresh input during dismissal in either motion mode.
    await page.addStyleTag({ content: `
      @keyframes fixture-exit { from { opacity: 1; } to { opacity: 0; } }
      [data-popover-content][data-state="closed"] { animation: fixture-exit 180ms; }
    ` });
    await expect(page.getByRole("combobox")).toBeFocused();
    await page.keyboard.press("Control+k");
    await expect(panel(page).locator("[data-command-root]")).toBeFocused();
    await page.evaluate(() => {
      const content = document.querySelector<HTMLElement>("[data-popover-content]")!;
      const observer = new MutationObserver(() => {
        for (const animation of content.getAnimations()) animation.pause();
      });
      observer.observe(content, { attributes: true, attributeFilter: ["data-state"] });
      (window as unknown as { exitObserver: MutationObserver }).exitObserver = observer;
    });
    await page.keyboard.press(closingKey);
    await expect(panel(page)).toHaveAttribute("data-state", "closed");
    await expect(page.getByRole("combobox")).toBeFocused();
    await expect(panel(page).getByRole("option").first()).toHaveAttribute("aria-disabled", "true");
    await page.keyboard.press("Enter");
    await page.keyboard.press("Escape");
    await panel(page).getByRole("option").first().dispatchEvent("click");
    await page.locator("main").getByRole("option").nth(1).click({ position: { x: 20, y: 15 } });
    expect(await calls(page)).toHaveLength(closingKey === "Enter" ? 1 : 0);
    expect(await calls(page, "dismiss")).toHaveLength(0);
    await expect(page.locator("main [data-command-item][data-selected]")).toHaveAttribute("data-value", "result-1");
    await page.evaluate(() => {
      (window as unknown as { exitObserver: MutationObserver }).exitObserver.disconnect();
      for (const animation of document.querySelector("[data-popover-content]")!.getAnimations()) animation.finish();
    });
    await expect(panel(page)).toHaveCount(0);
    await expect(page.getByRole("combobox")).toBeFocused();
    await page.keyboard.press("Control+k");
    await expect(panel(page).locator("[data-command-root]")).toBeFocused();
  });

}

test("ActionPanel: actual portal inherits live typography, radius and surface overrides", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=root");
  await expect(page.getByRole("combobox")).toBeFocused();
  await page.keyboard.press("Control+k");
  await expect(panel(page).locator("[data-command-root]")).toBeFocused();
  await page.evaluate(() => {
    for (const [name, value] of Object.entries({
      "font-sans": "monospace", "text-body": "16px", "radius-overlay": "12px",
      "surface-raised": "rgb(31, 32, 33)", "overlay-width": "260px",
    })) document.documentElement.style.setProperty(`--dango-${name}`, value);
  });
  await expect(panel(page)).toHaveCSS("font-family", "monospace");
  await expect(panel(page)).toHaveCSS("border-radius", "12px");
  await expect(panel(page)).toHaveCSS("background-color", "rgb(31, 32, 33)");
  await expect(panel(page)).toHaveCSS("width", "260px");
  await expect(panel(page).getByRole("option").first()).toHaveCSS("font-size", "16px");
  expect(await panel(page).evaluate(node => !node.parentElement?.closest("[data-command-root]"))).toBe(true);
  await expect.poll(() => panel(page).evaluate(node => node.getAnimations({ subtree: true }).length)).toBe(0);
});


test("ActionPanel: opening and dismissing preserves carried failure", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=failure");
  const failure = page.getByText("Couldn't paste. Try again.");
  await expect(failure).toBeVisible();
  await page.keyboard.press("Control+k");
  await expect(panel(page).locator("[data-command-root]")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(panel(page)).toHaveCount(0);
  await expect(failure).toBeVisible();
  await expect(page.getByRole("combobox")).toBeFocused();
  await page.keyboard.type("Result");
  await expect(failure).not.toBeVisible();
});

test("ActionPanel: removal of all actions closes safely", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=root");
  await expect(page.getByRole("combobox")).toBeFocused();
  await page.keyboard.press("Control+k");
  await expect(panel(page).locator("[data-command-root]")).toBeFocused();
  await page.evaluate(item => window.dangoFixture.results("", [{ ...item, actions: [] }]), rootItems[0]);
  await expect(panel(page)).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Actions", exact: true })).toBeDisabled();
  await expect(page.getByRole("combobox")).toBeFocused();
  expect(await calls(page)).toHaveLength(0);
});

});
}
