import { expect, test } from "@playwright/test";
import { detailTree } from "../fixtures/data";
import type {} from "../fixtures/main";

for (const scene of ["root", "list", "form", "detail", "protocol-error"]) {
  test(`presentation: ${scene} shares shell geometry`, async ({ page }) => {
    await page.goto(`/tests/fixtures/?scene=${scene === "protocol-error" ? "detail" : scene}`);
    await expect(page.getByText("Dango", { exact: true })).toBeVisible();
    if (scene === "protocol-error") {
      await page.evaluate((tree) => window.dangoFixture.replace({ ...tree, protocolVersion: 99 }), detailTree);
      await expect(page.getByText("This view needs a newer version of Dango.")).toBeVisible();
    }
    await expect(page.locator("main")).toHaveCSS("width", "720px");
    await expect(page.locator("main")).toHaveCSS("height", "400px");
    await expect(page.locator("main")).toHaveCSS("border-radius", "14px");
    if (scene !== "protocol-error") {
      await expect(page.locator("footer")).toHaveCSS("height", "40px");
      expect((await page.locator("footer").boundingBox())?.y).toBe(359);
    }
  });
}

for (const kind of ["text", "password", "template", "toggle"] as const) {
  test(`FormFields: first ${kind} field focuses and submits string values`, async ({ page }) => {
    await page.goto("/tests/fixtures/?scene=form");
    await expect(page.getByLabel("Name", { exact: true })).toBeFocused();
    await page.evaluate((kind) => window.dangoFixture.replace({
      protocolVersion: 1,
      view: { kind: "form", itemId: "first-field", actions: [{ id: "save", title: "Save", shortcut: null }],
        fields: [{ id: "first", label: "First field", kind, value: kind === "toggle" ? "true" : "Initial value" }] },
    }), kind);
    const field = page.getByLabel("First field", { exact: true });
    await expect(field).toBeFocused();
    if (kind === "toggle") {
      await expect(field).toHaveRole("checkbox");
      await expect(field).toBeChecked();
      await page.keyboard.press("Space");
      await expect(field).not.toBeChecked();
    } else {
      await expect(field).toHaveValue("Initial value");
      if (kind === "password") await expect(field).toHaveAttribute("type", "password");
      await page.keyboard.type(" edited");
      await expect(field).toHaveValue("Initial value edited");
    }
    await page.keyboard.press(kind === "template" ? "Control+Enter" : "Enter");
    await expect.poll(() => page.evaluate(() => window.dangoFixture.calls.filter(c => c.command === "run_action"))).toHaveLength(1);
    expect(await page.evaluate(() => window.dangoFixture.calls.find(c => c.command === "run_action")?.args)).toEqual({
      extensionId: "fixture", itemId: "first-field", actionId: "save",
      values: { first: kind === "toggle" ? "false" : "Initial value edited" },
    });
  });
}

test("FormFields: template help and editable errors stay associated", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=template");
  const field = page.getByLabel("Template", { exact: true });
  await expect(field).toBeFocused();
  await expect(field).toHaveAccessibleDescription("Will ask for: name");
  await expect(field).toHaveAttribute("aria-invalid", "false");
  await page.evaluate(() => window.dangoFixture.setTemplateInspection({ arguments: [], error: "Couldn't read template. Check the placeholders." }));
  await field.fill("Hello {{");
  await expect(field).toHaveAccessibleDescription("Couldn't read template. Check the placeholders.");
  await expect(field).toHaveAttribute("aria-invalid", "true");
  await expect(field).toBeFocused();
  await expect(field).toBeEditable();
  await page.evaluate(() => window.dangoFixture.setTemplateInspection({ arguments: [], error: null }));
  await field.fill("Plain text");
  await expect(field).toHaveAccessibleDescription("Nothing to fill in");
  await expect(field).toHaveAttribute("aria-invalid", "false");
});
