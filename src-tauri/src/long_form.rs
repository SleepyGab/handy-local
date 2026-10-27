//! Pause-based live and batch decoding for non-streaming ASR models.
//! Short-window models can lose content even when their encoder accepts longer
//! buffers. Keep each decode bounded, retaining context across cuts, and only
//! remove duplicate text at boundaries that actually overlap in the audio.

use std::ops::Range;

use anyhow::{Context, Result};
use transcribe_cpp::{Error, RunOptions, Session};
use transcribe_rs::onnx::cohere::{CohereModel, CohereParams};

use crate::audio_toolkit::{chunking, EarshotVad, VoiceActivityDetector};

const RATE: usize = 16_000;
const WINDOW: usize = chunking::MAX_CHUNK_SAMPLES;
const OVERLAP: usize = 3 * RATE;
const MIN_RETRY: usize = 3 * RATE;

pub fn transcribe_cpp(
    session: &mut Session,
    audio: &[f32],
    options: &RunOptions,
    words: &[String],
) -> Result<String> {
    transcribe(audio, &mut |samples| {
        decode_cpp(session, samples, options, words)
    })
}

fn decode_cpp(
    session: &mut Session,
    samples: &[f32],
    options: &RunOptions,
    words: &[String],
) -> std::result::Result<String, DecodeFailure> {
    match crate::vocabulary::transcribe(session, samples, options, words) {
        Ok(result) if !session.was_truncated() => Ok(result.text),
        Ok(_) => Err(DecodeFailure::Retry(anyhow::anyhow!(
            "ASR output truncated"
        ))),
        Err(error @ (Error::OutputTruncated { .. } | Error::InputTooLong(_))) => {
            Err(DecodeFailure::Retry(error.into()))
        }
        Err(error) => Err(DecodeFailure::Fatal(error.into())),
    }
}

pub fn transcribe_onnx(
    model: &mut CohereModel,
    audio: &[f32],
    language: Option<String>,
) -> Result<String> {
    // Nine control tokens leave 1015 generation slots in the 1024-token
    // decoder. The ONNX API does not expose a truncation flag; short windows
    // and the full available budget avoid its default 512-token ceiling.
    let params = CohereParams {
        language,
        max_new_tokens: Some(1015),
        ..Default::default()
    };
    transcribe(audio, &mut |samples| {
        model
            .transcribe_with(samples, &params)
            .map(|result| result.text)
            .map_err(|error| DecodeFailure::Fatal(error.into()))
    })
}

enum DecodeFailure {
    Retry(anyhow::Error),
    Fatal(anyhow::Error),
}

fn transcribe(
    audio: &[f32],
    decode: &mut impl FnMut(&[f32]) -> std::result::Result<String, DecodeFailure>,
) -> Result<String> {
    let mut chunks = Vec::new();
    for range in plan_chunks(audio) {
        decode_range(audio, range, decode, &mut chunks)?;
    }

    let mut joined = TranscriptJoiner::default();
    for chunk in chunks {
        joined.append(chunk);
    }
    Ok(joined.text)
}

struct DecodedChunk {
    range: Range<usize>,
    text: String,
}

#[derive(Default)]
struct TranscriptJoiner {
    text: String,
    previous_end: usize,
    previous_has_text: bool,
}

impl TranscriptJoiner {
    fn append(&mut self, chunk: DecodedChunk) {
        let overlap = if self.previous_has_text {
            self.previous_end.saturating_sub(chunk.range.start)
        } else {
            0
        };
        append_chunk(&mut self.text, &chunk.text, overlap);
        self.previous_end = chunk.range.end;
        self.previous_has_text = !chunk.text.trim().is_empty();
    }
}

/// Bounded audio pending a reliable pause cut. Reuses the batch planner and
/// boundary merger; only the undecoded tail is kept after a cut is committed.
#[derive(Default)]
pub struct LiveDecoder {
    pending: Vec<f32>,
    offset: usize,
    pub samples: usize,
    joined: TranscriptJoiner,
}

impl LiveDecoder {
    pub fn text(&self) -> &str {
        &self.joined.text
    }

