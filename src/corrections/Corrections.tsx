import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
  if (!last)
    return (
      <div className="lc">
        <p>{t("corrections.expired")}</p>
        <button onClick={() => run("dismiss")}>{t("corrections.close")}</button>
      </div>
    );
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
  return (
    <main className="lc" aria-label={t("corrections.title")}>
      <header>
        <strong>
          {t(
            last.editing
              ? "corrections.edit"
              : last.suggestion
                ? "corrections.prompt"
                : "corrections.ready",
          )}
        </strong>
        <button
          className="lc-close"
          aria-label={t("corrections.close")}
          onClick={() => run("dismiss")}
        >
          ×
        </button>
      </header>
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
          <del>{last.suggestion.from}</del>
          <span>→</span>
          <strong>{last.suggestion.to}</strong>
        </p>
      ) : (
        <p className="lc-hint">
          {t(
            last.observing ? "corrections.observing" : "corrections.manualHint",
          )}
        </p>
      )}
      {message && (
        <p className="lc-message" role="status">
          {t(`corrections.${errorKey}`)}
        </p>
      )}
      <footer>
        {!saved && (last.editing || last.suggestion) && (
          <button
            className="lc-primary"
            disabled={busy || !from.trim() || !to.trim() || from === to}
            onClick={() => run("save", { id: last.id, from, to })}
          >
            {t("corrections.remember")}
          </button>
        )}
        {!last.editing && !saved && (
          <button disabled={busy} onClick={() => run("edit", { id: last.id })}>
            {t("corrections.edit")}
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
          <button onClick={() => run("dismiss")}>
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
