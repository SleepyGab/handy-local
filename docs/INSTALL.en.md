# Handy Local — Apple Silicon installation

Preview v0.9.8-local.1 · for Apple Silicon MacBooks/Mac minis, including M5/32 GB.
Use a recent macOS version. This preview has not been tested on older macOS or Intel.

1. Open the DMG and drag **Handy Local.app** into **Applications**. Eject the DMG;
   launch the copy in Applications. Quit original Handy/Wispr Flow if they use Fn.
2. This preview is ad-hoc signed, not Apple-notarized. If macOS blocks it, confirm
   that it came from the maintainer/release page, then use **System Settings →
   Privacy & Security → Open Anyway** yourself. Do not disable Gatekeeper globally.
   If macOS reports damage or malware, stop and obtain a verified copy.
   Apple's guidance: https://support.apple.com/en-us/102445
3. Allow **Microphone** and **Accessibility** (called **Device Control and Data
   Access** on some macOS versions). These allow recording, observing the last
   dictated text field and inserting text. Restart Handy Local after permission setup.
4. In **Models**, download **Qwen3-ASR 1.7B**, choosing **Q5_K_M** when quantization
   is offered, then select it as the active model. The model is about 1.5 GB and
   is NOT included in the installer. Download requires internet; dictation afterward
   runs locally. Other available local models can be downloaded and selected too.
5. In **General → Transcribe Shortcut**, assign **Fn**, and set **Shortcut Behavior**
   to **Hold**. In macOS Keyboard settings, set “Press 🌐 key to” to “Do Nothing”
   if the system emoji/language action conflicts. For mixed Chinese/English, leave
   recognition Language on **Auto Detect**. Set Handy's interface language to
   **English** in its language setting if needed.
6. Click a text field, hold Fn, speak, then release. The transcription should paste
   into that field. First inference may take longer while the model loads.
7. In **Local corrections**, confirmed edits can be remembered with **Add?**.
   Use the menu bar's **Correct Last Dictation** if an editor does not support
   automatic detection. **Export dictionary / Import dictionary** moves only
   confirmed phrase pairs between computers; it does not transfer models.

Optional: enable launch at login, start hidden, and unload the model after 5 minutes
in Handy's settings. A fresh installation has its own preferences and empty dictionary;
this package does not copy the developer's settings, recordings or remembered words.

Speech recognition and dictionary corrections are local. **Text polishing** is
optional and off by default: save your own DeepSeek API key in settings, then
explicitly enable it to send transcription text (never audio) to DeepSeek.
The saved-key card shows a masked key and its final four characters. **Show key**
reveals it on request; **Remove key** deletes it and turns polishing off. Only one
DeepSeek key is stored, and saving another replaces it.
Errors fall back to the original. **Restore original** in the latest result or
History copies the original for you to paste. Explicit model downloads use the
network, and the app you dictate into may itself send text online. For local-only
backups, save exported dictionaries outside cloud-synced folders.

Updates use the same app data, but this ad-hoc preview may require re-granting
permissions after an update. If an enabled Accessibility entry is stale, remove
and re-add Handy Local in System Settings, then restart. If a native Save dialog
stays disabled after an update, quit/reopen Handy Local before retrying.
