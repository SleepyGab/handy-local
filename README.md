# Handy Local

Local voice dictation with a correction dictionary that stays on your Mac.

An independent community fork of [Handy](https://github.com/cjpais/Handy),
originally based on **v0.9.8**. The [upstream baseline](.github/upstream-sync.json)
records the currently integrated official release. Not an official Handy release or an endorsed project.
Handy provides the dictation engine, model manager, shortcuts and application UI.
This fork adds explicit local correction memory and a compact correction prompt.

The default branch is `nibi` (previously `handy-local`). Merge feature work and
official stable-release updates into `nibi`; `main` retains upstream Handy code.

## Install

[Download the Apple Silicon preview](https://github.com/SleepyGab/nibi/releases)
· [English installation guide](docs/INSTALL.en.md)
· [中文安装说明](docs/INSTALL.zh-CN.md)

Handy Local is an **early macOS Apple Silicon preview**, **not notarized by Apple**.
Current local builds reuse a certificate identity; older preview packages used ad-hoc signing. It has been tested on one Apple Silicon Mac;
a second-machine installation still needs validation. Intel, Windows and Linux
builds of this fork have not been validated. Models are downloaded separately.

## What is different?

- Hold a configurable shortcut (including Fn on macOS) to dictate; release to
  transcribe locally and paste into the current text field.
- Normal dictation stays quiet. A supported edit to the last dictated text can
  show a narrow correction capsule; nothing is remembered until you click **Add?**.
- **Correct Last Dictation** in the menu bar provides a manual route.
- Confirmed phrase replacements stay in a local dictionary, with delete controls.
- Optional **Text polishing** uses your own DeepSeek API key after local speech
  recognition. It is off by default, sends text only, and falls back to the
  original on errors. Latest results and History let you restore/copy the original.
- Export/import the dictionary as JSON, preview conflicts, skip duplicates and
  back up the old dictionary before changes.
- Light/dark styling and localized correction UI (English, Simplified Chinese,
  Traditional Chinese), including a flowing pink border and reduced-motion support.

Correction detection depends on macOS Accessibility support in the destination
editor. It observes only the last dictated field for up to 45 seconds; changing
focus stops observation. Whole-sentence rewrites and pure deletions are not learned.
This is exact phrase replacement after transcription, not model fine-tuning.

## Privacy

Published packages use the `local-only` Cargo feature. It blocks the inherited
generic cloud post-processing/model-list calls and automatic app update checks.
The separate DeepSeek text-polishing option is off by default; only explicitly
enabling it sends completed transcription text to DeepSeek, billed to your own
account. Saving a key alone does not enable it. Audio inference and dictionary
corrections run locally; recordings are never sent to this API. Explicit model
downloads still use the network. Audio/history storage follows Handy's local History settings; inspect or
clear it there. The portable dictionary contains only confirmed correction pairs.
There is no account or cloud dictionary sync.

The API key is stored in the local settings file (restricted to the current user
on macOS/Linux), not bundled with the app. The latest polishing result stays in
memory until cleared or the app quits; saved History keeps both text versions.
**Restore original** copies it for you to paste in place of the polished text.

This is not a system-wide network firewall. Text pasted into an online app is
subject to that app's own behavior. Exporting into an iCloud/Dropbox folder lets
that service sync the file; choose a local folder if you do not want that.

## Build this fork

Install Rust, Bun, Xcode command line tools and CMake; see [BUILD.md](BUILD.md)
for the inherited platform prerequisites. From this checkout:

```sh
bun install --frozen-lockfile
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run build:local
```

Always use `build:local` for Handy Local. A plain upstream build command does not
include the local-only feature, the fork's separate application identity or stable
macOS signing. On macOS, a code-signing certificate is required; see the
[signing setup and permission recovery guide](docs/INSTALL.en.md#local-builds-and-permission-recovery).
The output is `src-tauri/target/release/bundle/macos/Handy Local.app`.

Implementation, limitations and focused test commands: [LOCAL_CORRECTIONS.md](LOCAL_CORRECTIONS.md).
The original README is preserved in [README.upstream.md](README.upstream.md).
Inherited upstream CI/release workflows are archived in `.github/upstream-workflows`
and do not run in this fork. They refer to upstream infrastructure and signing keys.

Official stable releases are checked weekly, with isolated sync branches and local
correction validation. See the [sync guide](docs/UPSTREAM_SYNC.zh-CN.md) for manual
checks, conflict handling, review and release steps. Nothing is automatically merged
or published to app users.

## Contribute / upstream

For this fork, read [CONTRIBUTING_LOCAL.md](CONTRIBUTING_LOCAL.md).
For contributions to Handy itself, follow its current [contribution guidelines](https://github.com/cjpais/Handy/blob/main/CONTRIBUTING.md).
Keep each proposed upstream change small and discuss new features first.

## Credits and license

MIT; see [LICENSE](LICENSE). Original copyright: CJ Pais and the Handy contributors.
Handy Local additions: SleepyGab and contributors. The original copyright notice
is retained. Dependencies and separately downloaded models retain their own licenses.
See [NOTICE.md](NOTICE.md). This fork was developed with substantial OpenAI Codex
assistance and manually tested by its maintainer; that is not an independent audit.
