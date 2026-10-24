import React from "react";
import ReactDOM from "react-dom/client";
import "@/i18n";
import { CorrectionPopup } from "./Corrections";
import "./corrections.css";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <CorrectionPopup />
  </React.StrictMode>,
);
