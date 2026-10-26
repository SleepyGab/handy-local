# nibi

Local voice dictation for macOS, with custom vocabulary, a correction dictionary,
and optional DeepSeek text polishing.

按住快捷键说话，松开即可输入。支持本地语音识别、自定义词汇、确认后记住纠正，
以及可选的 DeepSeek 文字润色与长口述整理。

nibi (formerly Handy Local) is an independent community fork of
[Handy](https://github.com/cjpais/Handy), originally based on **v0.9.8**.
Handy provides the local dictation engine, model manager, shortcuts and core UI;
nibi adds vocabulary tools, explicit correction memory and optional text polishing,
with its own name and artwork. It is not an official Handy release or an endorsed
project. The [upstream baseline](.github/upstream-sync.json) records the currently
integrated official release.

## Features

- **Dictate into the current text field.** Hold a configurable shortcut (including
  Fn on macOS), speak, then release to transcribe locally and paste. Toggle mode
  is also available. Download and select a local speech recognition model,
  including Qwen3-ASR or Whisper.
- **Teach names and terms.** Add correct spellings under **Dictionary → Custom
  vocabulary**, without supplying a misspelling. Qwen3-ASR and Whisper use them as
  local recognition hints; other engines use approximate English-word correction.
  Hints do not guarantee spelling and do not train the model.
- **Remember corrections with your approval.** A supported edit to the last
  dictated text can show a compact **Add?** prompt. Nothing is remembered until
  you confirm. **Correct Last Dictation** in the menu bar provides a manual route;
  replacement rules can also be added and removed in Dictionary.
- **Move replacement rules between Macs.** Export/import confirmed correction
  pairs as JSON, preview conflicts, skip duplicates and back up the old dictionary
  before changes. This transfer does not include custom vocabulary, models or History.
- **Optionally polish and organize speech.** Use your own DeepSeek API key to
  remove filler words and improve punctuation, wording and paragraphs. Longer
  dictation can become clear notes, with tasks, discussion points, decisions and
  follow-ups organized where appropriate. The prompt is designed to preserve
  substantive details, original languages, uncertainty and conditions.
- **Keep the original available.** Polishing is off by default and falls back to
  the original on errors. The latest result and saved History keep both versions;
  **Restore original** copies the original for you to paste.
- **Make it comfortable.** Light/dark themes, nibi artwork, and correction and
  dictionary UI in English, Simplified Chinese and Traditional Chinese.

Correction detection depends on macOS Accessibility support in the destination
editor. It observes only the last dictated field for up to 45 seconds; changing
focus stops observation. Whole-sentence rewrites and pure deletions are not learned.
Replacement rules apply exact phrases after transcription; they are separate from
vocabulary hints and model training. Text polishing can still make mistakes, so
review important output.

## Install and availability

[Release downloads](https://github.com/SleepyGab/nibi/releases)
· [English installation guide](docs/INSTALL.en.md)
· [中文安装说明](docs/INSTALL.zh-CN.md)

The current source is **nibi v0.0.1**. The available release download is the older
**Handy Local v0.9.8-local.1** preview; it predates the nibi rebrand, custom
vocabulary and DeepSeek polishing described above. Build from the `nibi` branch
for the current features. The old package keeps its original name.

nibi is an **early macOS Apple Silicon preview**, **not notarized by Apple**.
Current local builds use a stable certificate identity; older preview packages
used ad-hoc signing. It has been tested on one Apple Silicon Mac; a second-machine
installation still needs validation. Intel, Windows and Linux builds of this fork
have not been validated. Models are downloaded separately.

## Privacy

nibi's `build:local` command enables the `local-only` Cargo feature. It blocks the
inherited generic cloud post-processing/model-list calls and automatic app update
checks. Speech recognition, vocabulary hints and dictionary corrections run locally.
There is no account or cloud dictionary sync.

**DeepSeek text polishing is optional and off by default.** Saving a key alone
does not enable it. When explicitly enabled, completed transcription text is sent
to DeepSeek using your API key and billed to your account. Audio is never sent to
this API. The vocabulary list is not attached to the request, but names and terms
present in the resulting transcript are included in the text sent for polishing.
Explicit model downloads still use the network.

The API key is stored in the local settings file (restricted to the current user
on macOS/Linux), not bundled with the app. The latest polishing result stays in
memory until cleared or the app quits; saved History keeps both text versions.
Audio/history storage follows the local History settings; inspect or clear it
there. Exported dictionaries contain only confirmed correction pairs.

This is not a system-wide network firewall. Text pasted into an online app is
subject to that app's own behavior. Exporting into an iCloud/Dropbox folder lets
that service sync the file; choose a local folder if you do not want that.

## Build nibi

Install Rust, Bun, Xcode command line tools and CMake; see [BUILD.md](BUILD.md)
for the inherited platform prerequisites. From this checkout:

```sh
bun install --frozen-lockfile
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run build:local
```

Always use `build:local` for nibi. A plain upstream build command does not include
the local-only feature, the fork's separate application identity or stable macOS
signing. On macOS, a code-signing certificate is required; see the
[signing setup and permission recovery guide](docs/INSTALL.en.md#local-builds-and-permission-recovery).
The output is `src-tauri/target/release/bundle/macos/nibi.app`.

The default branch is `nibi` (previously `handy-local`); `main` retains upstream
Handy code. The existing app identifier and data directory are retained so the
rename preserves Handy Local settings and dictionaries.

Implementation, limitations and focused test commands: [LOCAL_CORRECTIONS.md](LOCAL_CORRECTIONS.md).
The original README is preserved in [README.upstream.md](README.upstream.md).
Inherited upstream CI/release workflows are archived in `.github/upstream-workflows`
and do not run in this fork. They refer to upstream infrastructure and signing keys.

Official stable releases are checked weekly, with isolated sync branches and local
correction validation. See the [sync guide](docs/UPSTREAM_SYNC.zh-CN.md) for manual
checks, conflict handling, review and release steps. Nothing is automatically merged
or published to app users.

## Contribute / upstream

For nibi, read [CONTRIBUTING_LOCAL.md](CONTRIBUTING_LOCAL.md).
For contributions to Handy itself, follow its current [contribution guidelines](https://github.com/cjpais/Handy/blob/main/CONTRIBUTING.md).
Keep each proposed upstream change small and discuss new features first.

## Credits and license

MIT; see [LICENSE](LICENSE). Original copyright: CJ Pais and the Handy contributors.
nibi (formerly Handy Local) additions: SleepyGab and contributors. The original
copyright notice is retained. Dependencies and separately downloaded models retain
their own licenses. See [NOTICE.md](NOTICE.md). This fork was developed with
substantial OpenAI Codex assistance and manually tested by its maintainer; that is
not an independent audit.
