# Handy Local — local corrections fork

Based on Handy v0.9.8 (14f6f0d31cb22a4268acfbddba3052820fd0a06e).

## Behavior

- Keep the existing Fn hold/release dictation flow and local Qwen model.
- Normal dictation stays silent, including fields without accessibility support.
- Open “Correct Last Dictation…” from the Handy menu bar icon; the item is enabled
  only while the recent in-memory dictation is available and recording is idle.
- Only a detected word edit shows a 42px-high, single-row nonactivating capsule.
  It shares Handy’s palette, with live interface-language
  updates ("记住？" / "Add?"). Click the phrase to open the full editor.
  Successful saves dismiss themselves after a short acknowledgement.
- On macOS, observe only the exact focused accessibility text field captured
  immediately before the paste. Confirm the actual pasted baseline first.
- Watch for up to 45 seconds, stopping when focus changes, surrounding text
  changes, the next recording starts, or corrections are disabled.
- Retry at most three transient AX read gaps on the original captured field; a
  confirmed focus change stops immediately. Never reacquire a different field.
  Local detection logs contain only fixed status categories and session counters,
  never text, selected words, window titles, or application names.
- After an edit is stable for 1.2 seconds and no text is selected, propose a
  small replacement. Additions, deletions, punctuation edits and sentence
  rewrites do not automatically become dictionary entries.
- Nothing is learned until the user clicks Remember. The editor lets the user
  choose a complete phrase and correct spelling, with a warning for single
  characters. Rules use case-sensitive exact matching, ASCII word boundaries,
  longest match first, and no cascading replacements.
- Try to update the original text using its captured accessibility element only
  if its entire value still matches the expected value. Otherwise offer an
  explicit Copy action. Never paste into whichever window happens to be active.
- The sidebar's Local corrections page can disable the feature or delete rules.
- Last text and field context live in memory for at most 120 seconds, cleared on
  next recording; only confirmed phrase pairs are written to
  `~/Library/Application Support/com.gabrielle.handylocal/local_corrections.json`
  using an atomic rename and owner-only file permissions.

## Privacy boundary

The `local-only` Cargo feature blocks remote chat-completion/model-list requests,
ignores dictation post-processing, and locks automatic update checks off. The
new correction code has no network client. The local app's webview CSP disallows
external fetch connections. Model downloads remain available when explicitly
requested in the model picker; downloaded model files reuse the Hugging Face
cache. This is not a process-wide firewall. Text deliberately dictated into an
online application follows that application's own transmission behavior.

## Build

```sh
export PATH="$HOME/.cargo/bin:$PATH"
bun install --frozen-lockfile
# Requires Xcode command line tools, cmake, Rust, Bun and the existing VAD resource.
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri build \
  --config src-tauri/tauri.corrections.conf.json --features local-only --bundles app
```

Output: `src-tauri/target/release/bundle/macos/Handy Local.app`.
The original `/Applications/Handy.app` and its app data are separate. The custom
bundle is ad-hoc signed; macOS may require its own microphone and accessibility
permissions. Do not run both copies with the Fn binding simultaneously.

## Validation

```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test --manifest-path src-tauri/Cargo.toml \
  --features local-only local_corrections --lib
bun run build
bun run lint
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo clippy --manifest-path src-tauri/Cargo.toml \
  --features local-only --lib
bunx playwright test tests/corrections.spec.ts --reporter=line
```

The UI tests mock the IPC boundary; they do not prove real macOS accessibility
compatibility. Real Fn, microphone capture and accessibility edits require a
native app smoke test. Unsupported browser/editors retain the manual correction
and explicit-copy route.
