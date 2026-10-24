//! Explicitly confirmed, on-device corrections. Raw field contents are held in
//! memory for at most two minutes; only confirmed phrase pairs go to disk.
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
#[cfg(target_os = "macos")]
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
#[cfg(target_os = "macos")]
tauri_panel! { panel!(CorrectionPanel { config: { can_become_key_window: true, is_floating_panel: true } }) }

const PROMPT_WIDTH: f64 = 344.0;
const PROMPT_HEIGHT: f64 = 156.0;
const EDITOR_WIDTH: f64 = 440.0;
const EDITOR_HEIGHT: f64 = 360.0;
static RECENT_DICTATION: AtomicBool = AtomicBool::new(false);
static POPUP_WANTED: AtomicBool = AtomicBool::new(false);

pub fn can_correct_last() -> bool {
    RECENT_DICTATION.load(Ordering::Relaxed)
}

/// The tray and settings page share the same expiry checks and editor action.
pub fn edit_last_from_tray(app: &AppHandle) {
    let id = app.try_state::<Corrections>().and_then(|m| {
        m.0.lock()
            .ok()
            .and_then(|s| s.session.as_ref().map(|s| s.id))
    });
    if let Some(id) = id {
        let _ = corrections_action(
            app.clone(),
            "edit".into(),
            serde_json::json!({"id":id}).to_string(),
        );
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Rule {
    pub from: String,
    pub to: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Dictionary {
    enabled: bool,
    rules: Vec<Rule>,
}
impl Default for Dictionary {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: vec![],
        }
    }
}
#[derive(Deserialize)]
struct Snapshot {
    text: String,
    start: usize,
    length: usize,
}

#[cfg(target_os = "macos")]
mod native {
    use super::Snapshot;
    use std::ffi::{c_char, c_void, CStr, CString};
    unsafe extern "C" {
        fn lc_capture() -> *mut c_void;
        fn lc_snapshot(handle: *mut c_void, focused: bool) -> *mut c_char;
        fn lc_replace(
            handle: *mut c_void,
            expected: *const c_char,
            start: isize,
            length: isize,
            replacement: *const c_char,
        ) -> bool;
        fn lc_release(handle: *mut c_void);
        fn lc_free(string: *mut c_char);
    }
    pub struct Field(*mut c_void);
    // All accesses are serialized by State's mutex. AX APIs support background
    // access; the retained element is never exposed outside this module.
    unsafe impl Send for Field {}
    impl Field {
        pub fn capture() -> Option<Self> {
            let p = unsafe { lc_capture() };
            (!p.is_null()).then_some(Self(p))
        }
        pub fn read(&self, focused: bool) -> Option<Snapshot> {
            unsafe {
                let p = lc_snapshot(self.0, focused);
                if p.is_null() {
                    return None;
                }
                let value = serde_json::from_slice(CStr::from_ptr(p).to_bytes()).ok();
                lc_free(p);
                value
            }
        }
        pub fn replace(&self, expected: &str, start: usize, length: usize, text: &str) -> bool {
            let (Ok(expected), Ok(text)) = (CString::new(expected), CString::new(text)) else {
                return false;
            };
            unsafe {
                lc_replace(
                    self.0,
                    expected.as_ptr(),
                    start as isize,
                    length as isize,
                    text.as_ptr(),
                )
            }
        }
    }
    impl Drop for Field {
        fn drop(&mut self) {
            unsafe { lc_release(self.0) }
        }
    }
}
#[cfg(not(target_os = "macos"))]
mod native {
    use super::Snapshot;
    pub struct Field;
    impl Field {
        pub fn capture() -> Option<Self> {
            None
        }
        pub fn read(&self, _: bool) -> Option<Snapshot> {
            None
        }
        pub fn replace(&self, _: &str, _: usize, _: usize, _: &str) -> bool {
            false
        }
    }
}

pub struct Capture {
    field: native::Field,
    before: Snapshot,
}
struct Session {
    id: u32,
    original: String,
    current: String,
    prefix: String,
    suffix: String,
    field: Option<native::Field>,
    started: Instant,
    baseline: bool,
    observing: bool,
    stable_text: String,
    stable_since: Instant,
    suggestion: Option<Rule>,
    dismissed: bool,
    editing: bool,
}
struct State {
    dictionary: Dictionary,
    path: PathBuf,
    session: Option<Session>,
    next_id: u32,
    storage_error: bool,
}
pub struct Corrections(Mutex<State>);
#[derive(Clone, Serialize)]
struct View {
    enabled: bool,
    rules: Vec<Rule>,
    last: Option<Last>,
    storage_error: bool,
}
#[derive(Clone, Serialize)]
struct Last {
    id: u32,
    original: String,
    current: String,
    suggestion: Option<Rule>,
    observing: bool,
    editing: bool,
}

fn view(state: &State) -> View {
    View {
        enabled: state.dictionary.enabled,
        rules: state.dictionary.rules.clone(),
        storage_error: state.storage_error,
        last: state.session.as_ref().map(|s| Last {
            id: s.id,
            original: s.original.clone(),
            current: s.current.clone(),
            suggestion: s.suggestion.clone(),
            observing: s.observing,
            editing: s.editing,
        }),
    }
}
fn publish(app: &AppHandle, state: &State) {
    let available = state.dictionary.enabled && state.session.is_some();
    if RECENT_DICTATION.swap(available, Ordering::Relaxed) != available {
        crate::tray::update_tray_menu(app);
    }
    let _ = app.emit("local-corrections-changed", view(state));
}
fn popup(app: &AppHandle, show: bool) {
    POPUP_WANTED.store(show, Ordering::Relaxed);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let show = POPUP_WANTED.load(Ordering::Relaxed);
        #[cfg(target_os = "macos")]
        if let Ok(panel) = handle.get_webview_panel("correction") {
            if show {
                panel.show();
            } else {
                panel.hide();
            }
        }
        #[cfg(not(target_os = "macos"))]
        if let Some(window) = handle.get_webview_window("correction") {
            if show {
                let _ = window.show();
            } else {
                let _ = window.hide();
            }
        }
    });
}
fn position_popup(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("correction") {
        if let Ok(Some(monitor)) = window.primary_monitor() {
            let area = monitor.work_area();
            let scale = monitor.scale_factor();
            let x = (area.position.x as f64 + area.size.width as f64 / 2.0) / scale
                - PROMPT_WIDTH / 2.0;
            let y =
                (area.position.y as f64 + area.size.height as f64) / scale - PROMPT_HEIGHT - 22.0;
            let _ = window.set_position(tauri::LogicalPosition::new(x, y));
        }
    }
}
fn persist(state: &mut State, dictionary: Dictionary) -> Result<(), String> {
    use std::io::Write;
    let result = (|| -> std::io::Result<()> {
        let tmp = state.path.with_extension("tmp");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(&serde_json::to_vec_pretty(&dictionary)?)?;
        file.sync_all()?;
        std::fs::rename(tmp, &state.path)?;
        Ok(())
    })();
    if result.is_err() {
        state.storage_error = true;
        return Err("storage".into());
    }
    state.storage_error = false;
    state.dictionary = dictionary;
    Ok(())
}

