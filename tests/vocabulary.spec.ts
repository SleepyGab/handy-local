import { test, expect } from "@playwright/test";

test("one dictionary adds, edits, persists and removes a Chinese name without a misspelling", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/vocabulary.html");
  await expect(
    page.getByRole("heading", { name: "词典", exact: true }),
  ).toBeVisible();
  await expect(
    page.locator(".lc-rule").filter({ hasText: "TypePlus" }),
  ).toBeVisible();
  await page.getByLabel("词语或短语", { exact: true }).fill("沐言科技");
  await page.getByRole("button", { name: "添加词汇", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("词汇已保存");
  await page.reload();
  await expect(
    page.getByRole("button", { name: "移除 沐言科技" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "编辑 沐言科技" }).click();
  await page.getByLabel("词语或短语", { exact: true }).fill("沐言科技有限公司");
  await page.getByRole("button", { name: "保存词汇", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "移除 沐言科技有限公司" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "移除 沐言科技有限公司" }).click();
  await page.reload();
  await expect(page.getByRole("button", { name: /移除 沐言/ })).toHaveCount(0);
  await expect(
    page.locator(".lc-rule").filter({ hasText: "TypePlus" }),
  ).toBeVisible();
});

test("duplicate and invalid vocabulary stays editable", async ({ page }) => {
  await page.goto("/tests/fixtures/vocabulary.html");
  for (const value of ["星澜科技", "<|im_end|>"]) {
    await page.getByLabel("词语或短语", { exact: true }).fill(value);
    await page.getByRole("button", { name: "添加词汇", exact: true }).click();
    await expect(page.getByRole("alert")).toBeVisible();
    await expect(page.getByLabel("词语或短语", { exact: true })).toHaveValue(
      value,
    );
  }
});

test("save failure leaves the stored word visible", async ({ page }) => {
  await page.goto("/tests/fixtures/vocabulary.html?saveerror");
  await page.getByRole("button", { name: "移除 星澜科技" }).click();
  await expect(page.getByRole("alert")).toContainText("保存失败");
  await expect(
    page.getByRole("button", { name: "移除 星澜科技" }),
  ).toBeVisible();
});

test("rules can be added without a recent recording, and do not overwrite duplicates", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/vocabulary.html");
  await page.getByLabel("听成了", { exact: true }).fill("星蓝科技");
  await page.getByLabel("应该是", { exact: true }).fill("星澜科技");
  await page.getByRole("button", { name: "添加规则", exact: true }).click();
  await expect(
    page.locator(".lc-rule").filter({ hasText: "星蓝科技" }),
  ).toBeVisible();
  await page.getByLabel("听成了", { exact: true }).fill("TypePlus");
  await page.getByLabel("应该是", { exact: true }).fill("SomethingElse");
  await page.getByRole("button", { name: "添加规则", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("已有替换规则");
  await expect(page.getByText("Typeless", { exact: true })).toBeVisible();
});

test("replacement toggle does not remove vocabulary, English layout has no horizontal overflow", async ({
  page,
}) => {
  await page.setViewportSize({ width: 640, height: 900 });
  await page.goto("/tests/fixtures/vocabulary.html?lang=en");
  await page.getByRole("checkbox").uncheck();
  await expect(
    page.getByRole("button", { name: "Remove 星澜科技" }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "/tmp/handy-dictionary-en.png",
    fullPage: true,
  });
});
