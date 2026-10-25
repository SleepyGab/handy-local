# Pinned transcribe.cpp sources

These are the crates.io 0.2.4 source distributions, including their MIT and
vendored ggml licenses. Only the files listed below differ. Cargo patches both
crates to these local paths; builds do not patch the user's Cargo cache or fetch
an unpinned branch. The native C++ source is included so the application builds
on a clean machine, without Ollama or LM Studio.

Original package SHA-256 checksums (from Cargo.lock before the patch):

- transcribe-cpp: `8afa3f82415ece32f944e0d5ad347716c7e86d41ee8727ad8143afb080a6a60c`
- transcribe-cpp-sys: `1c6946c7bf90046fc51e5bc25373a4f6270ca10fdcf4f35b1174075e43c535b5`

## Local patch

Adds a private Q3RN run extension containing borrowed UTF-8 context, an owning
Rust CString wrapper, and per-invocation system-context tokenization for
Qwen3-ASR (single and batched runs). Empty context preserves the upstream chat
template. Other architectures retain their existing extensions. The generic
ABI is unchanged; the additive FFI struct is maintained separately from the
generated upstream bindings. The extension validates its size/kind before any
cast, bounds bytes and tokens, and rejects chat delimiters. Context is never
stored in shared session/model state. It is not a cloud request or a model edit.

The application additionally runs an unprompted pass, honors explicit spelling
cues, and falls back to that complete result if the hinted pass fails. See
`src-tauri/src/vocabulary.rs` and the local audio evaluation report.

When updating upstream, replace BOTH crates from matching pinned distributions,
port these changes (or use an equivalent upstream context API), and rerun the
paired audio evaluation plus the native ABI/lifetime checks. Do not attach a
Whisper extension to Qwen3-ASR.

Changed/new source files:

- `transcribe-cpp/src/family.rs`
- `transcribe-cpp/src/lib.rs`
- `transcribe-cpp-sys/bindings/rust/sys/src/lib.rs`
- `transcribe-cpp-sys/bindings/rust/sys/src/qwen3_asr.rs`
- `transcribe-cpp-sys/include/transcribe/extensions.h`
- `transcribe-cpp-sys/include/transcribe/qwen3_asr.h`
- `transcribe-cpp-sys/src/arch/qwen3_asr/model.cpp`
