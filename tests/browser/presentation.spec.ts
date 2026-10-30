import { expect, test } from "@playwright/test";
import { detailTree } from "../fixtures/data";
import type {} from "../fixtures/main";

test("presentation: every view fills the window with the footer at the bottom", async ({ page }) => {
  for (const scene of ["root", "list", "form", "detail"]) {
    await page.goto(`/tests/fixtures/?scene=${scene}`);
    await expect(page.getByText("Dango", { exact: true })).toBeVisible();
    const shell = (await page.locator("main").boundingBox())!;
    const footer = (await page.locator("footer").boundingBox())!;
    expect(shell).toEqual({ x: 0, y: 0, ...page.viewportSize()! });
    expect(shell.y + shell.height - (footer.y + footer.height)).toBeLessThanOrEqual(2);
  }
});

test("presentation: a newer protocol version asks for an update", async ({ page }) => {
  await page.goto("/tests/fixtures/?scene=detail");
  await expect(page.getByText("Dango", { exact: true })).toBeVisible();
  await page.evaluate((tree) => window.dangoFixture.replace({ ...tree, protocolVersion: 99 }), detailTree);
  await expect(page.getByText("This view needs a newer version of Dango.")).toBeVisible();
});

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
