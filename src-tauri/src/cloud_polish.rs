//! Explicitly opted-in, text-only DeepSeek polishing for Handy Local.
//! The generic cloud client remains blocked by the local-only feature.

use crate::settings::{get_settings, AppSettings, SETTINGS_STORE_PATH};
use reqwest::{redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

const KEY_ID: &str = "deepseek";
const ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
pub(crate) const PROMPT: &str = include_str!("../resources/prompts/deepseek-polish.txt");
const MAX_INPUT_CHARS: usize = 20_000;

#[derive(Clone, Serialize, specta::Type)]
pub struct LastPolish {
    pub original: String,
    pub polished: Option<String>,
    pub fallback: bool,
}

#[derive(Default)]
pub struct CloudPolishState {
    last: Mutex<Option<LastPolish>>,
}

#[derive(Serialize, specta::Type)]
pub struct CloudPolishStatus {
    pub enabled: bool,
    pub key_configured: bool,
    pub key_preview: Option<String>,
    pub last: Option<LastPolish>,
}

fn api_key(settings: &AppSettings) -> &str {
    settings
        .post_process_api_keys
        .get(KEY_ID)
        .map(|key| key.trim())
        .unwrap_or_default()
}

fn eligible(settings: &AppSettings, text: &str) -> bool {
    settings.cloud_polish_enabled && !api_key(settings).is_empty() && !text.trim().is_empty()
}

fn key_preview(settings: &AppSettings) -> Option<String> {
    let key = api_key(settings);
    if key.is_empty() {
        return None;
    }
    // Never reveal an entire malformed/legacy short key in the status response.
    let suffix: String = if key.chars().count() > 4 {
        key.chars().skip(key.chars().count() - 4).collect()
    } else {
        String::new()
    };
    Some(format!("••••••••{suffix}"))
}

fn replace_key(settings: &mut AppSettings, key: &str) {
    if key.is_empty() {
        settings.post_process_api_keys.remove(KEY_ID);
        settings.cloud_polish_enabled = false;
    } else {
        settings
            .post_process_api_keys
            .insert(KEY_ID.into(), key.into());
    }
}

fn save_settings(app: &AppHandle, settings: AppSettings) -> Result<(), String> {
    let store = app
        .store(crate::portable::store_path(SETTINGS_STORE_PATH))
        .map_err(|_| "save_failed")?;
    // The existing settings store also owns provider keys. Restrict its file
    // before writing a new credential, including when this is the first save.
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let path = crate::portable::app_data_dir(app)
            .map_err(|_| "save_failed")?
            .join(SETTINGS_STORE_PATH);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|_| "save_failed")?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "save_failed")?;
    }
    let previous = store.get("settings");
    store.set(
        "settings",
        serde_json::to_value(settings).map_err(|_| "save_failed")?,
    );
    if store.save().is_err() {
        if let Some(previous) = previous {
            store.set("settings", previous);
        } else {
            store.delete("settings");
        }
        return Err("save_failed".into());
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn cloud_polish_status(app: AppHandle) -> Result<CloudPolishStatus, String> {
    let settings = get_settings(&app);
    let last = app
        .state::<CloudPolishState>()
        .last
        .lock()
        .map_err(|_| "state_unavailable")?
        .clone();
    Ok(CloudPolishStatus {
        enabled: settings.cloud_polish_enabled,
        key_configured: !api_key(&settings).is_empty(),
        key_preview: key_preview(&settings),
        last,
    })
}

/// Return the saved key only for the user's explicit reveal action.
#[tauri::command]
#[specta::specta]
pub fn reveal_cloud_polish_key(app: AppHandle) -> String {
    api_key(&get_settings(&app)).to_owned()
}

#[tauri::command]
#[specta::specta]
pub fn set_cloud_polish_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    if enabled && api_key(&settings).is_empty() {
        return Err("missing_key".into());
    }
    settings.cloud_polish_enabled = enabled;
    save_settings(&app, settings)?;
    let _ = app.emit("cloud-polish-changed", ());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_cloud_polish_key(app: AppHandle, key: String) -> Result<(), String> {
    let key = key.trim();
    if key.chars().any(|c| c.is_whitespace() || c.is_control())
        || (!key.is_empty() && key.len() < 16)
    {
        return Err("invalid_key".into());
    }
    let mut settings = get_settings(&app);
    replace_key(&mut settings, key);
    save_settings(&app, settings)?;
    let _ = app.emit("cloud-polish-changed", ());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn clear_last_cloud_polish(app: AppHandle) -> Result<(), String> {
    *app.state::<CloudPolishState>()
        .last
        .lock()
        .map_err(|_| "state_unavailable")? = None;
    let _ = app.emit("cloud-polish-changed", ());
    Ok(())
}

fn request_body(text: &str) -> serde_json::Value {
    serde_json::json!({
        "model": "deepseek-flash",
        "messages": [
            {"role": "system", "content": PROMPT.trim()},
            {"role": "user", "content": format!("<transcript>\n{text}\n</transcript>")}
        ],
        "thinking": {"type": "disabled"},
        "temperature": 0,
        "max_tokens": 8192,
        "stream": false
    })
}

#[derive(Deserialize)]
struct Completion {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    finish_reason: Option<String>,
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

fn completed_text(completion: Completion) -> Result<String, &'static str> {
    let choice = completion.choices.into_iter().next().ok_or("empty")?;
    if choice.finish_reason.as_deref() != Some("stop") {
        return Err("incomplete");
    }
    let text = choice
        .message
        .content
        .unwrap_or_default()
        .trim()
        .to_string();
    if text.is_empty() || text.contains("<think>") || text.contains("</think>") {
        return Err("empty_or_reasoning");
    }
    Ok(text)
}

async fn request_polish(
    client: &Client,
    endpoint: &str,
    key: &str,
    text: &str,
) -> Result<String, &'static str> {
    let response = client
        .post(endpoint)
        .bearer_auth(key)
        .json(&request_body(text))
        .send()
        .await
        .map_err(|_| "network_or_timeout")?;
    if !response.status().is_success() {
        // Never log a provider response that could echo the key or transcript.
        return Err("api_error");
    }
    let completion = response.json().await.map_err(|_| "invalid_response")?;
    completed_text(completion)
}

fn result_snapshot(original: &str, result: Result<String, &'static str>) -> LastPolish {
    LastPolish {
        original: original.to_string(),
        fallback: result.is_err(),
        polished: result.ok(),
    }
}

/// Accepts only completed local transcription text; no audio or file path enters
/// this API. Disabled/missing-key/blank cases return before constructing a client.
pub(crate) async fn polish(app: &AppHandle, original: &str) -> Option<String> {
    let settings = get_settings(app);
    if !eligible(&settings, original) {
        return None;
    }
    let result = if original.chars().count() > MAX_INPUT_CHARS {
        Err("input_too_long")
    } else {
        match Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(12))
            .redirect(Policy::none())
            .build()
        {
            Ok(client) => request_polish(&client, ENDPOINT, api_key(&settings), original).await,
            Err(_) => Err("client_unavailable"),
        }
    };
    // If the user turns this off or changes the key during a request, discard
    // the result. This cannot retract text already sent, but prevents its use.
    let current = get_settings(app);
    let result = if !current.cloud_polish_enabled || api_key(&current) != api_key(&settings) {
        Err("settings_changed")
    } else {
        result
    };
    if let Err(reason) = &result {
        log::warn!("DeepSeek polishing returned the original text: {reason}");
        let _ = app.emit("cloud-polish-fallback", ());
    }
    let snapshot = result_snapshot(original, result);
    let polished = snapshot.polished.clone();
    if let Ok(mut last) = app.state::<CloudPolishState>().last.lock() {
        *last = Some(snapshot);
    }
    let _ = app.emit("cloud-polish-changed", ());
    polished
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn saved_key_status_is_masked_and_replacement_does_not_keep_old_keys() {
        let mut settings: AppSettings = serde_json::from_str("{}").unwrap();
        replace_key(&mut settings, "sk-test-credential-1234");
        assert!(!settings.cloud_polish_enabled);
        let status = CloudPolishStatus {
            enabled: false,
            key_configured: true,
            key_preview: key_preview(&settings),
            last: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("••••••••1234"));
        assert!(!json.contains(api_key(&settings)));
        settings.cloud_polish_enabled = true;
        replace_key(&mut settings, "sk-new-credential-5678");
        assert!(settings.cloud_polish_enabled);
        assert!(!serde_json::to_string(&settings)
            .unwrap()
            .contains("sk-test-credential-1234"));
        assert_eq!(key_preview(&settings).as_deref(), Some("••••••••5678"));
    }

    #[test]
    fn removing_saved_key_erases_credential_and_disables_polishing() {
        let mut settings: AppSettings = serde_json::from_str("{}").unwrap();
        replace_key(&mut settings, "sk-test-credential-1234");
        settings.cloud_polish_enabled = true;
        settings
            .post_process_api_keys
            .insert("other-provider".into(), "untouched".into());
        replace_key(&mut settings, "");
        assert!(!settings.post_process_api_keys.contains_key(KEY_ID));
        assert!(!settings.cloud_polish_enabled);
        assert!(key_preview(&settings).is_none());
        assert_eq!(
            settings
                .post_process_api_keys
                .get("other-provider")
                .unwrap(),
            "untouched"
        );
        assert!(!serde_json::to_string(&settings)
            .unwrap()
            .contains("sk-test-credential-1234"));
        replace_key(&mut settings, "tiny");
        assert_eq!(key_preview(&settings).as_deref(), Some("••••••••"));
    }

    #[test]
    fn disabled_and_legacy_settings_never_opt_in() {
        let mut settings: AppSettings = serde_json::from_str("{}").unwrap();
        settings
            .post_process_api_keys
            .insert(KEY_ID.into(), "test-key".into());
        settings.post_process_enabled = true;
        assert!(!eligible(&settings, "中文 PR"));
        settings.cloud_polish_enabled = true;
        assert!(eligible(&settings, "中文 PR"));
        assert!(!eligible(&settings, "  \n"));
        settings.post_process_api_keys.remove(KEY_ID);
        assert!(!eligible(&settings, "中文 PR"));
    }

    #[test]
    fn request_is_text_only_with_thinking_disabled() {
        let body = request_body("中文 PR，不要 merge。");
        assert_eq!(body["model"], "deepseek-flash");
        assert_eq!(body["thinking"]["type"], "disabled");
        assert_eq!(
            body["messages"][1]["content"],
            "<transcript>\n中文 PR，不要 merge。\n</transcript>"
        );
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        assert!(body.get("audio").is_none());
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn incomplete_empty_and_reasoning_responses_preserve_original() {
        for (finish, content) in [
            ("length", "partial"),
            ("stop", "  "),
            ("stop", "<think>reasoning</think>text"),
            ("content_filter", ""),
        ] {
            let completion = serde_json::from_value(serde_json::json!({"choices": [{"finish_reason": finish, "message": {"content": content}}]})).unwrap();
            let result = result_snapshot("原文 0.5%", completed_text(completion));
            assert_eq!(result.original, "原文 0.5%");
            assert!(result.fallback);
            assert!(result.polished.is_none());
        }
    }

    fn mock_server(
        status: &str,
        body: &str,
        delay: Duration,
    ) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/chat/completions", listener.local_addr().unwrap());
        let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let handle = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buf = [0u8; 4096];
                let size = socket.read(&mut buf).unwrap();
                if size == 0 {
                    break;
                }
                bytes.extend_from_slice(&buf[..size]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            std::thread::sleep(delay);
            let _ = socket.write_all(response.as_bytes());
            String::from_utf8(bytes).unwrap()
        });
        (url, handle)
    }

    #[tokio::test]
    async fn success_preserves_both_versions_and_failures_return_original() {
        let client = Client::builder()
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();
        let (url, worker) = mock_server(
            "200 OK",
            r#"{"choices":[{"finish_reason":"stop","message":{"content":"整理后的文字。"}}]}"#,
            Duration::ZERO,
        );
        let result = result_snapshot(
            "整理前的文字",
            request_polish(&client, &url, "fake-key", "整理前的文字").await,
        );
        assert_eq!(result.original, "整理前的文字");
        assert_eq!(result.polished.as_deref(), Some("整理后的文字。"));
        assert!(!result.fallback);
        assert!(worker.join().unwrap().contains("Bearer fake-key"));
        for (status, body, delay) in [
            ("401 Unauthorized", "secret-echo", Duration::ZERO),
            ("200 OK", "invalid", Duration::ZERO),
            ("200 OK", "{}", Duration::from_millis(250)),
        ] {
            let (url, worker) = mock_server(status, body, delay);
            let result = result_snapshot(
                "原文",
                request_polish(&client, &url, "fake-key", "原文").await,
            );
            assert!(result.fallback);
            assert_eq!(result.original, "原文");
            assert!(result.polished.is_none());
            worker.join().unwrap();
        }
    }

    #[tokio::test]
    #[ignore = "Requires an explicitly provided test credential; sends synthetic text only"]
    async fn live_deepseek_text_smoke() {
        let key = std::env::var("HANDY_DEEPSEEK_SMOKE_KEY").expect("test credential required");
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(12))
            .redirect(Policy::none())
            .build()
            .unwrap();
        let text = request_polish(
            &client,
            ENDPOINT,
            &key,
            "嗯这个 PR 先不要 merge 等 CI 全绿了再说",
        )
        .await
        .expect("DeepSeek text request must complete");
        assert!(text.contains("不要 merge"));
        assert!(text.contains("CI"));
    }
}
