# Attribution

nibi (formerly Handy Local) is an independent fork of Handy v0.9.8, upstream commit
`14f6f0d31cb22a4268acfbddba3052820fd0a06e`.

- Original project: https://github.com/cjpais/Handy
- Copyright (c) 2025 CJ Pais; MIT license retained in LICENSE.
- nibi / Handy Local additions: Copyright (c) 2026 SleepyGab and contributors, MIT.
- The Handy name and artwork identify the inherited project; this fork does not
  imply endorsement or claim ownership of upstream branding.
- Native/frontend dependencies retain their respective licenses; exact dependency
  versions are recorded in Cargo.lock and bun.lock. The included VAD resource is
  inherited from Handy; speech recognition model weights are not in the installer.
- Separately downloaded model weights have their own license terms, available from
  the model's source repository. This fork's MIT license does not replace them.

The initial Handy Local v0.9.8-local.1 binary preview predates the nibi rebrand,
custom vocabulary and DeepSeek text polishing. It contains the unchanged application
built from commit `dddc28d` with `local-only`; the preview tag adds packaging and
documentation only.
