//! Local, explicit dictionary migration. Native file pickers run off the UI
//! thread; previews never mutate rules and carry no file paths from the webview.
use super::{persist, publish, Corrections, Dictionary, Rule};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

const FORMAT: &str = "handy-local-dictionary";
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_RULES: usize = 10_000;

#[derive(Serialize, Deserialize)]
struct DictionaryFile {
    format: String,
    version: u32,
    rules: Vec<Rule>,
}
#[derive(Serialize)]
struct Conflict {
    from: String,
    current: String,
    incoming: String,
}
#[derive(Serialize)]
struct Preview {
    token: u32,
    count: usize,
    added: usize,
    duplicates: usize,
    conflicts: Vec<Conflict>,
}
struct Pending {
    token: u32,
    rules: Vec<Rule>,
    baseline: Vec<Rule>,
}
#[derive(Default)]
struct Transfers {
    busy: AtomicBool,
    pending: Mutex<Option<Pending>>,
    next: Mutex<u32>,
}
pub(super) fn initialize(app: &AppHandle) {
    app.manage(Transfers::default());
}

fn parse_file(bytes: &[u8]) -> Result<Vec<Rule>, String> {
    if bytes.len() > MAX_BYTES {
        return Err("fileLimit".into());
    }
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| "invalidFile")?;
    let rules = if value.get("format").is_some() || value.get("version").is_some() {
        let file: DictionaryFile = serde_json::from_value(value).map_err(|_| "invalidFile")?;
        if file.format != FORMAT || file.version != 1 {
            return Err("unsupportedFile".into());
        }
        file.rules
    } else {
        // Also accept the original local_corrections.json for existing users.
        serde_json::from_value::<Dictionary>(value)
            .map_err(|_| "invalidFile")?
            .rules
    };
    validate_rules(rules)
}
pub(super) fn validate_rules(rules: Vec<Rule>) -> Result<Vec<Rule>, String> {
    if rules.len() > MAX_RULES {
        return Err("fileLimit".into());
    }
    let mut unique: Vec<Rule> = Vec::new();
    let mut seen = HashMap::new();
    for rule in rules {
        if rule.from == rule.to
            || [&rule.from, &rule.to].iter().any(|s| {
                s.is_empty()
                    || s.trim() != s.as_str()
                    || s.chars().count() > 64
                    || s.chars().any(char::is_control)
            })
        {
            return Err("invalidFile".into());
        }
        if let Some(previous) = seen.get(&rule.from) {
            if previous != &rule.to {
                return Err("ambiguousFile".into());
            }
            continue;
        }
        seen.insert(rule.from.clone(), rule.to.clone());
        unique.push(rule);
    }
    Ok(unique)
}
fn preview(token: u32, existing: &[Rule], incoming: &[Rule]) -> Preview {
    let by_from: HashMap<_, _> = existing
        .iter()
        .map(|r| (r.from.as_str(), r.to.as_str()))
        .collect();
    let mut result = Preview {
        token,
        count: incoming.len(),
        added: 0,
        duplicates: 0,
        conflicts: vec![],
    };
    for rule in incoming {
        match by_from.get(rule.from.as_str()) {
            None => result.added += 1,
            Some(to) if *to == rule.to => result.duplicates += 1,
            Some(to) => result.conflicts.push(Conflict {
                from: rule.from.clone(),
                current: (*to).into(),
                incoming: rule.to.clone(),
            }),
        }
    }
    result
}
fn merge(existing: &[Rule], incoming: &[Rule], replace: bool) -> Vec<Rule> {
    let mut merged = existing.to_vec();
    let mut positions: HashMap<_, _> = merged
        .iter()
        .enumerate()
        .map(|(i, r)| (r.from.clone(), i))
        .collect();
    for rule in incoming {
        if let Some(&at) = positions.get(&rule.from) {
            if replace {
                merged[at] = rule.clone();
            }
        } else {
            positions.insert(rule.from.clone(), merged.len());
            merged.push(rule.clone());
        }
    }
    merged
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("writeFile")?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| "writeFile")?;
    file.write_all(bytes).map_err(|_| "writeFile")?;
    file.as_file().sync_all().map_err(|_| "writeFile")?;
    file.persist(path).map_err(|_| "writeFile")?;
    Ok(())
}
fn encode(rules: Vec<Rule>) -> Result<Vec<u8>, String> {
    let bytes = serde_json::to_vec_pretty(&DictionaryFile {
        format: FORMAT.into(),
        version: 1,
        rules: validate_rules(rules)?,
    })
    .map_err(|_| "invalidFile")?;
    if bytes.len() > MAX_BYTES {
        return Err("fileLimit".into());
    }
    Ok(bytes)
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct Request {
    token: u32,
    policy: String,
}

#[tauri::command]
#[specta::specta]
pub async fn corrections_transfer(
    app: AppHandle,
    operation: String,
    payload: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || transfer(&app, &operation, &payload))
        .await
        .map_err(|_| "transferError".to_string())?
}
fn transfer(app: &AppHandle, operation: &str, payload: &str) -> Result<String, String> {
    let transfers = app.state::<Transfers>();
    if transfers.busy.swap(true, Ordering::AcqRel) {
        return Err("transferBusy".into());
    }
    struct BusyGuard<'a>(&'a AtomicBool);
    impl Drop for BusyGuard<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let _guard = BusyGuard(&transfers.busy);
    let manager = app.state::<Corrections>();
    let request: Request = serde_json::from_str(payload).map_err(|_| "transferError")?;
    match operation {
        "export" => {
            let (rules, live_path) = {
                let state = manager.0.lock().unwrap();
                if state.storage_error {
                    return Err("storage".into());
                }
                (state.dictionary.rules.clone(), state.path.clone())
            };
            let bytes = encode(rules.clone())?;
            let mut dialog = app
                .dialog()
                .file()
                .add_filter("JSON", &["json"])
                .set_file_name("Handy-Local-Dictionary.json");
            if let Some(window) = app.get_webview_window("main") {
                dialog = dialog.set_parent(&window);
            }
            let Some(file) = dialog.blocking_save_file() else {
                return Ok("{\"status\":\"cancelled\"}".into());
            };
            let path = file.into_path().map_err(|_| "writeFile")?;
            if path == live_path
                || (path.exists() && path.canonicalize().ok() == live_path.canonicalize().ok())
            {
                return Err("liveFile".into());
            }
            atomic_write(&path, &bytes)?;
            Ok(serde_json::json!({"status":"exported", "count":rules.len()}).to_string())
        }
        "import" => {
            *transfers.pending.lock().unwrap() = None;
            let mut dialog = app.dialog().file().add_filter("JSON", &["json"]);
            if let Some(window) = app.get_webview_window("main") {
                dialog = dialog.set_parent(&window);
            }
            let Some(file) = dialog.blocking_pick_file() else {
                return Ok("{\"status\":\"cancelled\"}".into());
            };
            let path = file.into_path().map_err(|_| "readFile")?;
            let file = std::fs::File::open(path).map_err(|_| "readFile")?;
            let mut bytes = Vec::new();
            file.take((MAX_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| "readFile")?;
            let rules = parse_file(&bytes)?;
            let baseline = manager.0.lock().unwrap().dictionary.rules.clone();
            let mut next = transfers.next.lock().unwrap();
            *next = next.wrapping_add(1);
            let result = preview(*next, &baseline, &rules);
            *transfers.pending.lock().unwrap() = Some(Pending {
                token: *next,
                rules,
                baseline,
            });
            Ok(serde_json::json!({"status":"preview", "preview":result}).to_string())
        }
        "cancel" => {
            let mut pending = transfers.pending.lock().unwrap();
            if pending.as_ref().is_some_and(|p| p.token == request.token) {
                *pending = None;
            }
            Ok("{\"status\":\"cancelled\"}".into())
        }
        "apply" => {
            if request.policy != "keep" && request.policy != "replace" {
                return Err("transferError".into());
            }
            let mut pending = transfers.pending.lock().unwrap();
            let import = pending
                .as_ref()
                .filter(|p| p.token == request.token)
                .ok_or("staleImport")?;
            let mut state = manager.0.lock().unwrap();
            let result = apply_import(&mut state, import, request.policy == "replace");
            if result.as_ref().is_err_and(|error| error == "staleImport") {
                *pending = None;
            }
            let result = result?;
            *pending = None;
            publish(app, &state);
            Ok(result)
        }
        _ => Err("transferError".into()),
    }
}

