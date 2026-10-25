# Local vocabulary evaluation — 2026-10-24

This records 40 synthetic speech cases, decoded with and without vocabulary
using Qwen3-ASR-1.7B-Q5_K_M on Metal (Apple M5 Pro, 64 GiB). macOS Tingting,
Meijia and Samantha voices produced the audio locally. No microphone audio,
personal history, API credentials or cloud requests were used.

| Group                           | Cases | Target terms correct, off | Target terms correct, on |
| ------------------------------- | ----: | ------------------------: | -----------------------: |
| Chinese company names           |    10 |                         1 |                        9 |
| Chinese/English mixed           |     9 |                         1 |                        8 |
| Ordinary words                  |    13 |                        13 |                       13 |
| Spelling/homophone stress cases |     8 |                         7 |                        8 |

These scores count exact, case-sensitive target-term inclusion, **not whole
transcript accuracy**. All 20 negative cases retained exactly the baseline
output. The initial prototype overrode explicit spellings; the final two-pass
guard preserves those baseline outputs. Known remaining failures are documented
in `summary.json`; in particular, correcting a name does not certify the meaning
of the rest of the sentence. This small synthetic set does not establish general
accuracy on human speech, accents, microphones or noise.

Median warm decode was 0.168 seconds without vocabulary and 0.328 seconds with
vocabulary; the latter includes the protective baseline pass. Peak process RSS
was about 2.47 GiB. Cached model load was 0.457 seconds; first loads may be much
slower. The paired raw outputs and timings are in `final-results.jsonl`.

## Reproduce

Install the three macOS voices if needed, download the named Qwen3-ASR GGUF,
and set `MODEL` to its local path. From the repository root:

```sh
python3 research/local-vocabulary-2026-10-24/prepare_audio.py
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run --release --locked \
  --manifest-path src-tauri/Cargo.toml --features local-only \
  --example vocabulary_eval -- "$MODEL" \
  research/local-vocabulary-2026-10-24/final-cases.json \
  research/local-vocabulary-2026-10-24/rerun-results.jsonl
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run --release --locked \
  --manifest-path src-tauri/Cargo.toml --features local-only \
  --example vocabulary_native_checks -- "$MODEL" \
  research/local-vocabulary-2026-10-24/audio/zh01.wav
```

Voice revisions may change synthesized audio and outputs. The native check also
tests empty/deleted context, invalid context bounds, NUL handling, and batch
parity/context lifetime. The recorded results predate PR preparation and were
collected with the same transcription implementation in the installed app's
source tree. No model weights or generated audio are included in this repository.
