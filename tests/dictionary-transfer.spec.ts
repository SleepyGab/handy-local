import { test, expect } from "@playwright/test";

test("import previews conflicts and defaults to preserving existing corrections", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/dictionary-transfer.html");
  await page.getByRole("button", { name: "导入词典" }).click();
  await expect(
    page.getByText("新增 1 条 · 相同 1 条 · 冲突 1 条"),
  ).toBeVisible();
  await expect(page.getByRole("combobox")).toHaveValue("keep");
  await expect(page.locator(".lc-rule")).toHaveCount(2);
  await page.screenshot({
    path: "../setup/handy-0.9.8/dictionary-import-preview.png",
  });
  await page.getByRole("button", { name: "确认导入" }).click();
  await expect(page.getByRole("status")).toContainText(
    "新增 1 条，替换 0 条，跳过 2 条",
  );
  await expect(page.locator(".lc-rule").first()).toContainText("Andromeda");
  await expect(page.locator(".lc-rule")).toHaveCount(3);
});
test("English import can explicitly replace a conflicting correction", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/dictionary-transfer.html?lang=en");
  await page.getByRole("button", { name: "Import dictionary" }).click();
  await page.getByRole("combobox").selectOption("replace");
  await page
    .getByRole("button", { name: "Import corrections", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText(
    "1 added, 1 replaced, 1 skipped",
  );
  await expect(page.locator(".lc-rule").first()).toContainText("ANDROMEDA");
});
test("cancelling preview or file picker does not change rules or report success", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/dictionary-transfer.html");
  await page.getByRole("button", { name: "导入词典" }).click();
  await page.getByRole("button", { name: "取消", exact: true }).click();
  await expect(page.locator(".lc-import-preview")).toHaveCount(0);
  await expect(page.locator(".lc-rule")).toHaveCount(2);
  await expect(page.getByRole("status")).toHaveCount(0);
  await page.goto("/tests/fixtures/dictionary-transfer.html?cancel=1");
  await page.getByRole("button", { name: "导出词典" }).click();
  await expect(page.getByRole("status")).toHaveCount(0);
});
test("invalid import is explained and export reports the entry count", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/dictionary-transfer.html?error=invalidFile");
  await page.getByRole("button", { name: "导入词典" }).click();
  await expect(page.getByRole("alert")).toContainText("未导入任何词条");
  await expect(page.locator(".lc-rule")).toHaveCount(2);
  await page.getByRole("button", { name: "导出词典" }).click();
  await expect(page.getByRole("status")).toContainText("已导出 2 条");
});