    /// Boundary context can still be revised by the following chunk. Keep
    /// that suffix tentative in the overlay, so committed text stays stable.
    pub fn preview(&self) -> (&str, &str) {
        let units = text_units(self.text());
        let held = OVERLAP * 12 / RATE;
        let split = units
            .get(units.len().saturating_sub(held))
            .map_or(0, |unit| unit.start);
        self.text().split_at(split)
    }

    pub fn feed(
        &mut self,
        audio: &[f32],
        session: &mut Session,
        options: &RunOptions,
        words: &[String],
    ) -> Result<bool> {
        self.push_with(audio, &mut |samples| {
            decode_cpp(session, samples, options, words)
        })
    }

    fn push_with(
        &mut self,
        audio: &[f32],
        decode: &mut impl FnMut(&[f32]) -> std::result::Result<String, DecodeFailure>,
    ) -> Result<bool> {
        self.samples += audio.len();
        self.pending.extend_from_slice(audio);
        let mut changed = false;
        // Five seconds of lookahead protects the upstream pause planner from
        // creating a short remainder. Additional context covers the overlap.
        while self.pending.len() > WINDOW + 5 * RATE + OVERLAP {
            let Some(range) = plan_chunks(&self.pending).into_iter().next() else {
                break;
            };
            let discard = range.end.saturating_sub(OVERLAP);
            anyhow::ensure!(discard > 0, "long-form chunk made no progress");
            self.decode_range(range, decode)?;
            self.pending.drain(..discard);
            self.offset += discard;
            changed = true;
        }
        Ok(changed)
    }

    pub fn finish(
        &mut self,
        session: &mut Session,
        options: &RunOptions,
        words: &[String],
    ) -> Result<()> {
        self.finish_with(&mut |samples| decode_cpp(session, samples, options, words))
    }

    fn finish_with(
        &mut self,
        decode: &mut impl FnMut(&[f32]) -> std::result::Result<String, DecodeFailure>,
    ) -> Result<()> {
        for range in plan_chunks(&self.pending) {
            self.decode_range(range, decode)?;
        }
        self.pending.clear();
        Ok(())
    }

    fn decode_range(
        &mut self,
        range: Range<usize>,
        decode: &mut impl FnMut(&[f32]) -> std::result::Result<String, DecodeFailure>,
    ) -> Result<()> {
        let mut chunks = Vec::new();
        decode_range(&self.pending, range, decode, &mut chunks)?;
        for mut chunk in chunks {
            chunk.range = chunk.range.start + self.offset..chunk.range.end + self.offset;
            self.joined.append(chunk);
        }
        Ok(())
    }
}

fn decode_range(
    audio: &[f32],
    range: Range<usize>,
    decode: &mut impl FnMut(&[f32]) -> std::result::Result<String, DecodeFailure>,
    chunks: &mut Vec<DecodedChunk>,
) -> Result<()> {
    match decode(&audio[range.clone()]) {
        Ok(text) => {
            log::debug!(
                "ASR chunk {:.2}-{:.2}s decoded",
                range.start as f64 / RATE as f64,
                range.end as f64 / RATE as f64
            );
            chunks.push(DecodedChunk { range, text });
            Ok(())
        }
        Err(DecodeFailure::Retry(error)) if range.len() > MIN_RETRY => {
            log::warn!("ASR chunk incomplete; retrying shorter audio: {error}");
            // Discard the incomplete hypothesis and decode both halves. The
            // limited overlap preserves boundary words without stalling the
            // recursion, even on a model that repeatedly exhausts its budget.
            let middle = range.start + range.len() / 2;
            let overlap = RATE.min(range.len() / 8);
            decode_range(audio, range.start..middle + overlap, decode, chunks)?;
            decode_range(audio, middle..range.end, decode, chunks)
        }
        Err(DecodeFailure::Retry(error) | DecodeFailure::Fatal(error)) => {
            Err(error).with_context(|| {
                format!(
                    "ASR transcription failed at {:.2}-{:.2}s",
                    range.start as f64 / RATE as f64,
                    range.end as f64 / RATE as f64
                )
            })
        }
    }
}