pub fn initialize(app: &AppHandle) {
    let Ok(dir) = crate::portable::app_data_dir(app) else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("local_corrections.json");
    let (dictionary, storage_error) = match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(d) => (d, false),
            Err(_) => (Dictionary::default(), true),
        },
        Err(e) => (
            Dictionary::default(),
            e.kind() != std::io::ErrorKind::NotFound,
        ),
    };
    app.manage(Corrections(Mutex::new(State {
        dictionary,
        path,
        session: None,
        next_id: 0,
        storage_error,
    })));
    #[cfg(target_os = "macos")]
    {
        if let Ok(panel) = PanelBuilder::<_, CorrectionPanel>::new(app, "correction")
            .url(tauri::WebviewUrl::App("src/corrections/index.html".into()))
            .title("Handy Local")
            .size(tauri::LogicalSize::new(PROMPT_WIDTH, PROMPT_HEIGHT).into())
            .level(PanelLevel::Floating)
            .no_activate(true)
            .transparent(true)
            .has_shadow(false)
            .corner_radius(18.0)
            .style_mask(StyleMask::empty().borderless().nonactivating_panel())
            .with_window(|w| {
                w.transparent(true)
                    .visible(false)
                    .focused(false)
                    .decorations(false)
                    .resizable(false)
                    .skip_taskbar(true)
            })
            .collection_behavior(
                CollectionBehavior::new()
                    .can_join_all_spaces()
                    .full_screen_auxiliary(),
            )
            .build()
        {
            panel.hide();
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "correction",
        tauri::WebviewUrl::App("src/corrections/index.html".into()),
    )
    .title("Handy Local")
    .inner_size(PROMPT_WIDTH, PROMPT_HEIGHT)
    .transparent(true)
    .visible(false)
    .focused(false)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .build();
    position_popup(app);
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(400));
        let manager = app.state::<Corrections>();
        let mut state = manager.0.lock().unwrap();
        let Some(session) = state.session.as_mut() else {
            continue;
        };
        if session.started.elapsed() > Duration::from_secs(120) {
            state.session = None;
            publish(&app, &state);
            popup(&app, false);
            continue;
        }
        if !session.observing {
            continue;
        }
        if session.started.elapsed() > Duration::from_secs(45) {
            session.observing = false;
            publish(&app, &state);
            continue;
        }
        let snapshot = session.field.as_ref().and_then(|field| field.read(true));
        let Some(snapshot) = snapshot else {
            session.observing = false;
            publish(&app, &state);
            continue;
        };
        // Only accept edits within the exact dictated span. The surrounding
        // field must still match; changing other text stops observation.
        let Some(current) = extract_span(&snapshot.text, &session.prefix, &session.suffix) else {
            session.observing = false;
            publish(&app, &state);
            continue;
        };
        if !session.baseline {
            if current == session.original {
                session.baseline = true;
            } else if session.started.elapsed() > Duration::from_secs(3) {
                session.observing = false;
                publish(&app, &state);
            }
            continue;
        }
        if current != session.stable_text {
            session.stable_text = current.to_string();
            session.stable_since = Instant::now();
            continue;
        }
        if session.stable_since.elapsed() < Duration::from_millis(1200)
            || snapshot.length != 0
            || current == session.current
        {
            continue;
        }
        session.current = current.to_string();
        session.suggestion = infer_rule(&session.original, current);
        let show = !session.dismissed && session.suggestion.is_some();
        publish(&app, &state);
        popup(&app, show);
    });
}

