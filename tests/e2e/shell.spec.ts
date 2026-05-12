import { expect, test } from "@playwright/test";

test("renders the Phase 0 shell", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("o-note")).toBeVisible();
  await expect(page.getByRole("tab", { name: "Report" })).toBeVisible();
  await expect(page.getByText("Fast local notes with sandboxed HTML artifacts.")).toBeVisible();
  await expect(page.getByText("SQLite first")).toBeVisible();
});