pub(crate) fn plan_chunks(audio: &[f32]) -> Vec<Range<usize>> {
    if audio.len() <= WINDOW {
        return if audio.is_empty() {
            Vec::new()
        } else {
            std::iter::once(0..audio.len()).collect()
        };
    }
    // Use Handy's pause-ranking planner without importing its newer engine
    // supervisor or transcribe-cpp 0.4 dependency. Earshot is bundled, needs
    // no model download, and only chooses cuts; it never discards audio.
    let scores = (|| -> Result<_> {
        let mut vad = EarshotVad::new(0.5)?;
        let frame = vad.frame_samples();
        let mut scores = Vec::with_capacity(audio.len().div_ceil(frame));
        for samples in audio.chunks(frame) {
            if samples.len() == frame {
                scores.push(vad.speech_probability(samples)?);
            } else {
                let mut padded = vec![0.0; frame];
                padded[..samples.len()].copy_from_slice(samples);
                scores.push(vad.speech_probability(&padded)?);
            }
        }
        Ok((scores, frame))
    })();
    let (probabilities, frame) = scores.unwrap_or_else(|error| {
        log::warn!("ASR pause detection failed; using bounded hard cuts: {error}");
        (Vec::new(), 1)
    });
    chunking::plan(audio.len(), &probabilities, frame)
        .into_iter()
        .map(|range| {
            range.start.saturating_sub(OVERLAP / 2)..(range.end + OVERLAP / 2).min(audio.len())
        })
        .collect()
}

fn unspaced(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x9fff | 0x20000..=0x323af | 0x3040..=0x30ff | 0xac00..=0xd7af)
}

struct TextUnit {
    normalized: String,
    start: usize,
    unspaced: bool,
}

// Whole words for spaced scripts, single characters for CJK. Comparing only
// complete units avoids removing "can" from "candy", and byte offsets keep
// the returned slice valid for UTF-8. Punctuation/case may change across runs.
fn text_units(text: &str) -> Vec<TextUnit> {
    let mut units: Vec<TextUnit> = Vec::new();
    let mut in_word = false;
    for (start, c) in text.char_indices() {
        if !c.is_alphanumeric() {
            in_word = false;
        } else if unspaced(c) {
            units.push(TextUnit {
                normalized: c.to_string(),
                start,
                unspaced: true,
            });
            in_word = false;
        } else if in_word {
            if let Some(unit) = units.last_mut() {
                unit.normalized.extend(c.to_lowercase());
            }
        } else {
            units.push(TextUnit {
                normalized: c.to_lowercase().collect(),
                start,
                unspaced: false,
            });
            in_word = true;
        }
    }
    units
}

struct OverlapMatch {
    left_start: usize,
    #[cfg(test)]
    right_end: usize,
}

fn overlap_match(previous: &str, next: &str, overlap_samples: usize) -> Option<OverlapMatch> {
    if overlap_samples == 0 {
        return None;
    }
    let left = text_units(previous);
    let right = text_units(next);
    // Limit matching to the shared audio. Never remove a phrase merely
    // because it occurred earlier in the recording.
    let max_units = (overlap_samples * 12 / RATE).max(1);
    let mut best = None;
    let mut best_score = 0;
    for left_count in 1..=left.len().min(max_units) {
        let suffix = &left[left.len() - left_count..];
        let cjk = suffix.iter().filter(|unit| unit.unspaced).count();
        if (cjk == 0 && left_count < 2) || (cjk > 0 && left_count < 4) {
            continue;
        }
        for right_count in
            left_count.saturating_sub(2).max(1)..=right.len().min(max_units).min(left_count + 2)
        {
            let minimum = left_count.min(right_count);
            let budget = (minimum / if cjk > 0 { 6 } else { 5 }).min(2);
            let distance = edit_distance(suffix, &right[..right_count]);
            if distance > budget {
                continue;
            }
            let score = minimum.saturating_sub(2 * distance);
            if score >= best_score {
                best_score = score;
                best = Some(OverlapMatch {
                    left_start: suffix[0].start,
                    #[cfg(test)]
                    right_end: right.get(right_count).map_or(next.len(), |unit| unit.start),
                });
            }
        }
    }
    best
}

fn edit_distance(left: &[TextUnit], right: &[TextUnit]) -> usize {
    let mut row: Vec<usize> = (0..=right.len()).collect();
    for (i, a) in left.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, b) in right.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (diagonal + usize::from(a.normalized != b.normalized))
                .min(row[j] + 1)
                .min(above + 1);
            diagonal = above;
        }
    }
    row[right.len()]
}