fn extract_span<'a>(text: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    text.strip_prefix(prefix)?.strip_suffix(suffix)
}
fn split_utf16(text: &str, start: usize, length: usize) -> Option<(String, String)> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if start.checked_add(length)? > units.len() {
        return None;
    }
    Some((
        String::from_utf16(&units[..start]).ok()?,
        String::from_utf16(&units[start + length..]).ok()?,
    ))
}
pub fn capture(app: &AppHandle) -> Option<Capture> {
    let manager = app.try_state::<Corrections>()?;
    if !manager.0.lock().unwrap().dictionary.enabled {
        return None;
    }
    let field = native::Field::capture()?;
    let before = field.read(true)?;
    Some(Capture { field, before })
}
pub fn begin(app: &AppHandle, text: String, capture: Option<Capture>) {
    let Some(manager) = app.try_state::<Corrections>() else {
        return;
    };
    let mut state = manager.0.lock().unwrap();
    if !state.dictionary.enabled || text.len() > 16000 {
        return;
    }
    let (field, prefix, suffix) = match capture {
        Some(c) => match split_utf16(&c.before.text, c.before.start, c.before.length) {
            Some((prefix, suffix)) => (Some(c.field), prefix, suffix),
            None => (None, String::new(), String::new()),
        },
        None => (None, String::new(), String::new()),
    };
    state.next_id = state.next_id.wrapping_add(1);
    let observing = field.is_some();
    state.session = Some(Session {
        id: state.next_id,
        original: text.clone(),
        current: text.clone(),
        prefix,
        suffix,
        field,
        started: Instant::now(),
        baseline: false,
        observing,
        stable_text: text,
        stable_since: Instant::now(),
        suggestion: None,
        dismissed: false,
        editing: false,
    });
    if let Some(w) = app.get_webview_window("correction") {
        let _ = w.set_size(tauri::LogicalSize::new(PROMPT_WIDTH, PROMPT_HEIGHT));
    }
    position_popup(app);
    publish(app, &state);
    // Ordinary dictation, including inaccessible text fields, stays silent.
    popup(app, false);
}
pub fn clear(app: &AppHandle) {
    if let Some(manager) = app.try_state::<Corrections>() {
        let mut state = manager.0.lock().unwrap();
        state.session = None;
        publish(app, &state);
        popup(app, false);
    }
}

