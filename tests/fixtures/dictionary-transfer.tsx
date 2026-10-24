// Development-only IPC fixture; not an application build entry point.
import React from "react";
import ReactDOM from "react-dom/client";
import { CorrectionsSettings } from "../../src/corrections/Corrections";

const query = new URLSearchParams(location.search);
const callbacks = new Map<number, (value: unknown) => void>();
const listeners = new Map<string, number>();
let next = 1;
const state = {
  enabled: true,
  last: null,
  storage_error: false,
  rules: [
    { from: "Andromida", to: "Andromeda" },
    { from: "Q Wen", to: "Qwen" },
  ],
};
const preview = {
  token: 1,
  count: 3,
  added: 1,
  duplicates: 1,
  conflicts: [
    { from: "Andromida", current: "Andromeda", incoming: "ANDROMEDA" },
  ],
};
Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {
    transformCallback(callback: (value: unknown) => void) {
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
        event?: string;
        handler?: number;
        operation?: string;
        payload?: string;
      },
    ) {
      if (command === "plugin:event|listen") {
        listeners.set(args.event!, args.handler!);
        return 1;
      }
      if (command === "get_app_settings")
        return { app_language: query.get("lang") || "zh" };
      if (command === "corrections_state") return JSON.stringify(state);
      if (command !== "corrections_transfer") return;
      if (query.has("cancel")) return JSON.stringify({ status: "cancelled" });
      if (args.operation === "export")
        return JSON.stringify({
          status: "exported",
          count: state.rules.length,
        });
      if (args.operation === "import") {
        if (query.has("error")) throw query.get("error");
        return JSON.stringify({ status: "preview", preview });
      }
      if (args.operation === "cancel")
        return JSON.stringify({ status: "cancelled" });
      if (args.operation === "apply") {
        const { policy } = JSON.parse(args.payload!);
        state.rules.push({ from: "Handee", to: "Handy" });
        if (policy === "replace") state.rules[0].to = "ANDROMEDA";
        callbacks.get(listeners.get("local-corrections-changed")!)?.({
          payload: structuredClone(state),
        });
        return JSON.stringify({
          status: "imported",
          added: 1,
          replaced: policy === "replace" ? 1 : 0,
          skipped: policy === "replace" ? 1 : 2,
        });
      }
    },
  },
});
Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
  value: { unregisterListener() {} },
});
await import("../../src/i18n");
ReactDOM.createRoot(document.getElementById("root")!).render(
  <CorrectionsSettings />,
);
