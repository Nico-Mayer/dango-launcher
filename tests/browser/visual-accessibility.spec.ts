import { expect, test, type Page } from "@playwright/test";
import { actions, detailTree, formTree, listTree, longActions, rootItems } from "../fixtures/data";
import type {} from "../fixtures/main";

// Resolve CSS colors in the browser, then composite each translucent ancestor.
async function textContrast(page: Page) {
  return page.evaluate(() => {
    const ctx = document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
    const rgba = (color: string) => {
      ctx.clearRect(0, 0, 1, 1);
      ctx.fillStyle = color;
      ctx.fillRect(0, 0, 1, 1);
      return [...ctx.getImageData(0, 0, 1, 1).data].map((x, i) => i === 3 ? x / 255 : x);
    };
    const over = (fg: number[], bg: number[]) => fg.slice(0, 3).map((x, i) => x * fg[3] + bg[i] * (1 - fg[3])).concat(1);
    const luminance = (color: number[]) => color.slice(0, 3).map(x => x / 255).map(x => x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4).reduce((sum, x, i) => sum + x * [0.2126, 0.7152, 0.0722][i], 0);
    const ratio = (a: number[], b: number[]) => (Math.max(luminance(a), luminance(b)) + 0.05) / (Math.min(luminance(a), luminance(b)) + 0.05);
    const background = (node: Element | null) => {
      const ancestors: Element[] = [];
      for (let el = node; el; el = el.parentElement) ancestors.unshift(el);
      return ancestors.reduce((bg, el) => over(rgba(getComputedStyle(el).backgroundColor), bg), [255, 255, 255, 1]);
    };
    const visible = (node: Element) => {
      const box = node.getBoundingClientRect();
      return box.width > 1 && box.height > 1 && box.bottom > 0 && box.top < innerHeight
        && !node.closest('[aria-hidden="true"], [disabled], .sr-only');
    };
    const measurements: { text: string; ratio: number }[] = [];
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const el = node.parentElement;
      if (!el || !node.textContent?.trim() || !visible(el) || el.closest("script, style")) continue;
      const bg = background(el);
      measurements.push({ text: node.textContent.trim(), ratio: ratio(over(rgba(getComputedStyle(el).color), bg), bg) });
    }
    for (const el of document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("input, textarea")) {
      if (!visible(el)) continue;
      const bg = background(el);
      measurements.push({ text: el.value || el.placeholder, ratio: ratio(over(rgba(getComputedStyle(el, el.value ? null : "::placeholder").color), bg), bg) });
    }
    const focus = document.activeElement;
    let focusRatio: number | null = null;
    if (focus && focus !== document.body) {
      const style = getComputedStyle(focus);
      const color = style.outlineStyle !== "none" ? style.outlineColor
        : style.boxShadow !== "none" ? style.boxShadow.match(/(?:rgba?\([^)]*\)|#[\da-f]+)/i)?.[0]
        : style.borderStyle !== "none" && parseFloat(style.borderWidth) > 0 ? style.borderColor : null;
      if (color) focusRatio = style.outlineStyle !== "none" && parseFloat(style.outlineOffset) > 0
        ? ratio(rgba(color), background(focus.parentElement))
        : Math.min(ratio(rgba(color), background(focus)), ratio(rgba(color), background(focus.parentElement)));
    }
    return { measurements, focusRatio };
  });
}

