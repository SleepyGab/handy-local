# nibi — Apple Silicon installation

Current source: nibi v0.0.1, for Apple Silicon Macs. Use a recent macOS version;
older macOS and Intel have not been validated.

The [release download](https://github.com/SleepyGab/nibi/releases) is still the older
**Handy Local v0.9.8-local.1** preview. Its app is named **Handy Local.app** and it
does not include the nibi rebrand, custom vocabulary or DeepSeek polishing.
The steps below describe current nibi builds; for the legacy download, use its
bundled installation guide. To build nibi, see [Build nibi](../README.md#build-nibi).

1. Copy **nibi.app** from your build output into **Applications** (or drag it
   from a nibi DMG when one is available, then eject the DMG). Launch that copy.
   Quit original Handy/Wispr Flow if they use Fn.
2. This local preview is signed, but not Apple-notarized. If macOS blocks it, confirm
   that it came from the maintainer/release page, then use **System Settings →
   Privacy & Security → Open Anyway** yourself. Do not disable Gatekeeper globally.
   If macOS reports damage or malware, stop and obtain a verified copy.
   Apple's guidance: https://support.apple.com/en-us/102445
3. Allow **Microphone** and **Accessibility** (called **Device Control and Data
   Access** on some macOS versions). These allow recording, observing the last
   dictated text field and inserting text. Restart nibi after permission setup.
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
this package does not copy the developer's settings, recordings or remembered words.

Speech recognition and dictionary corrections are local. **Text polishing** is
optional and off by default: save your own DeepSeek API key in settings, then
explicitly enable it to send transcription text (never audio) to DeepSeek.
Polishing can also organize longer dictation into notes, task lists, or discussion,
decision and action lists where appropriate. Review important output for accuracy.
The saved-key card shows a masked key and its final four characters. **Show key**
reveals it on request; **Remove key** deletes it and turns polishing off. Only one
DeepSeek key is stored, and saving another replaces it.
Errors fall back to the original. **Restore original** in the latest result or
History copies the original for you to paste. Explicit model downloads use the
network, and the app you dictate into may itself send text online. For local-only
backups, save exported dictionaries outside cloud-synced folders.

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
