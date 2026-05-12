import { expect, test } from "@playwright/test";

test("renders the Phase 1 core notes shell", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("o-note")).toBeVisible();
  await expect(page.getByRole("button", { name: "New MD" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Note content" })).toBeVisible();
  await expect(page.getByText("PHASE 1", { exact: true })).toBeVisible();
  await expect(page.getByText(/Metadata lists stay separate from note bodies/)).toBeVisible();
});
