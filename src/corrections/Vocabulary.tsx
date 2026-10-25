import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

export function Vocabulary() {
  const { t } = useTranslation();
  const [words, setWords] = useState<string[] | null>(null);
  const [draft, setDraft] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  useEffect(() => {
    let active = true;
    invoke<{ custom_words: string[] }>("get_app_settings")
      .then((settings) => active && setWords(settings.custom_words || []))
      .catch(() => active && setError("loadError"));
    return () => {
      active = false;
    };
  }, []);

  const save = async (updated: string[], message: string) => {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await invoke("update_custom_words", { words: updated });
      setWords(updated);
      setEditing(null);
      setDraft("");
      setNotice(message);
    } catch (cause) {
      const code = String(cause);
      setError(
        ["invalidWord", "duplicate", "wordLimit"].includes(code)
          ? code
          : "saveError",
      );
    } finally {
      setBusy(false);
    }
  };
  const add = () => {
    if (!words) return;
    const normalized = draft.trim().replace(/\s+/g, " ");
    if (
      words.some(
        (w) => w !== editing && w.toLowerCase() === normalized.toLowerCase(),
      )
    ) {
      setError("duplicate");
      return;
    }
    void save(
      editing === null
        ? [...words, normalized]
        : words.map((w) => (w === editing ? normalized : w)),
      "saved",
    );
  };

  return (
    <section
      className="lc-vocabulary"
      aria-labelledby="vocabulary-title"
      aria-busy={busy}
    >
      <h3 id="vocabulary-title">{t("dictionary.vocabulary")}</h3>
      <p className="lc-hint">{t("dictionary.description")}</p>
      <form
        className="lc-vocabulary-form"
        onSubmit={(event) => {
          event.preventDefault();
          add();
        }}
      >
        <label>
          {t("dictionary.word")}
          <input
            value={draft}
            disabled={busy || words === null}
            maxLength={100}
            placeholder={t("dictionary.placeholder")}
            onChange={(event) => {
              setDraft(event.target.value);
              setError("");
              setNotice("");
            }}
          />
        </label>
        <button
          type="submit"
          disabled={busy || words === null || !draft.trim()}
        >
          {t(editing === null ? "dictionary.add" : "dictionary.save")}
        </button>
        {editing !== null && (
          <button
            type="button"
            disabled={busy}
            onClick={() => {
              setEditing(null);
              setDraft("");
            }}
          >
            {t("dictionary.cancel")}
          </button>
        )}
      </form>
      <p className="lc-hint">
        {t("dictionary.limit", { count: words?.length ?? 0 })}
      </p>
      {error && <p role="alert">{t(`dictionary.${error}`)}</p>}
      {notice && <p role="status">{t(`dictionary.${notice}`)}</p>}
      {words?.length === 0 && (
        <p className="lc-hint">{t("dictionary.empty")}</p>
      )}
      {words?.map((word) => (
        <div className="lc-rule" key={word}>
          <strong>{word}</strong>
          <div className="lc-transfer-actions">
            <button
              disabled={busy}
              aria-label={t("dictionary.editWord", { word })}
              onClick={() => {
                setEditing(word);
                setDraft(word);
                setError("");
                setNotice("");
              }}
            >
              {t("dictionary.edit")}
            </button>
            <button
              disabled={busy}
              aria-label={t("dictionary.removeWord", { word })}
              onClick={() =>
                void save(
                  words.filter((w) => w !== word),
                  "removed",
                )
              }
            >
              {t("corrections.delete")}
            </button>
          </div>
        </div>
      ))}
    </section>
  );
}