for (const palette of ["dark", "light"]) {
  for (const width of [720, 480]) {
    test(`visual: ${palette} ${width} long content, contrast, focus and bounded actions`, async ({ page }, testInfo) => {
      await page.emulateMedia({ reducedMotion: "reduce" });
      const size = { width, height: width === 720 ? 400 : 300 };
      await page.setViewportSize(size);
      const records: { scene: string; minimumText: number; focus: number | null }[] = [];
      for (const scene of ["root", "refreshing-list", "detail", "form", "failure", "loading-list", "empty-list"]) {
        await page.goto(`/tests/fixtures/?scene=${scene}`);
        await expect(page.getByText("Dango", { exact: true })).toBeVisible();
        await page.evaluate(palette => document.documentElement.classList.toggle("dark", palette === "dark"), palette);
        if (scene === "root") await page.evaluate(items => window.dangoFixture.results("", items), rootItems.map(item => ({ ...item, title: item.title + " long title".repeat(15), subtitle: "Long metadata ".repeat(12), actions: longActions })));
        if (scene === "refreshing-list") {
          const tree = structuredClone(listTree);
          if (tree.view.kind === "list") {
            tree.view.loading = true;
            tree.view.items = tree.view.items.map(item => ({ ...item, title: "Unbroken".repeat(40), subtitle: "Supporting detail ".repeat(12), actions: longActions.map(a => ({ ...a, title: a.title + " long label".repeat(15) })) }));
          }
          await page.evaluate(tree => window.dangoFixture.replace(tree), tree);
        }
        if (scene === "detail") await page.evaluate(tree => window.dangoFixture.replace(tree), { ...detailTree, view: { kind: "detail" as const, markdown: "Unbroken".repeat(150), loading: true, actions } });
        if (scene === "form") {
          const tree = structuredClone(formTree);
          if (tree.view.kind === "form") tree.view.fields.forEach(f => f.label += "Unbroken".repeat(40));
          await page.evaluate(tree => window.dangoFixture.replace(tree), tree);
          await page.evaluate(() => window.dangoFixture.setTemplateInspection({ arguments: ["Unbroken".repeat(60)], error: null }));
          await page.locator("textarea").fill("Long {{name}} ".repeat(20));
          await page.locator("textarea").focus();
        }
        const overflow = await page.locator('main, main form, main [aria-busy], main [tabindex="-1"], [data-command-list]').evaluateAll(nodes => nodes.filter(n => n.scrollWidth > n.clientWidth + 1).map(n => ({ tag: n.tagName, class: n.className, scroll: n.scrollWidth, width: n.clientWidth })));
        expect(overflow, `${scene}: horizontal overflow`).toEqual([]);
        const trigger = page.getByRole("button", { name: "Actions", exact: true });
        await expect(trigger).toBeInViewport();
        expect((await page.locator("footer").boundingBox())!.height).toBe(40);
        if (["root", "refreshing-list"].includes(scene)) {
          await expect(page.getByRole("combobox")).toHaveCSS("height", "64px");
          await expect(page.locator("main [data-command-item]").first()).toHaveCSS("height", "56px");
          await page.locator("main [data-command-item]").nth(1).hover();
        }
        const reading = await textContrast(page);
        expect(reading.measurements.filter(x => x.ratio < 4.5), `${scene}: low text contrast`).toEqual([]);
        records.push({ scene, minimumText: Math.min(...reading.measurements.map(x => x.ratio)), focus: reading.focusRatio });
        if (scene === "form") expect(reading.focusRatio, `${scene}: focus contrast`).toBeGreaterThanOrEqual(3);
        if (["root", "refreshing-list"].includes(scene)) {
          await expect(page.getByRole("combobox")).toBeFocused();
          await expect(page.getByRole("combobox")).toHaveCSS("box-shadow", "none");
          await expect(page.getByRole("combobox")).toHaveCSS("outline-style", "none");
        }
        await page.screenshot({ path: testInfo.outputPath(`${palette}-${width}-${scene}.png`) });
        if (scene === "refreshing-list") {
          await page.keyboard.press("Control+k");
          const panel = page.locator("[data-popover-content]");
          await expect(panel.locator("[data-command-root]")).toBeFocused();
          for (let i = 0; i < 30; i++) await page.keyboard.press("ArrowDown");
          await expect(panel.getByRole("option").last()).toHaveAttribute("data-selected");
          await expect(panel.getByRole("option").last()).toBeInViewport();
          const box = (await panel.boundingBox())!;
          expect(box.x).toBeGreaterThanOrEqual(0);
          expect(box.y).toBeGreaterThanOrEqual(0);
          expect(box.x + box.width).toBeLessThanOrEqual(size.width);
          expect(box.y + box.height).toBeLessThanOrEqual(size.height);
          expect(await panel.evaluate(n => n.scrollWidth <= n.clientWidth)).toBe(true);
          const reading = await textContrast(page);
          expect(reading.measurements.filter(x => x.ratio < 4.5)).toEqual([]);
          await expect(panel.locator("[data-command-root]")).toHaveCSS("outline-style", "none");
          await expect(panel.locator("[data-selected]")).toHaveCount(1);
          records.push({ scene: "actions", minimumText: Math.min(...reading.measurements.map(x => x.ratio)), focus: reading.focusRatio });
          await page.screenshot({ path: testInfo.outputPath(`${palette}-${width}-actions.png`) });
        }
      }
      await testInfo.attach(`${palette}-${width}-contrast.json`, { body: JSON.stringify(records, null, 2), contentType: "application/json" });
    });
  }
}

