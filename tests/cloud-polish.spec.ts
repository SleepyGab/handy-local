import { expect, test } from "@playwright/test";

test("provider keys stay separate and switching providers always turns polishing off", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?saved");
  const provider = page.getByLabel("AI 服务平台");
  await expect(provider).toHaveValue("deepseek");
  await expect(provider.locator("option").first()).toHaveAttribute(
    "value",
    "deepseek",
  );
  await page.getByRole("checkbox").check();
  await page.getByRole("button", { name: "显示 key", exact: true }).click();
  await provider.selectOption("openai");
  await expect(page.getByRole("checkbox")).not.toBeChecked();
  await expect(page.getByRole("checkbox")).toBeDisabled();
  await expect(page.getByLabel("已保存的 OpenAI key")).toHaveCount(0);
  await expect(page.getByLabel("模型", { exact: true })).toHaveValue(
    "gpt-6-luna",
  );
  await page
    .getByLabel("OpenAI API key", { exact: true })
    .fill("sk-openai-fixture-5678");
  await page.getByRole("button", { name: "保存 key", exact: true }).click();
  await expect(page.getByRole("checkbox")).not.toBeChecked();
  await page.getByRole("checkbox").check();
  await provider.selectOption("deepseek");
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveValue(
    "••••••••1234",
  );
  await expect(page.getByRole("checkbox")).not.toBeChecked();
  await page.getByRole("button", { name: "移除 key" }).click();
  await provider.selectOption("openai");
  await expect(page.getByLabel("已保存的 OpenAI key")).toHaveValue(
    "••••••••5678",
  );
  await page.reload();
  await expect(provider).toHaveValue("openai");
  await expect(page.getByRole("checkbox")).not.toBeChecked();
});

test("writing mode and model persist without enabling polishing", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html");
  await page.getByLabel("整理模式", { exact: true }).selectOption("meeting");
  await expect(page.getByText(/整理讨论、决定和后续行动/)).toBeVisible();
  await page.getByLabel("整理模式", { exact: true }).selectOption("journal");
  await expect(page.getByText(/把口述想法整理成自然段落/)).toBeVisible();
  await page.getByLabel("模型", { exact: true }).fill("another-flash-model");
  await page.getByRole("button", { name: "保存模型", exact: true }).click();
  await page.reload();
  await expect(page.getByLabel("整理模式", { exact: true })).toHaveValue(
    "journal",
  );
  await expect(page.getByLabel("模型", { exact: true })).toHaveValue(
    "another-flash-model",
  );
  await expect(page.getByRole("checkbox")).not.toBeChecked();
  await page.getByLabel("模型", { exact: true }).fill("");
  await page.getByRole("button", { name: "保存模型", exact: true }).click();
  await expect(page.getByLabel("模型", { exact: true })).toHaveValue(
    "deepseek-flash",
  );
});

test("a failed provider change keeps the existing provider and key", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?saved&configerror");
  await page.getByLabel("AI 服务平台").selectOption("anthropic");
  await expect(page.getByRole("alert")).toContainText("保存失败");
  await expect(page.getByLabel("AI 服务平台")).toHaveValue("deepseek");
  await expect(page.getByLabel("已保存的 DeepSeek key")).toHaveValue(
    "••••••••1234",
  );
});

test("polishing wait limit is separate from recording and persists across modes and providers", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html?saved");
  const timeout = page.getByLabel("润色最长等待时间");
  await expect(timeout).toHaveValue("12");
  await expect(timeout.locator("option")).toHaveText([
    "12 秒（默认）",
    "30 秒",
    "60 秒",
    "120 秒",
  ]);
  await expect(page.getByText(/不是录音时长限制/)).toBeVisible();
  await timeout.selectOption("60");
  await expect(
    page.getByText("已保存润色等待时间。", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("checkbox")).not.toBeChecked();
  await page.getByLabel("整理模式", { exact: true }).selectOption("meeting");
  await page.getByLabel("AI 服务平台").selectOption("openai");
  await page.reload();
  await expect(timeout).toHaveValue("60");
  await expect(page.getByRole("checkbox")).not.toBeChecked();
});

test("a failed timeout change retains the saved budget", async ({ page }) => {
  await page.goto("/tests/fixtures/cloud-polish.html?configerror");
  const timeout = page.getByLabel("润色最长等待时间");
  await timeout.selectOption("120");
  await expect(page.getByRole("alert")).toContainText("保存失败");
  await expect(timeout).toHaveValue("12");
  await page.reload();
  await expect(timeout).toHaveValue("12");
});

test("saving a masked key does not opt in; removing it disables polishing", async ({
  page,
}) => {
  await page.goto("/tests/fixtures/cloud-polish.html");
  const toggle = page.getByRole("checkbox", {
    name: "开启文字润色",
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
