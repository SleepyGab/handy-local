import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Download, Upload } from "lucide-react";
import { useTranslation } from "react-i18next";

type Preview = {
  token: number;
  count: number;
  added: number;
  duplicates: number;
  conflicts: { from: string; current: string; incoming: string }[];
};
type Result = {
  status: "cancelled" | "preview" | "exported" | "imported";
  preview?: Preview;
  count?: number;
  added?: number;
  replaced?: number;
  skipped?: number;
};
const errorKeys = new Set([
  "invalidFile",
  "unsupportedFile",
  "ambiguousFile",
  "fileLimit",
  "readFile",
  "writeFile",
  "liveFile",
  "staleImport",
  "backupFailed",
  "transferBusy",
  "storage",
]);

export function DictionaryTransfer({ disabled }: { disabled: boolean }) {
  const { t } = useTranslation();
  const [preview, setPreview] = useState<Preview | null>(null);
  const [policy, setPolicy] = useState("keep");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [result, setResult] = useState<Result | null>(null);
  const transfer = async (operation: string) => {
    setBusy(true);
    setError("");
    setResult(null);
    try {
      const raw = await invoke<string>("corrections_transfer", {
        operation,
        payload: JSON.stringify({ token: preview?.token ?? 0, policy }),
      });
      const response: Result = JSON.parse(raw);
      if (response.status === "preview" && response.preview) {
        setPreview(response.preview);
        setPolicy("keep");
      } else {
        setPreview(null);
        if (response.status !== "cancelled") setResult(response);
      }
    } catch (cause) {
      const code = String(cause);
      setError(errorKeys.has(code) ? code : "transferError");
      if (code === "staleImport") setPreview(null);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="lc-transfer" aria-busy={busy}>
      <div className="lc-dictionary-heading">
        <h3>{t("corrections.dictionary")}</h3>
        <div className="lc-transfer-actions">
          <button
            disabled={disabled || busy || !!preview}
            onClick={() => transfer("export")}
          >
            <Download size={14} aria-hidden="true" />
            {t("corrections.exportDictionary")}
          </button>
          <button
            disabled={disabled || busy || !!preview}
            onClick={() => transfer("import")}
          >
            <Upload size={14} aria-hidden="true" />
            {t("corrections.importDictionary")}
          </button>
        </div>
      </div>
      <p className="lc-hint">{t("corrections.transferHint")}</p>
      {error && (
        <p className="lc-transfer-error" role="alert">
          {t(`corrections.${error}`)}
        </p>
      )}
      {result && (
        <p className="lc-message" role="status">
          {t(`corrections.${result.status}`, { ...result })}
        </p>
      )}
      {preview && (
        <section
          className="lc-import-preview"
          aria-label={t("corrections.importPreview")}
        >
          <strong>{t("corrections.importPreview")}</strong>
          <p>
            {t("corrections.importSummary", {
              added: preview.added,
              duplicates: preview.duplicates,
              conflicts: preview.conflicts.length,
            })}
          </p>
          {preview.count === 0 && <p>{t("corrections.emptyImport")}</p>}
          {!!preview.conflicts.length && (
            <>
              <div className="lc-conflicts">
                {preview.conflicts.map((conflict) => (
                  <div className="lc-conflict" key={conflict.from}>
                    <strong>{conflict.from}</strong>
                    <span>
                      {t("corrections.currentVersion", {
                        word: conflict.current,
                      })}
                    </span>
                    <span>
                      {t("corrections.importedVersion", {
                        word: conflict.incoming,
                      })}
                    </span>
                  </div>
                ))}
              </div>
              <label className="lc-conflict-choice">
                {t("corrections.conflictPolicy")}
                <select
                  value={policy}
                  disabled={busy}
                  onChange={(event) => setPolicy(event.target.value)}
                >
                  <option value="keep">{t("corrections.keepExisting")}</option>
                  <option value="replace">
                    {t("corrections.useImported")}
                  </option>
                </select>
              </label>
            </>
          )}
          <p className="lc-hint">{t("corrections.importBackupHint")}</p>
          <div className="lc-transfer-actions lc-import-actions">
            <button disabled={busy} onClick={() => transfer("cancel")}>
              {t("corrections.cancelImport")}
            </button>
            <button
              className="lc-primary"
              disabled={busy || preview.count === 0}
              onClick={() => transfer("apply")}
            >
              {t("corrections.confirmImport")}
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