for (const palette of ["dark", "light"]) {
  test(`visual: ${palette} fields/errors/trigger focus contrast and live state overrides`, async ({ page }, testInfo) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/tests/fixtures/?scene=form");
    await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
    await page.evaluate(palette => document.documentElement.classList.toggle("dark", palette === "dark"), palette);
    for (const label of ["Name", "Password", "Enabled", "Template"]) {
      await page.getByLabel(label, { exact: true }).focus();
      await page.keyboard.press("Tab");
      await page.keyboard.press("Shift+Tab");
      const reading = await textContrast(page);
      expect(reading.measurements.filter(x => x.ratio < 4.5)).toEqual([]);
      expect(reading.focusRatio, label).toBeGreaterThanOrEqual(3);
    }
    await page.evaluate(() => window.dangoFixture.setTemplateInspection({ arguments: [], error: "Couldn't read template. Check the placeholders." }));
    await page.getByLabel("Template", { exact: true }).fill("{{");
    await expect(page.getByLabel("Template", { exact: true })).toHaveAttribute("aria-invalid", "true");
    const error = page.getByText("Couldn't read template. Check the placeholders.");
    await error.scrollIntoViewIfNeeded();
    await expect(error).toBeInViewport();
    expect((await textContrast(page)).measurements.filter(x => x.ratio < 4.5)).toEqual([]);
    await page.screenshot({ path: testInfo.outputPath(`${palette}-field-error.png`) });
    await page.getByRole("button", { name: "Actions", exact: true }).focus();
    expect((await textContrast(page)).focusRatio).toBeGreaterThanOrEqual(3);

    await page.evaluate(() => window.dangoFixture.show("refreshing-list"));
    await expect(page.getByRole("combobox")).toBeFocused();
    await page.evaluate(() => {
      for (const [role, value] of Object.entries({
        "selection-bg": "rgb(41, 81, 121)", "focus-ring": "rgb(121, 191, 241)",
        "hover-bg": "rgb(71, 72, 73)", "busy-text": "rgb(201, 202, 203)",
        "busy-indicator": "rgb(111, 171, 231)", "error-text": "rgb(241, 141, 151)",
      })) document.documentElement.style.setProperty(`--dango-${role}`, value);
    });
    await expect(page.locator("main [data-selected]")).toHaveCSS("background-color", "rgb(41, 81, 121)");
    await expect(page.getByRole("status")).toHaveCSS("color", "rgb(201, 202, 203)");
    await expect(page.locator(".dango-busy-edge")).toHaveCSS("background-color", "rgb(111, 171, 231)");
    await expect(page.getByRole("combobox")).toHaveCSS("box-shadow", "none");
    await page.keyboard.press("Control+k");
    const panel = page.locator("[data-popover-content]");
    await expect(panel.locator("[data-command-root]")).toBeFocused();
    await expect(panel.locator("[data-selected]")).toHaveCSS("background-color", "rgb(41, 81, 121)");
    await expect(panel.locator("[data-command-root]")).toHaveCSS("outline-style", "none");
    await panel.getByRole("option").nth(1).hover();
    await expect(panel.getByRole("option").nth(1)).toHaveCSS("background-color", "rgb(71, 72, 73)");
    await expect(panel.locator("[data-selected]")).toHaveCSS("background-color", "rgb(41, 81, 121)");
    await page.keyboard.press("Escape");
    await expect(panel).toHaveCount(0);
    await page.evaluate(() => window.dangoFixture.emit("dango://failed", "Couldn't paste. Try again."));
    await expect(page.getByText("Couldn't paste. Try again.")).toHaveCSS("color", "rgb(241, 141, 151)");
    await expect(page.getByRole("status")).toHaveCSS("color", "rgb(201, 202, 203)");
  });
}