// Exact, single-pass phrase matching: longest phrases win, output is never
// matched again, and ASCII words cannot replace parts of other words.
fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
pub fn replace_rules(text: &str, rules: &[Rule]) -> String {
    let mut ordered: Vec<_> = rules.iter().filter(|r| !r.from.is_empty()).collect();
    ordered.sort_by_key(|r| std::cmp::Reverse(r.from.len()));
    let mut output = String::new();
    let mut i = 0;
    while i < text.len() {
        let found = ordered.iter().find(|r| {
            if !text[i..].starts_with(&r.from) {
                return false;
            }
            let left = r.from.chars().next().is_some_and(word)
                && text[..i].chars().next_back().is_some_and(word);
            let right = r.from.chars().next_back().is_some_and(word)
                && text[i + r.from.len()..].chars().next().is_some_and(word);
            !left && !right
        });
        if let Some(rule) = found {
            output.push_str(&rule.to);
            i += rule.from.len();
        } else {
            let c = text[i..].chars().next().unwrap();
            output.push(c);
            i += c.len_utf8();
        }
    }
    output
}
pub fn apply(app: &AppHandle, text: &str) -> String {
    let Some(manager) = app.try_state::<Corrections>() else {
        return text.into();
    };
    let state = manager.0.lock().unwrap();
    if state.dictionary.enabled {
        replace_rules(text, &state.dictionary.rules)
    } else {
        text.into()
    }
}
pub fn infer_rule(before: &str, after: &str) -> Option<Rule> {
    let a: Vec<char> = before.chars().collect();
    let b: Vec<char> = after.chars().collect();
    let mut start = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
    let mut end_a = a.len();
    let mut end_b = b.len();
    while end_a > start && end_b > start && a[end_a - 1] == b[end_b - 1] {
        end_a -= 1;
        end_b -= 1;
    }
    // Additions, deletions, punctuation-only edits and rewrites are not word corrections.
    if start == end_a || start == end_b {
        return None;
    }
    while start > 0
        && (a.get(start).is_some_and(|c| word(*c)) || b.get(start).is_some_and(|c| word(*c)))
        && word(a[start - 1])
    {
        start -= 1;
    }
    while end_a < a.len() && word(a[end_a]) {
        end_a += 1;
    }
    while end_b < b.len() && word(b[end_b]) {
        end_b += 1;
    }
    let from: String = a[start..end_a].iter().collect();
    let to: String = b[start..end_b].iter().collect();
    if from.chars().count() > 32
        || to.chars().count() > 32
        || from.contains(char::is_whitespace)
        || to.contains(char::is_whitespace)
        || !from.chars().any(char::is_alphanumeric)
        || !to.chars().any(char::is_alphanumeric)
    {
        return None;
    }
    Some(Rule { from, to })
}

