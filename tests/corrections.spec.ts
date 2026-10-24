import { test, expect, Page } from "@playwright/test";

async function mockCorrections(
  page: Page,
  suggestion = false,
  editing = !suggestion,
  observing = true,
  language = "zh",
) {
  await page.addInitScript(
    ({ suggestion, editing, observing, language }) => {
      const callbacks = new Map<number, (payload: unknown) => void>();
      let next = 1;
      const listeners = new Map<string, number>();
      window.addEventListener("test-language", ((
        event: CustomEvent<string>,
      ) => {
        callbacks.get(listeners.get("app-language-changed")!)?.({
          payload: event.detail,
        });
      }) as EventListener);
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
        callbacks.get(listeners.get("local-corrections-changed")!)?.({
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
            args: {
              handler?: number;
              event?: string;
              action?: string;
              payload?: string;
            },
          ) {
            if (command === "plugin:event|listen") {
              listeners.set(args.event!, args.handler!);
              return 1;
            }
            if (command === "plugin:event|unlisten") return;
            if (command === "corrections_state") return JSON.stringify(state);
            if (command === "get_app_settings")
              return { app_language: language };
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
    { suggestion, editing, observing, language },
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
  await expect(
    page.getByRole("button", { name: "记住？", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "记住？", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("已记住");
  await expect(page.getByRole("button", { name: "完成" })).toHaveCount(0);
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
    await expect(page.locator(".lc-card, .lc-capsule")).toHaveCount(0);
    await expect(page.getByText("文字已填入")).toHaveCount(0);
    await expect(page.getByText("这个文本框暂不支持自动检测")).toHaveCount(0);
  });
}

test("English capsule fits native size and follows live interface language changes", async ({
  page,
}) => {
  await mockCorrections(page, true, false, true, "en");
  await page.setViewportSize({ width: 414, height: 58 });
  await expect(
    page.getByRole("button", { name: "Add?", exact: true }),
  ).toBeVisible();
  const capsule = page.locator(".lc-capsule");
  await expect(capsule).toHaveCSS("height", "42px");
  await expect(
    page.getByRole("button", { name: "Dismiss", exact: true }),
  ).toBeInViewport();
  await page.screenshot({ path: "../setup/handy-0.9.8/capsule-en.png" });
  await page.evaluate(() =>
    window.dispatchEvent(new CustomEvent("test-language", { detail: "zh" })),
  );
  await expect(
    page.getByRole("button", { name: "记住？", exact: true }),
  ).toBeVisible();
  await page.screenshot({ path: "../setup/handy-0.9.8/capsule-zh.png" });
  await page.evaluate(() =>
    window.dispatchEvent(new CustomEvent("test-language", { detail: "en" })),
  );
  await page.getByRole("button", { name: "Add?", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Added");
});

test("clicking the phrase opens the full editor", async ({ page }) => {
  await mockCorrections(page, true);
  await page.getByRole("button", { name: "调整词条", exact: true }).click();
  await expect(page.getByLabel("听成了")).toHaveValue("Andromida");
  await expect(page.getByLabel("应该是")).toHaveValue("Andromeda");
});
