import React from "react";
import ReactDOM from "react-dom/client";
import "@/i18n";
import { CorrectionPopup } from "./Corrections";
import "./corrections.css";
import { listen } from "@tauri-apps/api/event";
import {
  applyTheme,
  getStoredTheme,
  syncThemeFromSettings,
} from "@/lib/utils/theme";
import type { Theme } from "@/bindings";
applyTheme(getStoredTheme());
syncThemeFromSettings();
listen<Theme>("theme-changed", (event) => applyTheme(event.payload));
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <CorrectionPopup />
  </React.StrictMode>,
);
