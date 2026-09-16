import { expect, test } from "@playwright/test";
import { rootItems } from "../fixtures/data";
import type {} from "../fixtures/main";

for (const scene of ["root", "list"]) {
  test(`theme: ${scene} preserves default density`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    await expect(page.getByRole("option")).toHaveCount(18);
    await expect(page.getByRole("combobox")).toHaveCSS("height", "64px");
    await expect(page.getByRole("option").first()).toHaveCSS("height", "56px");
    await expect(page.getByText("Dango", { exact: true }).locator("..")).toHaveCSS("height", "40px");
    await expect(page.getByRole("option").first()).toHaveCSS("background-color", "rgb(41, 61, 89)");
    await expect(page.locator("html")).toHaveClass("dark");
    await expect(page.locator("html")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await expect(page.locator("body")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  });
}

const overrides = {
  "--dango-selection-bg": "rgb(10, 30, 50)",
  "--dango-selection-text": "rgb(201, 202, 203)",
  "--dango-selection-edge": "rgb(81, 82, 83)",
  "--dango-hover-bg": "rgb(40, 50, 60)",
  "--dango-focus-ring": "rgb(70, 80, 90)",
  "--dango-control-bg": "rgb(100, 110, 120)",
  "--dango-control-text": "rgb(121, 122, 123)",
  "--dango-control-accent": "rgb(131, 132, 133)",
  "--dango-keycap-bg": "rgb(130, 140, 150)",
  "--dango-keycap-text": "rgb(151, 152, 153)",
  "--dango-error-text": "rgb(160, 170, 180)",
  "--dango-busy-text": "rgb(190, 200, 210)",
  "--dango-busy-indicator": "rgb(211, 212, 213)",
};

for (const scene of ["root", "list"]) {
  test(`theme: independent states reach ${scene}, actions, and portal`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene}&tokens=1`);
    const rows = page.getByRole("option");
    await expect(rows).toHaveCount(18);
    const portal = page.locator("[data-token-portal]");
    await expect(portal).toHaveCount(1);
    expect(await portal.evaluate((node) => node.parentElement === document.body)).toBe(true);
    await page.evaluate((values) => {
      for (const [key, value] of Object.entries(values)) document.documentElement.style.setProperty(key, value);
    }, overrides);
    await expect(rows.first()).toHaveCSS("background-color", overrides["--dango-selection-bg"]);
    await expect(rows.first().getByText("Clipboard History", { exact: true })).toHaveCSS("color", overrides["--dango-selection-text"]);
    await expect(rows.first()).toHaveCSS("box-shadow", /rgb\(81, 82, 83\)/);
    await rows.nth(2).hover();
    await expect(rows.nth(2)).toHaveCSS("background-color", overrides["--dango-hover-bg"]);
    await expect(page.locator("kbd").first()).toHaveCSS("background-color", overrides["--dango-keycap-bg"]);
    await expect(page.locator("kbd").first()).toHaveCSS("color", overrides["--dango-keycap-text"]);
    await expect(page.locator("[data-token-selection]")).toHaveCSS("background-color", overrides["--dango-selection-bg"]);
    await expect(page.locator("[data-token-hover]")).toHaveCSS("background-color", overrides["--dango-hover-bg"]);
    await expect(page.locator("[data-token-focus]")).toHaveCSS("border-color", overrides["--dango-focus-ring"]);
    await expect(page.locator("[data-token-busy]")).toHaveCSS("color", overrides["--dango-busy-text"]);
    await expect(page.locator("[data-token-indicator]")).toHaveCSS("background-color", overrides["--dango-busy-indicator"]);
    await page.keyboard.press("Control+k");
    await expect(page.locator("[data-popover-content]").getByRole("option").first()).toHaveCSS("background-color", overrides["--dango-selection-bg"]);
    await expect(page.locator("[data-popover-content]").getByRole("option").first()).toHaveCSS("color", overrides["--dango-selection-text"]);
    await page.locator("[data-popover-content]").getByRole("option").nth(1).hover();
    await expect(page.locator("[data-popover-content]").getByRole("option").nth(1)).toHaveCSS("background-color", overrides["--dango-hover-bg"]);
    await page.evaluate(() => document.documentElement.style.setProperty("--dango-selection-bg", "rgb(1, 2, 3)"));
    await expect(rows.first()).toHaveCSS("background-color", "rgb(1, 2, 3)");
    await expect(page.locator("[data-popover-content]").getByRole("option").first()).toHaveCSS("background-color", "rgb(1, 2, 3)");
    await expect(page.locator("[data-token-selection]")).toHaveCSS("background-color", "rgb(1, 2, 3)");
    await expect(page.locator("[data-token-hover]")).toHaveCSS("background-color", overrides["--dango-hover-bg"]);
    await expect(page.locator("[data-token-focus]")).toHaveCSS("border-color", overrides["--dango-focus-ring"]);
    await expect(page.locator("[data-token-busy]")).toHaveCSS("color", overrides["--dango-busy-text"]);
  });
}

test("theme: controls, errors, pending status and tints resolve independent roles", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=form");
  await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
  await page.evaluate((values) => {
    for (const [key, value] of Object.entries(values)) document.documentElement.style.setProperty(key, value);
  }, overrides);
  for (const label of ["Name", "Password", "Template"]) {
    const field = page.getByLabel(label, { exact: label !== "Template" });
    await expect(field).toHaveCount(1);
    await field.focus();
    await expect(field).toHaveCSS("background-color", overrides["--dango-control-bg"]);
    await expect(field).toHaveCSS("color", overrides["--dango-control-text"]);
    await expect(field).toHaveCSS("border-color", overrides["--dango-focus-ring"]);
  }
  await expect(page.getByLabel("Enabled")).toHaveCSS("accent-color", overrides["--dango-control-accent"]);
  await page.evaluate(() => window.dangoFixture.setTemplateInspection({ arguments: [], error: "Couldn't read template. Check the placeholders." }));
  await page.getByLabel("Template").fill("Hello {{");
  await expect(page.getByText("Couldn't read template. Check the placeholders.")).toHaveCSS("color", overrides["--dango-error-text"]);
  await page.evaluate(() => window.dangoFixture.show("failure"));
  await expect(page.getByText("Couldn't paste. Try again.")).toHaveCSS("color", overrides["--dango-error-text"]);
  await page.evaluate(() => window.dangoFixture.show("root"));
  await expect(page.getByRole("option")).toHaveCount(18);
  const tints = ["blue", "green", "amber", "purple", "red", "pink"];
  for (const [index, tint] of tints.entries()) {
    await page.evaluate((tint) => {
      document.documentElement.style.setProperty(`--dango-tint-${tint}`, "rgb(11, 22, 33)");
      document.documentElement.style.setProperty(`--dango-tint-${tint}-foreground`, "rgb(222, 223, 224)");
    }, tint);
    const tile = page.getByRole("option").nth(index).locator(":scope > div").first();
    await expect(tile).toHaveCSS("background-color", "rgb(11, 22, 33)");
    await expect(tile).toHaveCSS("color", "rgb(222, 223, 224)");
  }
  // A command result starts pending work; normal fixture root items are actions.
  await page.evaluate((item) => window.dangoFixture.results("", [{ ...item, actions: [] }]), rootItems[0]);
  await expect(page.getByRole("option")).toHaveCount(1);
  await page.keyboard.press("Enter");
  await expect(page.getByText("Running Clipboard History…")).toHaveCSS("color", overrides["--dango-busy-text"]);
  expect(await page.locator(".dango-busy-edge").evaluate(node => getComputedStyle(node, "::after").backgroundImage)).toContain(overrides["--dango-busy-indicator"]);
});

test("theme: text hierarchy and raised surface inherit from document root", async ({ page }) => {
  await page.goto("/tests/fixtures/?tokens=1");
  await expect(page.getByRole("option")).toHaveCount(18);
  await page.evaluate(() => {
    for (const [role, value] of Object.entries({
      "text-primary": "rgb(210, 211, 212)", "text-secondary": "rgb(180, 181, 182)",
      "text-tertiary": "rgb(150, 151, 152)", "surface-raised": "rgb(20, 21, 22)",
    })) document.documentElement.style.setProperty(`--dango-${role}`, value);
  });
  await expect(page.getByRole("option").nth(1).getByText("Result 2", { exact: true })).toHaveCSS("color", "rgb(210, 211, 212)");
  await expect(page.getByText("Dango", { exact: true })).toHaveCSS("color", "rgb(180, 181, 182)");
  await expect(page.getByText("Search copied text and images")).toHaveCSS("color", "rgb(150, 151, 152)");
  await expect(page.locator("[data-token-portal]")).toHaveCSS("color", "rgb(210, 211, 212)");
  await expect(page.locator("[data-token-portal]")).toHaveCSS("background-color", "rgb(20, 21, 22)");
  await page.keyboard.press("Control+k");
  await expect(page.locator("[data-popover-content]")).toHaveCSS("background-color", "rgb(20, 21, 22)");
  await expect(page.locator("[data-popover-content]").getByRole("option").nth(1)).toHaveCSS("color", "rgb(180, 181, 182)");
  await page.evaluate(() => window.dangoFixture.show("detail"));
  await expect(page.getByText("Working…")).toHaveCSS("color", "rgb(210, 211, 212)");
});

for (const palette of ["dark", "light"]) {
  for (const scene of ["root", "list", "form", "detail", "failure", "long-actions"]) {
    test(`theme: ${palette} ${scene} renders built-in palette`, async ({ page }, testInfo) => {
      await page.goto(`/tests/fixtures/?scene=${scene}`);
      await expect(page.getByText("Dango", { exact: true })).toBeVisible();
      await page.evaluate((palette) => document.documentElement.classList.toggle("dark", palette === "dark"), palette);
      if (["root", "list", "failure", "long-actions"].includes(scene)) await expect(page.getByRole("option")).toHaveCount(18);
      if (scene === "long-actions") {
        await page.keyboard.press("Control+k");
        await expect(page.locator("[data-popover-content]").getByRole("option")).toHaveCount(24);
      }
      const shell = page.locator("main");
      await expect(shell).toHaveCSS("background-color", palette === "dark" ? "rgb(32, 33, 36)" : "rgb(250, 250, 250)");
      await expect(page.getByText("Dango", { exact: true })).toHaveCSS("color", palette === "dark" ? "rgb(200, 203, 210)" : "rgb(68, 74, 85)");
      await expect(page.locator("html")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
      await page.screenshot({ path: testInfo.outputPath(`${palette}-${scene}.png`) });
    });
  }
}

test("theme: foundation overrides reach existing consumers and portal", async ({ page }) => {
  await page.goto("/tests/fixtures/?tokens=1");
  await expect(page.getByRole("option")).toHaveCount(18);
  await page.evaluate(() => {
    for (const [role, value] of Object.entries({
      "font-sans": "monospace", "text-body": "15px", "text-meta": "13px",
      "text-search": "23px", "weight-medium": "600", "radius-row": "9px",
      "radius-shell": "15px", "radius-overlay": "11px", "radius-control": "7px",
      "row-inset": "14px", "overlay-width": "280px", "edge-width": "2px",
      "elevation-overlay": "0px 1px 2px rgb(1, 2, 3)", "layer-overlay": "12",
    })) document.documentElement.style.setProperty(`--dango-${role}`, value);
  });
  await expect(page.getByRole("combobox")).toHaveCSS("font-size", "23px");
  await expect(page.getByRole("option").first()).toHaveCSS("border-radius", "9px");
  await expect(page.getByRole("option").first()).toHaveCSS("padding-left", "14px");
  await expect(page.getByRole("option").first().getByText("Clipboard History", { exact: true })).toHaveCSS("font-size", "15px");
  await expect(page.getByText("Dango", { exact: true })).toHaveCSS("font-size", "13px");
  await expect(page.getByText("Dango", { exact: true })).toHaveCSS("font-weight", "600");
  await expect(page.getByText("Dango", { exact: true }).locator("..")).toHaveCSS("border-top-width", "2px");
  await expect(page.locator("[data-token-portal]")).toHaveCSS("font-family", "monospace");
  await expect(page.locator("[data-token-portal]")).toHaveCSS("border-radius", "7px");
  await page.keyboard.press("Control+k");
  const panel = page.locator("[data-popover-content]");
  await expect(panel).toHaveCSS("width", "280px");
  await expect(panel).toHaveCSS("border-radius", "11px");
  await expect(panel).toHaveCSS("box-shadow", /rgb\(1, 2, 3\) 0px 1px 2px/);
  await expect(panel).toHaveCSS("z-index", "12");
  await page.evaluate(async () => {
    document.documentElement.style.setProperty("--dango-icon-status", "18px");
    await window.dangoFixture.show("failure");
  });
  await expect(page.getByText("Couldn't paste. Try again.").locator("svg")).toHaveCSS("width", "18px");
});
