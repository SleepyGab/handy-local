// Development-only IPC fixture, excluded from application entry points.
import React from "react";
import ReactDOM from "react-dom/client";
import { Toaster } from "sonner";
import { CloudPolishSettings } from "../../src/components/settings/CloudPolishSettings";
import { HistorySettings } from "../../src/components/settings/history/HistorySettings";
import "../../src/App.css";

const query = new URLSearchParams(location.search);
const callbacks = new Map<number, (payload: unknown) => void>();
const listeners = new Map<string, number>();
let next = 1;
const original = "嗯费用有两笔每月89元另外一次性39元";
const polished = "费用有两笔：每月 89 元，另有一次性费用 39 元。";
let savedKey =
  localStorage.getItem("fixture-saved-key") ||
  (query.has("saved") ? "sk-fixture-saved-1234" : "");
const models: Record<string, string> = {
  deepseek: "deepseek-flash",
  openai: "gpt-6-luna",
  anthropic: "claude-haiku-5-5",
  gemini: "gemini-3.5-flash-lite",
};
const state = {
  provider: localStorage.getItem("fixture-provider") || "deepseek",
  mode: localStorage.getItem("fixture-mode") || "general",
  model: "",
  timeout_seconds: Number(localStorage.getItem("fixture-timeout") || 12),
  enabled: Boolean(
    savedKey && localStorage.getItem("fixture-enabled") === "true",
  ),
  key_configured: Boolean(savedKey),
  key_preview: savedKey ? "••••••••" + savedKey.slice(-4) : null,
  last: query.has("last")
    ? {
        original,
        polished: query.has("fallback") ? null : polished,
        fallback: query.has("fallback"),
      }
    : null,
};
state.model =
  localStorage.getItem(`fixture-model-${state.provider}`) ||
  models[state.provider];
if (state.provider !== "deepseek") {
  savedKey = localStorage.getItem(`fixture-key-${state.provider}`) || "";
  state.key_configured = Boolean(savedKey);
  state.key_preview = savedKey ? "••••••••" + savedKey.slice(-4) : null;
  state.enabled = Boolean(
    savedKey && localStorage.getItem("fixture-enabled") === "true",
  );
}
const testState = { clipboard: "", calls: [] as string[] };
Object.assign(window, { polishTest: testState });
Object.defineProperty(navigator, "clipboard", {
  value: {
    async writeText(text: string) {
      if (query.has("copyerror")) throw new Error("clipboard denied");
      testState.clipboard = text;
    },
  },
});
Object.defineProperty(window, "__TAURI_OS_PLUGIN_INTERNALS__", {
  value: { os_type: "macos", platform: "macos", arch: "aarch64" },
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
        key?: string;
        provider?: string;
        mode?: string;
        model?: string;
        timeoutSeconds?: number;
        enabled?: boolean;
        handler?: number;
        event?: string;
      },
    ) {
      testState.calls.push(command);
      if (command === "plugin:event|listen") {
        listeners.set(args.event!, args.handler!);
        return next++;
      }
      if (command === "plugin:event|unlisten") return;
      if (command === "get_app_settings")
        return { app_language: query.get("lang") || "zh" };
      if (command === "cloud_polish_status") return structuredClone(state);
      if (command === "configure_cloud_polish") {
        if (query.has("configerror")) throw "save_failed";
        if (args.provider && args.provider !== state.provider) {
          state.provider = args.provider;
          state.enabled = false;
          localStorage.setItem("fixture-provider", state.provider);
          localStorage.removeItem("fixture-enabled");
          savedKey =
            localStorage.getItem(
              state.provider === "deepseek"
                ? "fixture-saved-key"
                : `fixture-key-${state.provider}`,
            ) ||
            (state.provider === "deepseek" && query.has("saved")
              ? "sk-fixture-saved-1234"
              : "");
          state.key_configured = Boolean(savedKey);
          state.key_preview = savedKey ? "••••••••" + savedKey.slice(-4) : null;
          state.model =
            localStorage.getItem(`fixture-model-${state.provider}`) ||
            models[state.provider];
        }
        if (args.mode) {
          state.mode = args.mode;
          localStorage.setItem("fixture-mode", state.mode);
        }
        if (args.timeoutSeconds !== undefined) {
          state.timeout_seconds = args.timeoutSeconds;
          localStorage.setItem(
            "fixture-timeout",
            String(state.timeout_seconds),
          );
        }
        if (args.model !== undefined) {
          state.model = args.model.trim() || models[state.provider];
          localStorage.setItem(`fixture-model-${state.provider}`, state.model);
        }
        return;
      }
      if (command === "set_cloud_polish_key") {
        if (query.has("saveerror") || (!args.key && query.has("removeerror")))
          throw "save_failed";
        savedKey = args.key || "";
        localStorage.setItem(
          state.provider === "deepseek"
            ? "fixture-saved-key"
            : `fixture-key-${state.provider}`,
          savedKey,
        );
        state.key_configured = Boolean(savedKey);
        state.key_preview = savedKey ? "••••••••" + savedKey.slice(-4) : null;
        if (!savedKey) {
          state.enabled = false;
          localStorage.removeItem("fixture-enabled");
        }
        return;
      }
      if (command === "reveal_cloud_polish_key") {
        if (query.has("revealerror")) throw "reveal_failed";
        if (query.has("slowreveal"))
          await new Promise((resolve) => setTimeout(resolve, 100));
        return savedKey;
      }
      if (command === "set_cloud_polish_enabled") {
        state.enabled = args.enabled!;
        localStorage.setItem("fixture-enabled", String(state.enabled));
        return;
      }
      if (command === "clear_last_cloud_polish") {
        state.last = null;
        return;
      }
      if (command === "get_history_entries")
        return {
          entries: [
            {
              id: 1,
              timestamp: 1792897200,
              file_name: "test.wav",
              title: "test",
              saved: false,
              transcription_text: original,
              post_processed_text: polished,
              post_process_prompt: null,
              post_process_requested: true,
            },
          ],
          has_more: false,
        };
    },
  },
});
Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
  value: { unregisterListener() {} },
});
const { default: i18n } = await import("../../src/i18n");
await i18n.changeLanguage(query.get("lang") || "zh");
ReactDOM.createRoot(document.getElementById("root")!).render(
  <>
    <Toaster />
    {query.has("history") ? <HistorySettings /> : <CloudPolishSettings />}
  </>,
);
