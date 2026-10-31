import { expect, test } from "@playwright/test";
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
