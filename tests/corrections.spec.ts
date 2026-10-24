import { test, expect, Page } from "@playwright/test";

async function mockCorrections(
  page: Page,
  suggestion = false,
  editing = !suggestion,
  observing = true,
) {
  await page.addInitScript(
    ({ suggestion, editing, observing }) => {
      const callbacks = new Map<number, (payload: unknown) => void>();
      let next = 1;
      let listener = 0;
      const state = {
        enabled: true,
        rules: [] as { from: string; to: string }[],
        storage_error: false,
        last: {
          id: 7,
          original: "请联系 Andromida。",
          current: suggestion ? "请联系 Andromeda。" : "请联系 Andromida。",
          suggestion: suggestion
            ? { from: "Andromida", to: "Andromeda" }
            : null,
          observing,
          editing,
        },
      };
      const emit = () =>
        callbacks.get(listener)?.({
          event: "local-corrections-changed",
          id: 1,
          payload: structuredClone(state),
        });
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: {
          transformCallback(callback: (payload: unknown) => void) {
            const id = next++;
            callbacks.set(id, callback);
            return id;
          },
          unregisterCallback(id: number) {
            callbacks.delete(id);
          },
          async invoke(
            command: string,
            args: { handler?: number; action?: string; payload?: string },
          ) {
            if (command === "plugin:event|listen") {
              listener = args.handler!;
              return 1;
            }
            if (command === "plugin:event|unlisten") return;
            if (command === "corrections_state") return JSON.stringify(state);
            if (command === "get_app_settings") return { app_language: "zh" };
            if (command !== "corrections_action") return;
            const payload = JSON.parse(args.payload || "{}");
            if (args.action === "edit") {
              state.last.editing = true;
              state.last.observing = false;
              emit();
              return "ok";
            }
            if (args.action === "save") {
              if (!state.last.original.includes(payload.from)) throw "phrase";
              state.rules.push({ from: payload.from, to: payload.to });
              state.last.current = state.last.original.replaceAll(
                payload.from,
                payload.to,
              );
              state.last.suggestion = null;
              emit();
              return suggestion ? "saved" : "saved_copy";
            }
            if (args.action === "copy") return "ok";
            if (args.action === "dismiss") return "ok";
          },
        },
      });
      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
        value: { unregisterListener() {} },
      });
    },
    { suggestion, editing, observing },
  );
  await page.setViewportSize({ width: 500, height: 430 });
  await page.goto("/src/corrections/index.html");
}

test("editing the last dictation confirms a pair and offers explicit copy fallback", async ({
  page,
}) => {
  await mockCorrections(page);
  await page.getByLabel("听成了").fill("Andromida");
  await page.getByLabel("应该是").fill("Andromeda");
  await page.getByRole("button", { name: "记住", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("无法安全修改原文本框");
  await page.getByRole("button", { name: "复制修正后的文字" }).click();
  await expect(page.getByRole("status")).toContainText("已复制");
});
test("an automatic suggestion keeps the saved acknowledgement after state refresh", async ({
  page,
}) => {
  await mockCorrections(page, true);
  await expect(page.getByText("记住这次纠正？")).toBeVisible();
  await page.getByRole("button", { name: "记住", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("已在本机记住");
  await expect(page.getByRole("button", { name: "完成" })).toBeVisible();
});
test("invalid manual phrase stays editable and is not acknowledged as saved", async ({
  page,
}) => {
  await mockCorrections(page);
  await page.getByLabel("听成了").fill("unrelated");
  await page.getByLabel("应该是").fill("corrected");
  await page.getByRole("button", { name: "记住", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("原文中存在的词");
  await expect(page.getByRole("button", { name: "完成" })).toHaveCount(0);
});

for (const observing of [true, false]) {
  test(`ordinary dictation stays silent (accessible field: ${observing})`, async ({
    page,
  }) => {
    await mockCorrections(page, false, false, observing);
    await expect(page.locator(".lc-card")).toHaveCount(0);
    await expect(page.getByText("文字已填入")).toHaveCount(0);
    await expect(page.getByText("这个文本框暂不支持自动检测")).toHaveCount(0);
  });
}