fn apply_import(
    state: &mut super::State,
    import: &Pending,
    replace: bool,
) -> Result<String, String> {
    if state.storage_error {
        return Err("storage".into());
    }
    if state.dictionary.rules != import.baseline {
        return Err("staleImport".into());
    }
    let summary = preview(import.token, &state.dictionary.rules, &import.rules);
    let rules = merge(&state.dictionary.rules, &import.rules, replace);
    if rules.len() > MAX_RULES {
        return Err("fileLimit".into());
    }
    if rules != state.dictionary.rules {
        let backup = serde_json::to_vec_pretty(&state.dictionary).map_err(|_| "storage")?;
        atomic_write(
            &state
                .path
                .with_file_name("local_corrections.before-import.json"),
            &backup,
        )
        .map_err(|_| "backupFailed")?;
        let enabled = state.dictionary.enabled;
        persist(state, Dictionary { enabled, rules })?;
    }
    let replaced = if replace { summary.conflicts.len() } else { 0 };
    Ok(serde_json::json!({"status":"imported", "added":summary.added,"replaced":replaced,"skipped":summary.duplicates + summary.conflicts.len() - replaced}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rule(from: &str, to: &str) -> Rule {
        Rule {
            from: from.into(),
            to: to.into(),
        }
    }
    #[test]
    fn portable_and_legacy_round_trip_only_rules() {
        let rules = vec![rule("Andromida", "Andromeda"), rule("新秀公园", "新绣公园")];
        let encoded = encode(rules.clone()).unwrap();
        assert_eq!(parse_file(&encoded).unwrap(), rules);
        assert!(!String::from_utf8(encoded).unwrap().contains("enabled"));
        let legacy = serde_json::to_vec(&Dictionary {
            enabled: false,
            rules: rules.clone(),
        })
        .unwrap();
        assert_eq!(parse_file(&legacy).unwrap(), rules);
    }
    #[test]
    fn malformed_or_ambiguous_files_are_rejected_as_a_whole() {
        assert!(parse_file(b"not json").is_err());
        assert!(
            parse_file(br#"{"format":"handy-local-dictionary","version":2,"rules":[]}"#).is_err()
        );
        for invalid in [
            rule("", "a"),
            rule("a", "a"),
            rule("a\nb", "c"),
            rule(" a", "b"),
            rule(&"字".repeat(65), "b"),
        ] {
            assert!(validate_rules(vec![rule("ok", "yes"), invalid]).is_err());
        }
        assert!(validate_rules(vec![rule("a", "b"), rule("a", "c")]).is_err());
        assert_eq!(
            validate_rules(vec![rule("a", "b"), rule("a", "b")]).unwrap(),
            vec![rule("a", "b")]
        );
        assert!(parse_file(&vec![b' '; MAX_BYTES + 1]).is_err());
    }
    #[test]
    fn preview_and_merge_respect_explicit_conflict_choice() {
        let old = vec![rule("a", "old"), rule("same", "same target")];
        let new = vec![
            rule("a", "new"),
            rule("same", "same target"),
            rule("b", "bee"),
        ];
        let summary = preview(1, &old, &new);
        assert_eq!(
            (summary.added, summary.duplicates, summary.conflicts.len()),
            (1, 1, 1)
        );
        assert_eq!(
            merge(&old, &new, false),
            vec![
                rule("a", "old"),
                rule("same", "same target"),
                rule("b", "bee")
            ]
        );
        assert_eq!(
            merge(&old, &new, true),
            vec![
                rule("a", "new"),
                rule("same", "same target"),
                rule("b", "bee")
            ]
        );
        assert_eq!(old[0].to, "old");
    }
    #[test]
    fn import_preserves_settings_backs_up_old_rules_and_rejects_stale_preview() {
        let dir = tempfile::tempdir().unwrap();
        let old = vec![rule("a", "old")];
        let mut state = super::super::State {
            dictionary: Dictionary {
                enabled: false,
                rules: old.clone(),
            },
            path: dir.path().join("local_corrections.json"),
            session: None,
            next_id: 0,
            storage_error: false,
        };
        let import = Pending {
            token: 1,
            baseline: old.clone(),
            rules: vec![rule("a", "new"), rule("b", "bee")],
        };
        apply_import(&mut state, &import, true).unwrap();
        assert!(!state.dictionary.enabled);
        assert_eq!(state.dictionary.rules, import.rules);
        let backup =
            std::fs::read(dir.path().join("local_corrections.before-import.json")).unwrap();
        assert_eq!(parse_file(&backup).unwrap(), old);
        let saved = std::fs::read(&state.path).unwrap();
        assert_eq!(parse_file(&saved).unwrap(), import.rules);
        assert_eq!(
            apply_import(&mut state, &import, false),
            Err("staleImport".into())
        );
        assert_eq!(std::fs::read(&state.path).unwrap(), saved);
    }
    #[test]
    fn failed_backup_leaves_live_dictionary_untouched() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("local_corrections.before-import.json")).unwrap();
        let old = vec![rule("a", "old")];
        let mut state = super::super::State {
            dictionary: Dictionary {
                enabled: true,
                rules: old.clone(),
            },
            path: dir.path().join("local_corrections.json"),
            session: None,
            next_id: 0,
            storage_error: false,
        };
        let import = Pending {
            token: 1,
            baseline: old.clone(),
            rules: vec![rule("a", "new")],
        };
        assert_eq!(
            apply_import(&mut state, &import, true),
            Err("backupFailed".into())
        );
        assert_eq!(state.dictionary.rules, old);
        assert!(!state.path.exists());
    }
    #[test]
    fn atomic_export_can_be_read_and_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dictionary.json");
        atomic_write(&path, &encode(vec![rule("a", "b")]).unwrap()).unwrap();
        atomic_write(&path, &encode(vec![rule("c", "d")]).unwrap()).unwrap();
        assert_eq!(
            parse_file(&std::fs::read(path).unwrap()).unwrap(),
            vec![rule("c", "d")]
        );
    }
}
