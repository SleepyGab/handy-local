import { expect, test } from "@playwright/test";

test("saving a masked key does not opt in; removing it disables polishing", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html");
  const toggle = page.getByRole("checkbox", {
    name: "使用 DeepSeek 润色转录文字",
  });
  await expect(toggle).not.toBeChecked();
  await expect(toggle).toBeDisabled();
  const key = page.getByLabel("DeepSeek API key");
  await expect(key).toHaveAttribute("type", "password");
  await key.fill("sk-test-only-not-a-real-api-key");
  await page.getByRole("button", { name: "保存 key", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("Key 已保存");
  await expect(key).toHaveValue("");
  await expect(toggle).toBeEnabled();
  await expect(toggle).not.toBeChecked();
  await toggle.check();
  await expect(toggle).toBeChecked();
  await page.getByRole("button", { name: "移除 key" }).click();
  await expect(toggle).not.toBeChecked();
  await expect(toggle).toBeDisabled();
});

test("a rejected save stays disabled and does not claim success", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?saveerror");
  await page
    .getByLabel("DeepSeek API key")
    .fill("sk-test-only-not-a-real-api-key");
  await page.getByRole("button", { name: "保存 key", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("保存失败");
  await expect(page.getByRole("checkbox")).not.toBeChecked();
  await expect(page.getByRole("checkbox")).toBeDisabled();
});

test("latest result preserves and restores the original without another cloud request", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?last");
  await expect(
    page.getByText("费用有两笔：每月 89 元，另有一次性费用 39 元。", {
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: "恢复原文" }).click();
  await expect(page.getByRole("status")).toContainText("原文已复制");
  expect(
    await page.evaluate(
      () =>
        (window as unknown as { polishTest: { clipboard: string } }).polishTest
          .clipboard,
    ),
  ).toBe("嗯费用有两笔每月89元另外一次性39元");
  await page.getByRole("button", { name: "清除最近结果" }).click();
  await expect(page.getByText("本次启动后还没有整理结果。")).toBeVisible();
});

test("failed polishing and denied clipboard show accurate recovery states", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?last&fallback&copyerror");
  await expect(page.getByText("暂时无法润色，已使用原文。")).toBeVisible();
  await expect(page.getByRole("button", { name: "复制润色结果" })).toHaveCount(
    0,
  );
  await page.getByRole("button", { name: "恢复原文" }).click();
  await expect(page.getByRole("alert")).toContainText("复制失败");
});

test("history shows both versions and copying follows the selected text", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?history");
  await expect(
    page.getByText("费用有两笔：每月 89 元，另有一次性费用 39 元。", {
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: "恢复原文" }).click();
  await expect(
    page.getByText("嗯费用有两笔每月89元另外一次性39元", { exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () =>
        (window as unknown as { polishTest: { clipboard: string } }).polishTest
          .clipboard,
    ),
  ).toBe("嗯费用有两笔每月89元另外一次性39元");
  await page.getByRole("button", { name: "查看润色结果" }).click();
  await expect(
    page.getByText("费用有两笔：每月 89 元，另有一次性费用 39 元。", {
      exact: true,
    }),
  ).toBeVisible();
});

test("English settings remain localized and readable", async ({ page }) => {
  await page.goto("/tests/fixtures/cloud-polish.html?lang=en&last");
  await expect(
    page.getByRole("heading", { name: "Text polishing", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Restore original" }),
  ).toBeVisible();
  await expect(page.getByRole("checkbox")).not.toBeChecked();
});

test("saved key is identifiable, revealed only on demand, and hidden when leaving the window", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?saved");
  const saved = page.getByLabel("已保存的 DeepSeek key");
  await expect(saved).toHaveValue("••••••••1234");
  await expect(saved).toHaveAttribute("readonly", "");
  expect(
    await page.evaluate(
      () =>
        (window as unknown as { polishTest: { calls: string[] } }).polishTest
          .calls,
    ),
  ).not.toContain("reveal_cloud_polish_key");
  await page.getByRole("button", { name: "显示 key", exact: true }).click();
  await expect(saved).toHaveValue("sk-fixture-saved-1234");
  await page.getByRole("button", { name: "隐藏 key", exact: true }).click();
  await expect(saved).toHaveValue("••••••••1234");
  await page.getByRole("button", { name: "显示 key", exact: true }).click();
  await expect(saved).toHaveValue("sk-fixture-saved-1234");
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  await expect(saved).toHaveValue("••••••••1234");
});

test("replacing a key updates the saved identity; removing it stays removed after reload", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html");
  const key = page.getByLabel("DeepSeek API key", { exact: true });
  await key.fill("sk-fixture-first-1234");
  await page.getByRole("button", { name: "保存 key", exact: true }).click();
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveValue(
    "••••••••1234",
  );
  await page.getByRole("checkbox").check();
  await key.fill("sk-fixture-second-5678");
  await page.getByRole("button", { name: "保存 key", exact: true }).click();
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveValue(
    "••••••••5678",
  );
  await expect(page.getByRole("checkbox")).toBeChecked();
  await page.reload();
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveValue(
    "••••••••5678",
  );
  await page.getByRole("button", { name: "移除 key" }).click();
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveCount(0);
  await expect(page.getByRole("checkbox")).not.toBeChecked();
  await page.reload();
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveCount(0);
  await expect(page.getByRole("checkbox")).toBeDisabled();
  await expect(page.getByRole("button", { name: "移除 key" })).toHaveCount(0);
});

test("a failed removal keeps the saved key visible and does not claim it was removed", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?saved&removeerror");
  await page.getByRole("button", { name: "移除 key" }).click();
  await expect(page.getByRole("alert")).toContainText("保存失败");
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveValue(
    "••••••••1234",
  );
  await expect(page.getByRole("button", { name: "移除 key" })).toBeEnabled();
});

test("a failed reveal stays masked and a late reveal does not undo hiding", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?saved&revealerror");
  await page.getByRole("button", { name: "显示 key", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("无法显示");
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveValue(
    "••••••••1234",
  );
  await page.goto("/tests/fixtures/cloud-polish.html?saved&slowreveal");
  await page.getByRole("button", { name: "显示 key", exact: true }).click();
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  await expect(
    page.getByRole("button", { name: "显示 key", exact: true }),
  ).toBeEnabled();
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveValue(
    "••••••••1234",
  );
});
