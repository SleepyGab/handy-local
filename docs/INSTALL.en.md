# nibi — Apple Silicon installation

Current source: nibi v0.0.2-preview.1, for Apple Silicon Macs. Use a recent macOS version;
older macOS and Intel have not been validated.

Download **nibi-v0.0.2-preview.1-macos-apple-silicon.dmg** from the
[preview release](https://github.com/SleepyGab/nibi/releases/tag/v0.0.2-preview.1).
It includes provider selection, separate meeting/journal prompts and configurable
polishing wait limits. This preview corresponds to the PR commit linked on the release page.
The older Handy Local preview remains available in the release history; the steps
below describe nibi. To build from source, see [Build nibi](../README.md#build-nibi).

1. Open the DMG and drag **nibi.app** onto its **Applications** folder shortcut.
   This copies the app into Applications; it is the standard drag-and-drop install step.
   Eject the DMG when copying finishes.
   Launch the copy in Applications. Quit original Handy/Wispr Flow if they use Fn.
   When replacing an existing installation, quit nibi before copying the new app.
2. This preview uses an Apple Development certificate and is not Apple-notarized.
   Copying the app does not grant security approval. If macOS blocks it, confirm
   that it came from the maintainer/release page, then use **System Settings →
   Privacy & Security → Open Anyway** yourself. Do not disable Gatekeeper globally.
   If macOS reports damage or malware, stop and obtain a verified copy.
   Apple's guidance: https://support.apple.com/en-us/102445
3. After the app opens, allow **Microphone** and **Accessibility** (called **Device Control and Data
   Access** on some macOS versions). These allow recording, observing the last
   dictated text field and inserting text. These are separate from macOS approval
   to open the app. Restart nibi after permission setup.
4. In **Models**, download **Qwen3-ASR 1.7B**, choosing **Q5_K_M** when quantization
   is offered, then select it as the active model. The model is about 1.5 GB and
   is NOT included in the installer. Download requires internet; dictation afterward
   runs locally. Other available local models can be downloaded and selected too.
5. In **General → Transcribe Shortcut**, assign **Fn**, and set **Shortcut Behavior**
   to **Hold**. In macOS Keyboard settings, set “Press 🌐 key to” to “Do Nothing”
   if the system emoji/language action conflicts. For mixed Chinese/English, leave
   recognition Language on **Auto Detect**. Set nibi's interface language to
   **English** in its language setting if needed.
6. Click a text field, hold Fn, speak, then release. The transcription should paste
   into that field. First inference may take longer while the model loads.
7. In **Dictionary → Replacement rules**, confirmed edits can be remembered with **Add?**.
   Use the menu bar's **Correct Last Dictation** if an editor does not support
   automatic detection. **Export dictionary / Import dictionary** moves only
   confirmed phrase pairs between computers; it does not transfer custom vocabulary,
   models or History. Under **Dictionary → Custom vocabulary**, add correct spellings
   of names and terms as local recognition hints. Hints do not guarantee spelling.

Optional: enable launch at login, start hidden, and unload the model after 5 minutes
in nibi's settings. A fresh installation has its own preferences and empty dictionary;
the installer contains no preconfigured personal data.

Speech recognition and dictionary corrections are local. **Text polishing** is
optional and off by default. Leave it off for everyday dictation. Choose DeepSeek
(the first suggestion), OpenAI, Anthropic or Google Gemini, save your own API key,
then explicitly enable it to send transcription text (never audio) to that provider.
Choose general cleanup, meeting notes or journal mode. The model is pre-filled
and can be changed to another supported model ID. Review important output for accuracy.
General cleanup preserves the existing dictation cleanup behavior. Meeting notes
organize topics, decisions and supported action items; Journal keeps the speaker's
voice in natural paragraphs. Both prompts instruct the model to avoid inventing details.
**Polishing wait limit** offers 12 (default), 30, 60 or 120 seconds, starting when
the cloud request is sent after local transcription. It applies to the next request
and does not limit how long you can record. A timeout keeps the original text.
The saved-key card shows a masked key and its final four characters. **Show key**
reveals it on request; **Remove key** deletes it and turns polishing off. Each
provider has a separate saved key. Replacing one does not affect other providers.
Switching providers turns polishing off until you explicitly enable it again.
Errors fall back to the original. **Restore original** in the latest result or
History copies the original for you to paste. Explicit model downloads use the
network, and the app you dictate into may itself send text online. For local-only
backups, save exported dictionaries outside cloud-synced folders.

There is no fixed recording timer cutoff. Long-recording reliability still depends
on the local model: Parakeet Unified EN streams English; Qwen3-ASR and Cohere currently
process audio after stopping without automatic long-audio splitting. Meeting mode
only formats the resulting text. Hour-long meeting transcription has not been validated.

Updates use the same app data. Moving from an older ad-hoc preview to a certificate
identity may require re-granting permissions once. If an enabled Accessibility entry is stale, remove
and re-add nibi in System Settings, then restart. If a native Save dialog
stays disabled after an update, quit/reopen nibi before retrying.

## Local builds and permission recovery

`bun run build:local` now uses a stable code-signing certificate from the macOS
keychain. With one valid identity it is selected automatically; with more than
one, set `HANDY_MACOS_SIGNING_IDENTITY` to its exact name or SHA-1 fingerprint.
List identities with `security find-identity -v -p codesigning`. Keep using the
same identity for updates. No certificate or private key is stored in this repo.
Developer ID is appropriate for distribution; Apple Development can sign builds
for your own Mac. This does not add notarization.

Older ad-hoc builds identify each version by its binary hash. The first update
to certificate signing may require removing the old Handy Local or nibi permission entry
with the minus button, adding **/Applications/nibi.app** again and enabling
it. The app checks automatically on return and while the permission page is open.
After 15 seconds without a grant it shows recovery instructions, **Check again**
and **Restart nibi**, instead of an endless waiting spinner.

Builds without a certificate stop with setup instructions. For a disposable
ad-hoc build only, explicitly set `HANDY_ALLOW_ADHOC_SIGNING=1` and
`HANDY_MACOS_SIGNING_IDENTITY=-`; this loses stable permission identity.
