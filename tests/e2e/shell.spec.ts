import { expect, test } from "@playwright/test";

test("renders the Phase 5 artifact shell", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("o-note")).toBeVisible();
  await expect(page.getByRole("button", { name: "New MD" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Note content" })).toBeVisible();
  await expect(page.getByText("PHASE 5", { exact: true })).toBeVisible();
  await expect(page.getByText(/artifact templates make plans/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Planning" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Import path" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Export path" })).toBeVisible();
  await page.getByRole("textbox", { name: "Search notes" }).fill("Obsidian");
  await page.getByRole("button", { name: "Open search result Obsidian import notes" }).click();
  await expect(page.getByRole("textbox", { name: "Note content" })).toHaveValue(/# Obsidian import notes/);
  await page.getByRole("tab", { name: "Preview" }).click();
  await page.getByRole("button", { name: /HTML artifact workflow/ }).click();
  await page.getByRole("tab", { name: "Preview" }).click();
  const htmlPreview = page.getByLabel("HTML artifact preview");
  await expect(htmlPreview).toBeVisible();
  await expect(htmlPreview).toHaveAttribute("sandbox", "allow-scripts");
  await page.getByRole("tab", { name: "Compare" }).click();
  await expect(page.getByLabel("Artifact comparison")).toBeVisible();
});
