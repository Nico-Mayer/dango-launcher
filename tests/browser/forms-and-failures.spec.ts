import { expect, test, type Page } from "@playwright/test";
import type {} from "../fixtures/main";

const actionCalls = (page: Page) => page.evaluate(() =>
  window.dangoFixture.calls.filter((call) => call.command === "run_action"));

test("FormFields: Enter submits text and unchanged initial values once", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=form");
  const name = page.getByLabel("Name", { exact: true });
  await expect(name).toBeFocused();
  await page.keyboard.type(" revised");
  await page.keyboard.press("Enter");
  await expect.poll(() => actionCalls(page)).toHaveLength(1);
  expect((await actionCalls(page))[0].args).toEqual({
    extensionId: "fixture", itemId: "fixture-form", actionId: "save",
    values: { name: "Morning note revised", password: "fixture-only", enabled: "true", template: "Hello {{name}}" },
  });
});

for (const modifier of ["Control", "Meta"]) {
  test(`FormFields: template Enter edits; ${modifier}+Enter submits once`, async ({ page }) => {
    await page.goto("/tests/fixtures/?scene=template");
    const template = page.getByLabel("Template");
    await expect(template).toBeFocused();
    await page.keyboard.press("Enter");
    await page.keyboard.type("Second line");
    await expect(template).toHaveValue("Hello {{name}}\nSecond line");
    expect(await actionCalls(page)).toEqual([]);
    await page.keyboard.press(`${modifier}+Enter`);
    await expect.poll(() => actionCalls(page)).toHaveLength(1);
    expect((await actionCalls(page))[0].args).toEqual({
      extensionId: "fixture", itemId: "fixture-form", actionId: "save",
      values: { template: "Hello {{name}}\nSecond line" },
    });
  });
}

for (const order of ["before-reset", "after-reset"]) {
  for (const clear of ["typing", "action", "Escape"]) {
    test(`App: failure ${order} survives activation; cleared by ${clear}`, async ({ page }) => {
      await page.goto("/tests/fixtures/");
      await expect(page.getByRole("option")).toHaveCount(18);
      await page.evaluate(async (order) => {
        if (order === "before-reset") await window.dangoFixture.emit("dango://failed", "Couldn't paste. Try again.");
        await window.dangoFixture.emit("dango://reset");
        if (order === "after-reset") await window.dangoFixture.emit("dango://failed", "Couldn't paste. Try again.");
        await window.dangoFixture.emit("dango://activate", null);
      }, order);
      const failure = page.getByText("Couldn't paste. Try again.");
      await expect(failure).toBeVisible();
      await expect(page.getByRole("combobox")).toBeFocused();
      await expect(page.getByRole("option")).toHaveCount(18);
      if (clear === "typing") await page.keyboard.type("Result");
      else await page.keyboard.press(clear === "action" ? "Enter" : "Escape");
      await expect(failure).not.toBeVisible();
    });
  }
}
