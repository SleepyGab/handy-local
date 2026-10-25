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
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [revealedKey, setRevealedKey] = useState<string | null>(null);
  const revealRequest = useRef(0);

  const hideSavedKey = useCallback(() => {
    revealRequest.current += 1;
    setRevealedKey(null);
  }, []);

  const refresh = useCallback(async () => {
    setStatus(await invoke<CloudPolishStatus>("cloud_polish_status"));
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
      <div className="border border-mid-gray/20 rounded-lg p-4 space-y-5">
        <label className="flex items-center justify-between gap-4">
          <span className="font-medium">{t("cloudPolish.enable")}</span>
          <input
            type="checkbox"
            checked={status?.enabled ?? false}
            disabled={busy || !status?.key_configured}
            onChange={(event) =>
              void change(
                () =>
                  invoke("set_cloud_polish_enabled", {
                    enabled: event.target.checked,
                  }),
                event.target.checked ? "enabledNotice" : "disabledNotice",
              )
            }
            className="w-5 h-5 accent-logo-primary"
          />
        </label>
        <p className="text-sm text-text/70">{t("cloudPolish.privacy")}</p>
        {status?.key_configured && (
          <div className="rounded-lg bg-mid-gray/5 border border-mid-gray/20 p-3 space-y-3">
            <label htmlFor="saved-deepseek-key" className="text-sm font-medium">
              {t("cloudPolish.savedKey")}
            </label>
            <Input
              id="saved-deepseek-key"
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
          <label htmlFor="deepseek-api-key" className="text-sm font-medium">
            {t("cloudPolish.apiKey")}
          </label>
          <Input
            id="deepseek-api-key"
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
        <p className="text-sm text-text/60">{t("cloudPolish.model")}</p>
      </div>
      {error && (
        <p role="alert" className="text-red-500">
          {t(`cloudPolish.${error}`)}
        </p>
      )}
      {notice && (
        <p role="status" className="text-sm text-text/80">
          {t(`cloudPolish.${notice}`)}
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
