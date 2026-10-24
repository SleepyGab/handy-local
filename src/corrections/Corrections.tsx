import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Sparkles, X, ArrowRight, Check } from "lucide-react";
import { useTranslation } from "react-i18next";
import "./corrections.css";

type Rule = { from: string; to: string };
type Last = {
  id: number;
  original: string;
  current: string;
  suggestion: Rule | null;
  observing: boolean;
  editing: boolean;
};
type State = {
  enabled: boolean;
  rules: Rule[];
  last: Last | null;
  storage_error: boolean;
};
async function action(name: string, payload: object = {}) {
  return invoke<string>("corrections_action", {
    action: name,
    payload: JSON.stringify(payload),
  });
}
function useCorrections() {
  const [state, setState] = useState<State | null>(null);
  useEffect(() => {
    let active = true;
    const subscription = listen<State>("local-corrections-changed", (event) => {
      if (active) setState(event.payload);
    });
    // Subscribe first so a dictation completed during window startup isn't lost.
    subscription
      .then(() => invoke<string>("corrections_state"))
      .then((raw) => {
        if (active) setState(JSON.parse(raw));
      })
      .catch(() => {});
    return () => {
      active = false;
      subscription.then((unlisten) => unlisten());
    };
  }, []);
  return state;
}
export function CorrectionPopup() {
  const { t } = useTranslation();
  const state = useCorrections();
  const last = state?.last;
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    setFrom(last?.suggestion?.from ?? "");
    setTo(last?.suggestion?.to ?? "");
  }, [last?.id, last?.suggestion?.from, last?.suggestion?.to]);
  useEffect(() => {
    setMessage("");
  }, [last?.id]);
  const run = useCallback(async (name: string, data: object = {}) => {
    setBusy(true);
    try {
      const result = await action(name, data);
      if (name === "save") setMessage(result);
      if (name === "copy") setMessage("copied");
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }, []);
  useEffect(() => {
    if (message !== "saved" && message !== "copied") return;
    const timer = window.setTimeout(() => {
      void run("dismiss", { id: last?.id });
    }, 1600);
    return () => window.clearTimeout(timer);
  }, [message, last?.id, run]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") void run("dismiss", { id: last?.id });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [last?.id, run]);
  // The hidden webview never renders a routine completion/unavailable notice.
  if (!last || (!last.editing && !last.suggestion && !message)) return null;
  const saved =
    message === "saved" || message === "saved_copy" || message === "copied";
  const errorKey = [
    "expired",
    "phrase",
    "storage",
    "clipboard",
    "saved",
    "saved_copy",
    "copied",
  ].includes(message)
    ? message
    : "error";
  if (!last.editing && message !== "saved_copy") {
    return (
      <main
        className={`lc lc-capsule ${saved ? "lc-capsule-saved" : ""}`}
        aria-label={t("corrections.prompt")}
      >
        {saved ? (
          <span className="lc-ack" role="status">
            <Check size={14} aria-hidden="true" />
            {t("corrections.remembered")}
          </span>
        ) : (
          <>
            {message ? (
              <span
                className="lc-capsule-error"
                role="status"
                title={t(`corrections.${errorKey}`)}
              >
                {t(`corrections.${errorKey}`)}
              </span>
            ) : (
              <button
                className="lc-capsule-pair"
                aria-label={t("corrections.adjust")}
                title={`${from} → ${to}`}
                disabled={busy}
                onClick={() => run("edit", { id: last.id })}
              >
                <span className="lc-before">{from}</span>
                <ArrowRight size={12} aria-hidden="true" />
                <span className="lc-after">{to}</span>
              </button>
            )}
            <button
              className="lc-remember"
              disabled={busy || !from.trim() || !to.trim() || from === to}
              onClick={() => run("save", { id: last.id, from, to })}
            >
              {t("corrections.rememberPrompt")}
            </button>
            <button
              className="lc-close"
              aria-label={t("corrections.close")}
              onClick={() => run("dismiss", { id: last.id })}
            >
              <X size={11} aria-hidden="true" />
            </button>
          </>
        )}
      </main>
    );
  }
  return (
    <main
      className={`lc lc-card ${last.editing ? "lc-editor" : "lc-prompt"}`}
      aria-label={t("corrections.title")}
    >
      <header>
        <span className="lc-mark" aria-hidden="true">
          {saved ? <Check size={16} /> : <Sparkles size={16} />}
        </span>
        <strong>
          {t(
            last.editing
              ? "corrections.edit"
              : last.suggestion
                ? "corrections.prompt"
                : "corrections.title",
          )}
        </strong>
        <button
          className="lc-close"
          aria-label={t("corrections.close")}
          onClick={() => run("dismiss", { id: last.id })}
        >
          <X size={15} aria-hidden="true" />
        </button>
      </header>
      <div className="lc-body">
        {last.editing ? (
          <>
            <p className="lc-hint">{t("corrections.selectHint")}</p>
            <textarea
              className="lc-original"
              readOnly
              value={last.original}
              aria-label={t("corrections.original")}
              onSelect={(event) => {
                const field = event.currentTarget;
                const selection = field.value.slice(
                  field.selectionStart,
                  field.selectionEnd,
                );
                if (selection) {
                  setFrom(selection);
                  setMessage("");
                }
              }}
            />
            <div className="lc-fields">
              <label>
                {t("corrections.from")}
                <input
                  value={from}
                  maxLength={64}
                  onChange={(e) => {
                    setFrom(e.target.value);
                    setMessage("");
                  }}
                />
              </label>
              <span>→</span>
              <label>
                {t("corrections.to")}
                <input
                  value={to}
                  maxLength={64}
                  onChange={(e) => {
                    setTo(e.target.value);
                    setMessage("");
                  }}
                />
              </label>
            </div>
            <p className="lc-hint">{t("corrections.exactHint")}</p>
            {Array.from(from).length === 1 && (
              <p className="lc-warning">{t("corrections.singleChar")}</p>
            )}
          </>
        ) : last.suggestion ? (
          <p className="lc-pair">
            <del title={last.suggestion.from}>{last.suggestion.from}</del>
            <ArrowRight size={15} aria-hidden="true" />
            <strong title={last.suggestion.to}>{last.suggestion.to}</strong>
          </p>
        ) : null}
        {message && (
          <p className="lc-message" role="status">
            {t(`corrections.${errorKey}`)}
          </p>
        )}
      </div>
      <footer>
        {!last.editing && !saved && (
          <button
            className="lc-quiet"
            disabled={busy}
            onClick={() => run("edit", { id: last.id })}
          >
            {t("corrections.adjust")}
          </button>
        )}
        {!saved && (last.editing || last.suggestion) && (
          <button
            className="lc-primary"
            disabled={busy || !from.trim() || !to.trim() || from === to}
            onClick={() => run("save", { id: last.id, from, to })}
          >
            {t("corrections.remember")}
          </button>
        )}
        {message === "saved_copy" && (
          <button
            className="lc-primary"
            onClick={() => run("copy", { corrected: last.current })}
          >
            {t("corrections.copy")}
          </button>
        )}
        {saved && (
          <button onClick={() => run("dismiss", { id: last.id })}>
            {t("corrections.done")}
          </button>
        )}
      </footer>
    </main>
  );
}
export function CorrectionsSettings() {
  const { t } = useTranslation();
  const state = useCorrections();
  const [error, setError] = useState("");
  const run = async (name: string, payload: object) => {
    try {
      await action(name, payload);
      setError("");
    } catch {
      setError(t("corrections.error"));
    }
  };
  return (
    <section className="lc lc-settings">
      <h2>{t("corrections.title")}</h2>
      <p>{t("corrections.privacy")}</p>
      <label className="lc-toggle">
        <input
          type="checkbox"
          checked={state?.enabled ?? false}
          onChange={(e) => run("toggle", { enabled: e.target.checked })}
        />
        {t("corrections.enabled")}
      </label>
      <button
        disabled={!state?.last}
        onClick={() => run("edit", { id: state?.last?.id })}
      >
        {t("corrections.edit")}
      </button>
      <p className="lc-hint">{t("corrections.menuHint")}</p>
      <p className="lc-hint">{t("corrections.retention")}</p>
      {(error || state?.storage_error) && (
        <p role="alert">{error || t("corrections.storage")}</p>
      )}
      <h3>{t("corrections.dictionary")}</h3>
      <p className="lc-hint">{t("corrections.exactHint")}</p>
      {!state?.rules.length && <p>{t("corrections.empty")}</p>}
      {state?.rules.map((rule) => (
        <div className="lc-rule" key={rule.from}>
          <span>
            {rule.from}
            <span className="lc-arrow">→</span>
            <strong>{rule.to}</strong>
          </span>
          <button onClick={() => run("delete", { from: rule.from })}>
            {t("corrections.delete")}
          </button>
        </div>
      ))}
    </section>
  );
}
