import React from "react";
import ReactDOM from "react-dom/client";
import { CorrectionsSettings } from "../../src/corrections/Corrections";
import "../../src/App.css";
const query = new URLSearchParams(location.search);
let words: string[] = JSON.parse(
  localStorage.getItem("test-vocabulary") || '["星澜科技"]',
);
const callbacks = new Map<number, (value: unknown) => void>();
const listeners = new Map<string, number>();
let next = 1;
const state = {
  enabled: true,
  rules: [{ from: "TypePlus", to: "Typeless" }],
  last: null,
  storage_error: false,
};
Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {
    transformCallback(fn: (value: unknown) => void) {
      const id = next++;
      callbacks.set(id, fn);
      return id;
    },
    unregisterCallback(id: number) {
      callbacks.delete(id);
    },
    async invoke(
      command: string,
      args: {
        event?: string;
        handler?: number;
        words?: string[];
        action?: string;
        payload?: string;
      },
    ) {
      if (command === "plugin:event|listen") {
        listeners.set(args.event!, args.handler!);
        return next++;
      }
      if (command === "get_app_settings")
        return { app_language: query.get("lang") || "zh", custom_words: words };
      if (command === "update_custom_words") {
        if (query.has("saveerror")) throw "storage";
        if (
          args.words!.some(
            (w) => Array.from(w).length > 50 || /[<>\x00-\x1f]/.test(w),
          )
        )
          throw "invalidWord";
        if (
          args.words!.length > 32 ||
          args.words!.reduce((n, w) => n + Array.from(w).length, 0) > 400
        )
          throw "wordLimit";
        words = args.words!;
        localStorage.setItem("test-vocabulary", JSON.stringify(words));
        return;
      }
      if (command === "corrections_state") return JSON.stringify(state);
      if (command === "corrections_action") {
        const p = JSON.parse(args.payload || "{}");
        if (args.action === "add") {
          if (state.rules.some((r) => r.from === p.from)) throw "duplicateRule";
          state.rules.push({ from: p.from, to: p.to });
        }
        if (args.action === "delete")
          state.rules = state.rules.filter((r) => r.from !== p.from);
        if (args.action === "toggle") state.enabled = p.enabled;
        callbacks.get(listeners.get("local-corrections-changed")!)?.({
          payload: structuredClone(state),
        });
        return "ok";
      }
    },
  },
});
Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
  value: { unregisterListener() {} },
});
const { default: i18n } = await import("../../src/i18n");
await i18n.changeLanguage(query.get("lang") || "zh");
ReactDOM.createRoot(document.getElementById("root")!).render(
  <CorrectionsSettings />,
);
