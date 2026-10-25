// Dev-only fixture: no operating-system permissions are read or changed.
import React, { useState } from "react";
import ReactDOM from "react-dom/client";
import { Toaster } from "sonner";
import AccessibilityOnboarding from "../../src/components/onboarding/AccessibilityOnboarding";
import AccessibilityPermissions from "../../src/components/AccessibilityPermissions";
import "../../src/App.css";

const query = new URLSearchParams(location.search);
const os = query.get("os") || "macos";
const state = {
  accessibility: query.has("granted"),
  microphone: !query.has("nomic"),
  error: query.has("error"),
  hang: query.has("hang"),
  initError: query.has("initerror"),
  completed: 0,
  calls: [] as string[],
  checksInFlight: 0,
  maxConcurrent: 0,
};
Object.assign(window, { permissionTest: state });
Object.defineProperty(window, "__TAURI_OS_PLUGIN_INTERNALS__", {
  value: { os_type: os, platform: os, arch: "aarch64" },
});
Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {
    async invoke(command: string) {
      state.calls.push(command);
      if (command === "get_app_settings")
        return { app_language: query.get("lang") || "en" };
      if (
        command === "plugin:macos-permissions|check_accessibility_permission"
      ) {
        if (state.error) throw new Error("Permission service unavailable");
        if (state.hang) return new Promise(() => {});
        state.checksInFlight++;
        state.maxConcurrent = Math.max(
          state.maxConcurrent,
          state.checksInFlight,
        );
        if (query.has("slow"))
          await new Promise((resolve) => setTimeout(resolve, 2500));
        state.checksInFlight--;
        return state.accessibility;
      }
      if (command === "plugin:macos-permissions|check_microphone_permission")
        return state.microphone;
      if (command === "get_windows_microphone_permission_status")
        return {
          supported: true,
          overall_access: state.microphone ? "allowed" : "denied",
        };
      if (command === "initialize_enigo" && state.initError)
        throw "Initialization failed";
      if (
        command === "get_available_microphones" ||
        command === "get_available_output_devices"
      )
        return [];
    },
  },
});
const { default: i18n } = await import("../../src/i18n");
await i18n.changeLanguage(query.get("lang") || "en");
function Fixture() {
  const [complete, setComplete] = useState(false);
  if (query.has("banner")) return <AccessibilityPermissions />;
  if (complete) return <div data-testid="complete">Complete</div>;
  return (
    <AccessibilityOnboarding
      preview={query.has("preview")}
      onComplete={() => {
        state.completed++;
        setComplete(true);
      }}
    />
  );
}
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Toaster />
    <Fixture />
  </React.StrictMode>,
);
