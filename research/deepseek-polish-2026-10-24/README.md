# DeepSeek application-prompt verification — 2026-10-24

Historical run using the installed application's exact polishing prompt and
request parameters against `deepseek-flash` (thinking disabled, temperature 0,
nonstreaming, max output 8192 tokens). Only synthetic transcription text was
sent: 10 Chinese and 10 Chinese/English mixed cases. No audio, personal history
or credentials are included here.

All 20 requests completed. Median end-to-end request time was 1.216 seconds
(range 0.902–2.093); Chinese median was 1.181 and mixed median was 1.270 seconds.
Returned usage totaled 9,390 input tokens and 595 output tokens. The estimated
cost at the off-peak prices recorded on the test date was CNY 0.00650348, based
on 5,374 cached input tokens, 4,016 uncached input tokens and 595 output tokens.
That is a usage-based estimate, not a settled bill or current price promise.

`results.jsonl` contains all synthetic inputs, outputs, timings and usage.
`review.json` records the assistant's qualitative review: no major meaning
reversal was observed in these 20 examples, but withdrawn wording and some
fillers remain. This was not a blind human evaluation and is not a general
accuracy estimate. Network conditions, provider revisions and input length can
change results. The tests in this PR do not call the live service or incur costs.

The prompt is `src-tauri/resources/prompts/deepseek-polish.txt`; its SHA-256 at
evaluation time was
`1d3391ded16c3aa8acd315aa35dc4979ad03bd044785a7a6bf42a1544b445044`.
The Rust `cloud_polish` tests use a local HTTP fixture for fallback, truncation,
empty responses and timeouts. The explicitly ignored live smoke test requires
a test credential supplied by the operator; it never reads a bundled key.
