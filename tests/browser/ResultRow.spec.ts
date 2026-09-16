import { expect, test, type Page } from "@playwright/test";
import { rootItems } from "../fixtures/data";
import type {} from "../fixtures/main";

async function installImageStub(page: Page) {
  const requests: string[] = [];
  await page.route(/^(asset:|https?:\/\/asset\.localhost)/, async (route) => {
    requests.push(route.request().url());
    await route.fulfill({ contentType: "image/png", path: "tests/fixtures/identity.png" });
  });
  return requests;
}

for (const palette of ["dark", "light"]) {
  test(`ResultRow: ${palette} identities, matches, previews and fallback`, async ({ page }, testInfo) => {
    const errors: string[] = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto("/tests/fixtures/?scene=root");
    await expect(page.getByRole("option")).toHaveCount(18);
    await page.evaluate(palette => document.documentElement.classList.toggle("dark", palette === "dark"), palette);
    await page.screenshot({ path: testInfo.outputPath(`${palette}-tints-first.png`) });
    await page.getByRole("option").nth(5).scrollIntoViewIfNeeded();
    await page.screenshot({ path: testInfo.outputPath(`${palette}-tints-last.png`) });
    const requests = await installImageStub(page);
    await page.evaluate(item => window.dangoFixture.results("", [
      item,
      { ...item, id: "application", title: "Application icon", icon: "/fixture/application.png", matchPositions: [] },
      { ...item, id: "favicon", title: "Quicklink favicon", icon: "/fixture/favicon.png", matchPositions: [] },
      { ...item, id: "unknown", title: "Unknown named icon", icon: "icon:unregistered", matchPositions: [] },
      { ...item, id: "missing", title: "Missing icon", icon: null, matchPositions: [] },
    ]), rootItems[0]);
    const rows = page.getByRole("option");
    await expect(rows).toHaveCount(5);
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowUp");
    const tile = rows.first().locator(":scope > div").first();
    await expect(tile).toHaveCSS("width", "32px");
    await expect(tile).toHaveCSS("border-radius", "6px");
    await expect(tile.locator("svg")).toHaveCSS("width", "20px");
    await expect(tile.locator("svg")).toHaveAttribute("width", "20");
    await expect(tile.locator("svg")).toHaveAttribute("stroke-width", "2.4");
    await expect(rows.first().locator(".font-semibold")).toHaveText("Clip");
    for (const index of [1, 2]) {
      const image = rows.nth(index).locator("img");
      await expect(image).toHaveCSS("object-fit", "contain");
      await expect(image).toHaveCSS("width", "32px");
      await expect(image).toHaveCSS("height", "32px");
      await expect(image).toHaveCSS("border-width", "0px");
      await expect(image).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
      await expect(image).toHaveCSS("filter", "none");
      await expect.poll(() => image.evaluate((node: HTMLImageElement) => node.naturalWidth / node.naturalHeight)).toBe(2);
    }
    for (const index of [3, 4]) {
      await expect(rows.nth(index).locator("img, svg")).toHaveCount(0);
      await expect(rows.nth(index).locator(":scope > div").first()).toHaveCSS("width", "32px");
      await expect(rows.nth(index).locator(":scope > div").first()).toHaveCSS("border-radius", "4px");
    }
    await page.screenshot({ path: testInfo.outputPath(`${palette}-result-identities.png`) });
    expect(requests).toHaveLength(2);
    expect(requests.every(url => /(?:application|favicon)\.png/.test(url))).toBe(true);

    await page.evaluate(() => window.dangoFixture.replace({
      protocolVersion: 1,
      view: { kind: "list", filtering: "launcher", loading: false, emptyState: null, items: [
        { id: "preview", title: "Clipboard preview", subtitle: "Copied image", icon: "/fixture/clipboard.png", actions: [] },
        { id: "unknown", title: "Unknown named icon", subtitle: null, icon: "icon:toString", actions: [] },
      ] },
    }));
    await expect(rows).toHaveCount(2);
    const preview = rows.first().locator("img");
    await expect(preview).toHaveCSS("object-fit", "cover");
    await expect(preview).toHaveCSS("border-width", "1px");
    await expect(preview).toHaveCSS("border-radius", "4px");
    await expect.poll(() => preview.evaluate((node: HTMLImageElement) => node.naturalWidth)).toBe(64);
    await expect(rows.nth(1).locator("img, svg")).toHaveCount(0);
    await page.screenshot({ path: testInfo.outputPath(`${palette}-clipboard-preview.png`) });
    expect(requests).toHaveLength(3);
    expect(errors).toEqual([]);
  });
}

test("ResultRow: icon, radius, edge and typography tokens reach each presentation", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=root");
  await expect(page.getByRole("option")).toHaveCount(18);
  await installImageStub(page);
  await page.evaluate(item => window.dangoFixture.results("", [item, { ...item, id: "missing", icon: null }]), rootItems[0]);
  await page.evaluate(() => {
    for (const [role, value] of Object.entries({
      "icon-tile": "36px", "icon-result": "22px", "radius-6": "7px", "radius-4": "5px",
      "text-body": "15px", "text-meta": "13px", "weight-semibold": "700", "edge-width": "2px",
    })) document.documentElement.style.setProperty(`--dango-${role}`, value);
  });
  const rows = page.getByRole("option");
  await expect(rows).toHaveCount(2);
  await expect(rows.first().locator(":scope > div").first()).toHaveCSS("width", "36px");
  await expect(rows.first().locator(":scope > div").first()).toHaveCSS("border-radius", "7px");
  await expect(rows.first().locator("svg")).toHaveCSS("width", "22px");
  await expect(rows.first().getByText("Clipboard History", { exact: true })).toHaveCSS("font-size", "15px");
  await expect(rows.first().getByText("Search copied text and images")).toHaveCSS("font-size", "13px");
  await expect(rows.first().locator(".font-semibold")).toHaveCSS("font-weight", "700");
  await expect(rows.nth(1).locator(":scope > div").first()).toHaveCSS("width", "36px");
  await expect(rows.nth(1).locator(":scope > div").first()).toHaveCSS("border-radius", "5px");
  await page.evaluate(() => window.dangoFixture.replace({
    protocolVersion: 1,
    view: { kind: "list", filtering: "launcher", loading: false, emptyState: null,
      items: [{ id: "preview", title: "Clipboard preview", subtitle: null, icon: "/fixture/clipboard.png", actions: [] }] },
  }));
  await expect(rows.first().locator("img")).toHaveCSS("width", "36px");
  await expect(rows.first().locator("img")).toHaveCSS("border-radius", "5px");
  await expect(rows.first().locator("img")).toHaveCSS("border-width", "2px");
});