fn append_chunk(joined: &mut String, next: &str, overlap_samples: usize) {
    let next = next.trim();
    if let Some(overlap) = overlap_match(joined, next, overlap_samples) {
        // The next chunk has more right-hand context at this boundary; keep
        // its spelling rather than preserving the previous chunk's cut word.
        joined.truncate(overlap.left_start);
        joined.truncate(joined.trim_end().len());
    }
    append_text(joined, next);
}

#[cfg(test)]
fn trim_overlap<'a>(previous: &str, next: &'a str, overlap_samples: usize) -> &'a str {
    let next = next.trim();
    overlap_match(previous, next, overlap_samples)
        .map_or(next, |matched| &next[matched.right_end..])
}

fn append_text(joined: &mut String, next: &str) {
    let next = next.trim();
    let Some(first) = next.chars().next() else {
        return;
    };
    // Ignore sentence punctuation when deciding whether this is a CJK join.
    if let Some(last) = joined.chars().rev().find(|c| c.is_alphanumeric()) {
        if !unspaced(last) && !unspaced(first) {
            joined.push(' ');
        }
    }
    joined.push_str(next);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_decode_does_work_before_stop_and_covers_the_final_tail() {
        let audio: Vec<f32> = (0..75 * RATE).map(|i| i as f32).collect();
        let mut covered = vec![false; audio.len()];
        let mut calls = 0;
        let mut decode = |chunk: &[f32]| {
            calls += 1;
            assert!(chunk.len() <= WINDOW + OVERLAP);
            let start = chunk[0] as usize;
            covered[start..start + chunk.len()].fill(true);
            Ok(format!("part{start}"))
        };
        let mut live = LiveDecoder::default();
        let mut updates = 0;
        for chunk in audio.chunks(RATE) {
            updates += usize::from(live.push_with(chunk, &mut decode).unwrap());
            assert!(live.pending.len() <= WINDOW + 5 * RATE + OVERLAP);
        }
        assert!(updates > 0, "completed chunks should appear before stop");
        assert!(live.text().starts_with("part0"));
        live.finish_with(&mut decode).unwrap();
        assert_eq!(live.samples, audio.len());
        assert!(calls > updates);
        assert!(covered.iter().all(|seen| *seen));
    }

    #[test]
    fn short_live_clips_decode_once_at_stop() {
        let mut live = LiveDecoder::default();
        let mut calls = 0;
        let mut decode = |_: &[f32]| {
            calls += 1;
            Ok("short clip".to_string())
        };
        assert!(!live.push_with(&vec![0.0; 5 * RATE], &mut decode).unwrap());
        assert!(live.text().is_empty());
        live.finish_with(&mut decode).unwrap();
        assert_eq!(calls, 1);
        assert_eq!(live.text(), "short clip");
    }

    #[test]
    fn long_audio_is_bounded_and_every_sample_is_covered() {
        for seconds in [0, 5, 25, 30, 35, 60, 97, 180] {
            // Sample values encode their position for the fake decoder.
            let audio: Vec<f32> = (0..seconds * RATE).map(|i| i as f32).collect();
            let mut coverage = vec![false; audio.len()];
            let text = transcribe(&audio, &mut |chunk| {
                assert!(chunk.len() <= WINDOW + OVERLAP);
                let start = chunk[0] as usize;
                coverage[start..start + chunk.len()].fill(true);
                Ok(format!("part{start}"))
            })
            .unwrap();
            assert!(coverage.iter().all(|covered| *covered));
            assert_eq!(text.is_empty(), seconds == 0);
            if seconds > 25 {
                assert!(text.split_whitespace().count() > 1);
            }
        }
    }

    #[test]
    fn merges_cjk_and_english_overlap_ignoring_case_and_punctuation() {
        let cases = [
            (
                "这是前半句，接下来继续测试。",
                "接下来继续测试，最后一句。",
                "最后一句。",
            ),
            (
                "We will test the final sentence.",
                "The final sentence, and the ending.",
                "and the ending.",
            ),
            (
                "接下来测试 Cohere model。",
                "测试 Cohere model，最后一句。",
                "最后一句。",
            ),
            (
                "日本語の長い音声です。",
                "長い音声です。最後です。",
                "最後です。",
            ),
        ];
        for (previous, next, expected) in cases {
            assert_eq!(trim_overlap(previous, next, OVERLAP), expected);
        }
    }

    #[test]
    fn preserves_nonoverlapping_text_partial_words_and_short_repetitions() {
        assert_eq!(
            trim_overlap("we can", "candy is good", OVERLAP),
            "candy is good"
        );
        assert_eq!(
            trim_overlap("no", "no, please stop", OVERLAP),
            "no, please stop"
        );
        assert_eq!(trim_overlap("好的", "好的，继续", OVERLAP), "好的，继续");
        assert_eq!(
            trim_overlap("repeat this", "repeat this again", 0),
            "repeat this again"
        );
        assert_eq!(trim_overlap("", "  final words  ", OVERLAP), "final words");
        assert_eq!(trim_overlap("last sentence", "last sentence", OVERLAP), "");
        let mut joined = "这是中文。".to_string();
        append_text(&mut joined, "下一句。");
        assert_eq!(joined, "这是中文。下一句。");
        let mut joined = "First sentence.".to_string();
        append_text(&mut joined, "Last sentence.");
        assert_eq!(joined, "First sentence. Last sentence.");
    }

    #[test]
    fn boundary_splice_repairs_a_cut_word_and_removes_near_duplicates() {
        let mut text = "上一句。第三部分下午准备整理书检".to_string();
        append_chunk(&mut text, "第三部分下午准备整理书架把历史书放好", OVERLAP);
        assert_eq!(text, "上一句。第三部分下午准备整理书架把历史书放好");
        let mut text = "中间的关键词是绿色火车第五部分明天我们打算去过".to_string();
        append_chunk(&mut text, "车第五部分明天我们打算去公园散步", OVERLAP);
        assert_eq!(text, "中间的关键词是绿色火车第五部分明天我们打算去公园散步");
        let mut text = "We have decided to visit the library today.".to_string();
        append_chunk(&mut text, "to visit a library today, after lunch.", OVERLAP);
        assert_eq!(
            text,
            "We have decided to visit a library today, after lunch."
        );
    }

    #[test]
    fn fuzzy_matching_does_not_remove_unrelated_sentences() {
        let mut text = "我们今天准备去公园散步。".to_string();
        append_chunk(&mut text, "他们明天计划到市场买菜。", OVERLAP);
        assert_eq!(text, "我们今天准备去公园散步。他们明天计划到市场买菜。");
    }

    #[test]
    fn truncation_retries_both_halves_and_discards_partial_output() {
        let audio: Vec<f32> = (0..20 * RATE).map(|i| i as f32).collect();
        let mut coverage = vec![false; audio.len()];
        let mut calls = 0;
        let text = transcribe(&audio, &mut |chunk| {
            calls += 1;
            if chunk.len() > 6 * RATE {
                return Err(DecodeFailure::Retry(anyhow::anyhow!("budget exhausted")));
            }
            let start = chunk[0] as usize;
            coverage[start..start + chunk.len()].fill(true);
            Ok(format!("part{start}"))
        })
        .unwrap();
        assert!(calls > 3);
        assert!(coverage.iter().all(|covered| *covered));
        assert!(text.starts_with("part0"));
    }

    #[test]
    fn permanent_truncation_and_late_failures_never_return_partial_success() {
        let audio = vec![0.0; 60 * RATE];
        let mut calls = 0;
        let result = transcribe(&audio, &mut |_| {
            calls += 1;
            if calls == 1 {
                Ok("first chunk".into())
            } else {
                Err(DecodeFailure::Fatal(anyhow::anyhow!("cancelled")))
            }
        });
        assert!(result.is_err());
        assert_eq!(calls, 2);
        let mut calls = 0;
        let result = transcribe(&audio, &mut |_| {
            calls += 1;
            Err(DecodeFailure::Retry(anyhow::anyhow!("truncated")))
        });
        assert!(result.is_err());
        assert!(calls < 10, "retry depth must be bounded");
    }
}
