# Handy Local — local corrections fork

Originally based on Handy v0.9.8 (14f6f0d31cb22a4268acfbddba3052820fd0a06e).
The current integrated release is recorded in [.github/upstream-sync.json](.github/upstream-sync.json).
See the [upstream sync guide](docs/UPSTREAM_SYNC.zh-CN.md) for the preservation and validation policy.

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
  immediately before the paste. Allow up to 3 seconds for the actual pasted
  baseline to settle. During this initial handshake only, accept CRLF/LF and
  trailing-newline normalization when the exact dictated text is unambiguous
  and the surrounding words match. Subsequent context checks stay strict.
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
- The sidebar's Dictionary page combines custom vocabulary and replacement rules.
  It can add rules without a recent dictation and remove existing rules.
- Last text and field context live in memory for at most 120 seconds, cleared on
  next recording; only confirmed phrase pairs are written to
  `~/Library/Application Support/com.gabrielle.handylocal/local_corrections.json`
  using an atomic rename and owner-only file permissions.

## Local vocabulary

Dictionary → Custom vocabulary accepts correct spellings without a misspelling.
Words remain in `settings_store.json` under the existing `custom_words` key;
there is no migration of replacement pairs. Add, edit, and remove are persisted
before success is shown. Replacement rules remain separate in
`local_corrections.json`, and their enable switch does not disable vocabulary.
The existing JSON transfer controls transfer replacement rules only, as labeled.

Qwen3-ASR receives bounded vocabulary as native system context through a pinned
local patch of transcribe.cpp 0.2.4 (see `src-tauri/vendor/PATCHES.md`). Whisper
keeps its native prompt; other engines keep their existing ASCII fuzzy fallback.
Qwen/Whisper output is not corrected a second time by the fuzzy matcher.

For Qwen with a nonempty vocabulary, a local unprompted pass protects explicitly
spoken spelling (such as 木头的木 or “spelled ...”); such utterances keep that
result. Other utterances get a second, prompted pass. A failed/blank hint pass
keeps the complete baseline, while cancellation still propagates. This adds
one local decode for most utterances with vocabulary. An empty vocabulary still
uses a single decode. Hints are not model training and do not guarantee spelling.

Limit: 32 terms, 50 Unicode characters per term, 400 total. Legacy oversized
lists are preserved, remain removable, and are bounded when building a prompt.
No dictionary list is added to DeepSeek requests. If cloud polishing is enabled,
the resulting transcription text is still sent as before.

## Dictionary transfer

Local corrections → Remembered words has Export dictionary / Import dictionary
buttons (English, Simplified Chinese and Traditional Chinese). Export chooses a
local JSON file containing only confirmed `from`/`to` rules, tagged
`handy-local-dictionary` version 1. It excludes recordings, transcripts, model
files, and settings. Import also accepts the old `local_corrections.json` file.

Import previews additions, duplicates and conflicts before changing anything.
Duplicates are skipped; conflicts default to keeping the current computer's
version, with an explicit option to use the imported version. The enabled setting
is preserved. Any changed import first saves the previous dictionary to
`local_corrections.before-import.json` in the app data directory. A failed backup
or a dictionary changed since preview prevents the import. Invalid files are
rejected in full; limits are 8 MiB and 10,000 rules. All file processing is local.

## Privacy boundary

The `local-only` Cargo feature blocks the inherited generic remote
chat-completion/model-list requests and locks automatic update checks off.
The separate `cloud_polish` module permits only an explicitly enabled, fixed
DeepSeek text endpoint. Saving a key does not enable it; audio inference and
dictionary corrections remain local. Dictionary corrections run before optional
polishing, and that exact pre-polish text is kept for fallback and recovery.
The correction code has no network client. The local app's webview CSP disallows
external fetch connections. Model downloads remain available when explicitly
requested in the model picker; downloaded model files reuse the Hugging Face
cache. This is not a process-wide firewall. Text deliberately dictated into an
online application follows that application's own transmission behavior.

Cloud polishing uses the benchmarked conservative prompt, disables thinking,
does not follow redirects, and has a 12-second request timeout. Incomplete,
empty, invalid and failed responses fall back to the original. Original and
polished text remain separate in History; the latest result also lives in memory
until cleared or exit, so recovery does not depend on successfully saving audio.
Restore copies the original for explicit pasting; it does not replace text in
an arbitrary foreground application. The user-owned key stays in local settings
with redacted debug formatting and owner-only file permissions on Unix.

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
bunx playwright test tests/corrections.spec.ts tests/dictionary-transfer.spec.ts --reporter=line
python3 scripts/check_local_fork.py
cargo test --manifest-path src-tauri/Cargo.toml --features local-only cloud_polish --lib
bunx playwright test tests/cloud-polish.spec.ts --reporter=line
```

The UI tests mock the IPC boundary; they do not prove real macOS accessibility
compatibility. Real Fn, microphone capture and accessibility edits require a
native app smoke test. Unsupported browser/editors retain the manual correction
and explicit-copy route.
