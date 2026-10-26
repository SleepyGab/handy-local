import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import type { CloudPolishStatus } from "@/bindings";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { copyToClipboard } from "./history/clipboard";

export const CloudPolishSettings = () => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<CloudPolishStatus | null>(null);
  const [key, setKey] = useState("");
  const [modelDraft, setModelDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [revealedKey, setRevealedKey] = useState<string | null>(null);
  const revealRequest = useRef(0);

  useEffect(() => {
    setModelDraft(status?.model ?? "");
  }, [status?.model, status?.provider]);

  const provider = status?.provider ?? "deepseek";
  const providerName = t(`cloudPolish.providers.${provider}.name`);
  const providerText = { provider: providerName };

  const hideSavedKey = useCallback(() => {
    revealRequest.current += 1;
    setRevealedKey(null);
  }, []);

  const refresh = useCallback(async () => {
    const value = await invoke<CloudPolishStatus>("cloud_polish_status");
    setStatus(value);
    setModelDraft(value.model);
  }, []);

  useEffect(() => {
    let active = true;
    const reload = () => {
      hideSavedKey();
      invoke<CloudPolishStatus>("cloud_polish_status")
        .then((value) => active && setStatus(value))
        .catch(() => active && setError("loadError"));
    };
    reload();
    const unlisten = listen("cloud-polish-changed", reload);
    window.addEventListener("blur", hideSavedKey);
    return () => {
      active = false;
      revealRequest.current += 1;
      window.removeEventListener("blur", hideSavedKey);
      void unlisten.then((stop) => stop());
    };
  }, [hideSavedKey]);

  const change = async (operation: () => Promise<unknown>, message: string) => {
    setBusy(true);
    setError("");
    setNotice("");
    hideSavedKey();
    try {
      await operation();
      await refresh();
      setNotice(message);
    } catch {
      setError("saveError");
    } finally {
      setBusy(false);
    }
  };

  const saveKey = () =>
    change(async () => {
      await invoke("set_cloud_polish_key", { key });
      setKey("");
    }, "keySaved");

  const showSavedKey = async () => {
    const request = ++revealRequest.current;
    setBusy(true);
    setError("");
    try {
      const saved = await invoke<string>("reveal_cloud_polish_key");
      if (request === revealRequest.current) setRevealedKey(saved);
    } catch {
      if (request === revealRequest.current) setError("revealError");
    } finally {
      setBusy(false);
    }
  };

  const copy = async (original: boolean) => {
    const text = original ? status?.last?.original : status?.last?.polished;
    if (!text) return;
    setError("");
    if (await copyToClipboard(text)) {
      setNotice(original ? "originalCopied" : "resultCopied");
    } else {
      setError("copyError");
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="space-y-2">
        <h2 className="text-lg font-semibold">{t("cloudPolish.title")}</h2>
        <p className="text-sm text-text/70">{t("cloudPolish.description")}</p>
      </div>
      <div className="border-2 border-logo-primary/40 bg-logo-primary/5 rounded-xl p-5 space-y-3">
        <label className="flex items-center justify-between gap-4 cursor-pointer">
          <span className="space-y-1">
            <span className="block text-base font-semibold">
              {t("cloudPolish.enable")}
            </span>
            <span className="block text-sm text-text/70">
              {t("cloudPolish.toggleHelp")}
            </span>
          </span>
          <span className="relative flex items-center gap-3 shrink-0">
            <span className="text-sm font-medium" aria-hidden="true">
              {t(status?.enabled ? "cloudPolish.on" : "cloudPolish.off")}
            </span>
            <input
              type="checkbox"
              aria-label={t("cloudPolish.enable")}
              aria-describedby="polish-privacy"
              checked={status?.enabled ?? false}
              disabled={
                busy || !status || (!status.key_configured && !status.enabled)
              }
              onChange={(event) =>
                void change(
                  () =>
                    invoke("set_cloud_polish_enabled", {
                      enabled: event.target.checked,
                    }),
                  event.target.checked ? "enabledNotice" : "disabledNotice",
                )
              }
              className="absolute right-0 w-12 h-7 opacity-0 peer z-10 cursor-pointer disabled:cursor-not-allowed"
            />
            <span className="relative w-12 h-7 rounded-full bg-mid-gray/30 transition-colors peer-checked:bg-logo-primary peer-focus-visible:ring-2 peer-focus-visible:ring-logo-primary peer-focus-visible:ring-offset-2 peer-disabled:opacity-40 after:content-[''] after:absolute after:top-1 after:left-1 after:w-5 after:h-5 after:bg-white after:rounded-full after:transition-transform peer-checked:after:translate-x-5" />
          </span>
        </label>
        <p id="polish-privacy" className="text-sm text-text/70">
          {t("cloudPolish.privacy", providerText)}
        </p>
      </div>
      <div className="border border-mid-gray/20 rounded-lg p-4 space-y-5">
        <div className="space-y-2">
          <label htmlFor="polish-mode" className="text-sm font-medium">
            {t("cloudPolish.mode")}
          </label>
          <select
            id="polish-mode"
            value={status?.mode ?? "general"}
            disabled={busy || !status}
            onChange={(event) =>
              void change(
                () =>
                  invoke("configure_cloud_polish", {
                    mode: event.target.value,
                  }),
                "modeSaved",
              )
            }
            className="w-full rounded-md border border-mid-gray/40 bg-background px-3 py-2 text-sm"
          >
            {["general", "meeting", "journal"].map((mode) => (
              <option key={mode} value={mode}>
                {t(`cloudPolish.modes.${mode}.name`)}
              </option>
            ))}
          </select>
          <p className="text-sm text-text/60">
            {t(`cloudPolish.modes.${status?.mode ?? "general"}.description`)}
          </p>
        </div>
        <div className="space-y-2">
          <label htmlFor="polish-timeout" className="text-sm font-medium">
            {t("cloudPolish.timeout")}
          </label>
          <select
            id="polish-timeout"
            value={status?.timeout_seconds ?? 12}
            disabled={busy || !status}
            aria-describedby="polish-timeout-help"
            onChange={(event) =>
              void change(
                () =>
                  invoke("configure_cloud_polish", {
                    timeoutSeconds: Number(event.target.value),
                  }),
                "timeoutSaved",
              )
            }
            className="w-full rounded-md border border-mid-gray/40 bg-background px-3 py-2 text-sm"
          >
            {[12, 30, 60, 120].map((seconds) => (
              <option key={seconds} value={seconds}>
                {t(
                  seconds === 12
                    ? "cloudPolish.timeoutDefault"
                    : "cloudPolish.timeoutSeconds",
                  { seconds },
                )}
              </option>
            ))}
          </select>
          <p id="polish-timeout-help" className="text-sm text-text/60">
            {t("cloudPolish.timeoutHelp")}
          </p>
        </div>
        <div className="space-y-2">
          <label htmlFor="polish-provider" className="text-sm font-medium">
            {t("cloudPolish.provider")}
          </label>
          <select
            id="polish-provider"
            value={provider}
            disabled={busy || !status}
            onChange={(event) =>
              void change(async () => {
                await invoke("configure_cloud_polish", {
                  provider: event.target.value,
                });
                setKey("");
              }, "providerSaved")
            }
            className="w-full rounded-md border border-mid-gray/40 bg-background px-3 py-2 text-sm"
          >
            {["deepseek", "openai", "anthropic", "gemini"].map((id) => (
              <option key={id} value={id}>
                {t(`cloudPolish.providers.${id}.option`)}
              </option>
            ))}
          </select>
          <p className="text-sm text-text/60">
            {t(`cloudPolish.providers.${provider}.description`)}
          </p>
          <p className="text-sm text-text/60">
            {t("cloudPolish.providerHelp")}
          </p>
        </div>
        {status?.key_configured && (
          <div className="rounded-lg bg-mid-gray/5 border border-mid-gray/20 p-3 space-y-3">
            <label htmlFor="saved-polish-key" className="text-sm font-medium">
              {t("cloudPolish.savedKey", providerText)}
            </label>
            <Input
              id="saved-polish-key"
              type="text"
              readOnly
              autoComplete="off"
              spellCheck={false}
              value={revealedKey ?? status.key_preview ?? ""}
              className="w-full font-mono"
            />
            <div className="flex gap-2 flex-wrap">
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() =>
                  revealedKey === null ? void showSavedKey() : hideSavedKey()
                }
              >
                {t(
                  revealedKey === null
                    ? "cloudPolish.showKey"
                    : "cloudPolish.hideKey",
                )}
              </Button>
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() =>
                  void change(async () => {
                    await invoke("set_cloud_polish_key", { key: "" });
                    setKey("");
                  }, "keyRemoved")
                }
              >
                {t("cloudPolish.removeKey")}
              </Button>
            </div>
            <p className="text-sm text-text/60">
              {t("cloudPolish.savedKeyHelp")}
            </p>
          </div>
        )}
        <div className="space-y-2">
          <label htmlFor="polish-api-key" className="text-sm font-medium">
            {t("cloudPolish.apiKey", providerText)}
          </label>
          <Input
            id="polish-api-key"
            type="password"
            autoComplete="off"
            spellCheck={false}
            value={key}
            onChange={(event) => setKey(event.target.value)}
            disabled={busy || !status}
            placeholder={t(
              status?.key_configured
                ? "cloudPolish.keyReplace"
                : "cloudPolish.keyPlaceholder",
              providerText,
            )}
            className="w-full"
          />
          <p className="text-sm text-text/60">
            {t(
              status?.key_configured
                ? "cloudPolish.keyConfigured"
                : "cloudPolish.keyMissing",
            )}
          </p>
          <div className="flex gap-2 flex-wrap">
            <Button
              onClick={() => void saveKey()}
              disabled={busy || !status || key.trim().length < 16}
            >
              {t("cloudPolish.saveKey")}
            </Button>
          </div>
        </div>
        <div className="space-y-2">
          <label htmlFor="polish-model" className="text-sm font-medium">
            {t("cloudPolish.model")}
          </label>
          <div className="flex gap-2">
            <Input
              id="polish-model"
              value={modelDraft}
              disabled={busy || !status}
              onChange={(event) => setModelDraft(event.target.value)}
              className="min-w-0 flex-1 font-mono"
            />
            <Button
              variant="secondary"
              className="shrink-0 whitespace-nowrap"
              disabled={busy || !status || modelDraft.trim() === status.model}
              onClick={() =>
                void change(
                  () => invoke("configure_cloud_polish", { model: modelDraft }),
                  "modelSaved",
                )
              }
            >
              {t("cloudPolish.saveModel")}
            </Button>
          </div>
          <p className="text-sm text-text/60">
            {t("cloudPolish.modelHelp", providerText)}
          </p>
        </div>
      </div>
      {error && (
        <p role="alert" className="text-red-500">
          {t(`cloudPolish.${error}`)}
        </p>
      )}
      {notice && (
        <p role="status" className="text-sm text-text/80">
          {t(`cloudPolish.${notice}`, providerText)}
        </p>
      )}
      <div className="border border-mid-gray/20 rounded-lg p-4 space-y-3">
        <h3 className="font-medium">{t("cloudPolish.lastTitle")}</h3>
        <p className="text-sm text-text/60">
          {t("cloudPolish.lastDescription")}
        </p>
        {status?.last ? (
          <>
            {status.last.fallback && (
              <p className="text-sm" role="status">
                {t("cloudPolish.fallback")}
              </p>
            )}
            <h4 className="text-sm font-medium">{t("cloudPolish.original")}</h4>
            <p className="text-sm whitespace-pre-wrap break-words select-text">
              {status.last.original}
            </p>
            {status.last.polished && (
              <>
                <h4 className="text-sm font-medium">
                  {t("cloudPolish.result")}
                </h4>
                <p className="text-sm whitespace-pre-wrap break-words select-text">
                  {status.last.polished}
                </p>
              </>
            )}
            <div className="flex gap-2 flex-wrap">
              <Button variant="secondary" onClick={() => void copy(true)}>
                {t("cloudPolish.restore")}
              </Button>
              {status.last.polished && (
                <Button variant="secondary" onClick={() => void copy(false)}>
                  {t("cloudPolish.copyResult")}
                </Button>
              )}
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() =>
                  void change(
                    () => invoke("clear_last_cloud_polish"),
                    "cleared",
                  )
                }
              >
                {t("cloudPolish.clear")}
              </Button>
            </div>
          </>
        ) : (
          <p className="text-sm text-text/60">{t("cloudPolish.noLast")}</p>
        )}
      </div>
    </div>
  );
};