#[tauri::command]
#[specta::specta]
pub fn corrections_state(app: AppHandle) -> Result<String, String> {
    let manager = app.try_state::<Corrections>().ok_or("unavailable")?;
    let state = manager.0.lock().unwrap();
    serde_json::to_string(&view(&state)).map_err(|_| "serialize".into())
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct Payload {
    id: u32,
    from: String,
    to: String,
    corrected: String,
    enabled: bool,
}
#[tauri::command]
#[specta::specta]
pub fn corrections_action(
    app: AppHandle,
    action: String,
    payload: String,
) -> Result<String, String> {
    let p: Payload = serde_json::from_str(&payload).map_err(|_| "invalid")?;
    let manager = app.try_state::<Corrections>().ok_or("unavailable")?;
    let mut state = manager.0.lock().unwrap();
    let mut result = "ok";
    match action.as_str() {
        "toggle" => {
            let mut d = state.dictionary.clone();
            d.enabled = p.enabled;
            persist(&mut state, d)?;
            if !p.enabled {
                state.session = None;
                popup(&app, false);
            }
        }
        "delete" => {
            let mut d = state.dictionary.clone();
            d.rules.retain(|r| r.from != p.from);
            persist(&mut state, d)?;
        }
        "dismiss" => {
            if p.id != 0 && state.session.as_ref().is_none_or(|s| s.id != p.id) {
                return Err("expired".into());
            }
            if let Some(s) = &mut state.session {
                s.dismissed = true;
                s.suggestion = None;
                s.editing = false;
            }
            popup(&app, false);
        }
        "edit" => {
            let s = state.session.as_mut().ok_or("expired")?;
            if s.id != p.id || s.started.elapsed() > Duration::from_secs(120) {
                return Err("expired".into());
            }
            s.observing = false;
            s.editing = true;
            POPUP_WANTED.store(true, Ordering::Relaxed);
            if let Some(w) = app.get_webview_window("correction") {
                let _ = w.set_size(tauri::LogicalSize::new(EDITOR_WIDTH, EDITOR_HEIGHT));
                let _ = w.center();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        "copy" => {
            app.clipboard()
                .write_text(p.corrected)
                .map_err(|_| "clipboard")?;
        }
        "save" => {
            let s = state.session.as_ref().ok_or("expired")?;
            if s.id != p.id || s.started.elapsed() > Duration::from_secs(120) {
                return Err("expired".into());
            }
            let from = p.from.trim();
            let to = p.to.trim();
            if from.is_empty()
                || to.is_empty()
                || from == to
                || from.chars().count() > 64
                || to.chars().count() > 64
                || from.contains(['\n', '\r', '\0'])
                || to.contains(['\n', '\r', '\0'])
                || !s.original.contains(from)
            {
                return Err("phrase".into());
            }
            // The preview must be exactly the confirmed rule applied to this
            // utterance; no unreviewed sentence rewrite can enter the dictionary.
            let corrected = replace_rules(
                &s.original,
                &[Rule {
                    from: from.into(),
                    to: to.into(),
                }],
            );
            if corrected == s.original || (!p.corrected.is_empty() && p.corrected != corrected) {
                return Err("phrase".into());
            }
            let mut d = state.dictionary.clone();
            d.rules.retain(|r| r.from != from);
            d.rules.push(Rule {
                from: from.into(),
                to: to.into(),
            });
            persist(&mut state, d)?;
            let s = state.session.as_mut().unwrap();
            s.observing = false;
            if s.current != corrected {
                let expected = format!("{}{}{}", s.prefix, s.current, s.suffix);
                let replaced = s.baseline
                    && s.field.as_ref().is_some_and(|f| {
                        f.replace(
                            &expected,
                            s.prefix.encode_utf16().count(),
                            s.current.encode_utf16().count(),
                            &corrected,
                        )
                    });
                result = if replaced { "saved" } else { "saved_copy" };
            } else {
                result = "saved";
            }
            if result == "saved_copy" {
                s.baseline = false;
                s.field = None;
            }
            if !s.editing {
                if let Some(w) = app.get_webview_window("correction") {
                    let _ = w.set_size(tauri::LogicalSize::new(PROMPT_WIDTH, 212.0));
                }
            }
            s.current = corrected;
            s.suggestion = None;
            s.dismissed = true;
        }
        _ => return Err("invalid".into()),
    }
    publish(&app, &state);
    Ok(result.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r(a: &str, b: &str) -> Rule {
        Rule {
            from: a.into(),
            to: b.into(),
        }
    }
    #[test]
    fn corrections_are_non_cascading_and_word_bounded() {
        assert_eq!(
            replace_rules(
                "cat catalog catapult Cat 猫猫",
                &[r("cat", "dog"), r("dog", "fox"), r("猫猫", "喵")]
            ),
            "dog catalog catapult Cat 喵"
        );
        assert_eq!(
            replace_rules("Qwen ASR", &[r("Qwen", "one"), r("Qwen ASR", "two")]),
            "two"
        );
    }
    #[test]
    fn unicode_selection_and_anchors() {
        assert_eq!(
            split_utf16("🙂你好end", 2, 2),
            Some(("🙂".into(), "end".into()))
        );
        assert!(split_utf16("🙂你好", 1, 1).is_none());
        assert_eq!(extract_span("前新名字后", "前", "后"), Some("新名字"));
        assert_eq!(extract_span("别处改了新名字后", "前", "后"), None);
    }
    #[test]
    fn infer_word_corrections_but_not_typing_or_sentences() {
        assert_eq!(
            infer_rule("call Andromida tomorrow", "call Andromeda tomorrow"),
            Some(r("Andromida", "Andromeda"))
        );
        assert_eq!(
            infer_rule("我们去新秀公园。", "我们去新绣公园。"),
            Some(r("秀", "绣"))
        );
        assert_eq!(infer_rule("hello", "hello world"), None);
        assert_eq!(infer_rule("hello world", "hello"), None);
        assert_eq!(infer_rule("hello.", "hello!"), None);
        assert_eq!(
            infer_rule("write a long report", "send the draft now"),
            None
        );
    }
    #[cfg(feature = "local-only")]
    #[tokio::test]
    async fn local_only_rejects_cloud_text_before_creating_a_request() {
        let provider = crate::settings::PostProcessProvider {
            id: "test".into(),
            label: "test".into(),
            base_url: "invalid-url".into(),
            allow_base_url_edit: false,
            models_endpoint: None,
            supports_structured_output: false,
        };
        assert_eq!(
            crate::llm_client::send_chat_completion(
                &provider,
                "unused".into(),
                "test",
                "private test text".into(),
                false
            )
            .await,
            Err("Cloud processing is disabled in Handy Local".into())
        );
        assert_eq!(
            crate::llm_client::fetch_models(&provider, "unused".into()).await,
            Err("Cloud processing is disabled in Handy Local".into())
        );
        assert!(crate::settings::update_checks_forced_disabled());
    }
    #[test]
    fn dictionary_persists_only_confirmed_rules() {
        let path =
            std::env::temp_dir().join(format!("handy-corrections-{}.json", std::process::id()));
        let mut state = State {
            dictionary: Dictionary::default(),
            path: path.clone(),
            session: None,
            next_id: 0,
            storage_error: false,
        };
        let d = Dictionary {
            enabled: true,
            rules: vec![r("新秀公园", "新绣公园")],
        };
        persist(&mut state, d).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("session"));
        let loaded: Dictionary = serde_json::from_str(&raw).unwrap();
        assert_eq!(loaded.rules, state.dictionary.rules);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::remove_file(path).unwrap();
    }
}
