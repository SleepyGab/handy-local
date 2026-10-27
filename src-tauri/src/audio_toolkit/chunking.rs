//! Pause-based cut planning adapted from Handy's long-form chunking.
//! Source: cjpais/Handy, commit 73c095857bc4eadff097f64767c40ad129cc372d,
//! src-tauri/src/audio_toolkit/chunking.rs (MIT).
//! Keep the upstream speech-segment and pause-ranking algorithm; Cohere uses
//! a 25 s maximum and 20 s target to leave room for boundary context.

use std::ops::Range;

const RATE: usize = 16_000;
const TARGET_SECONDS: f64 = 20.0;
pub const MAX_CHUNK_SAMPLES: usize = 25 * RATE;
const MIN_CHUNK_SAMPLES: usize = 5 * RATE;
const THRESHOLD: f32 = 0.5;
const NEG_THRESHOLD: f32 = THRESHOLD - 0.15;
const MIN_SPEECH_SAMPLES: usize = 250 * RATE / 1000;
const MIN_SILENCE_SAMPLES: usize = 100 * RATE / 1000;
const SPEECH_PAD_SAMPLES: usize = 30 * RATE / 1000;

/// Plan contiguous, complete coverage. Missing scores mean no known pauses;
/// the upstream hard-cut fallback still bounds the input to the ASR model.
pub fn plan(samples: usize, probabilities: &[f32], frame_samples: usize) -> Vec<Range<usize>> {
    let segments = speech_segments(probabilities, frame_samples, samples);
    let candidates = if probabilities.is_empty() {
        Vec::new()
    } else {
        cut_candidates(&segments, samples)
    };
    let mut cuts = plan_cuts(&candidates, samples, 0);
    cuts.push(samples);
    let mut start = 0;
    cuts.into_iter()
        .filter_map(|end| {
            let range = start..end;
            start = end;
            (!range.is_empty()).then_some(range)
        })
        .collect()
}

/// Speech segments, in samples, from per-frame speech probabilities: Silero's
/// `get_speech_timestamps` without a maximum speech length. `samples` is the
/// length of the audio the probabilities score.
fn speech_segments(probs: &[f32], frame_samples: usize, samples: usize) -> Vec<Range<usize>> {
    let mut segments: Vec<Range<usize>> = Vec::new();
    let mut triggered = false;
    let mut start = 0;
    let mut temp_end: Option<usize> = None;
    for (i, &prob) in probs.iter().enumerate() {
        let at = i * frame_samples;
        if prob >= THRESHOLD {
            temp_end = None;
            if !triggered {
                triggered = true;
                start = at;
            }
            continue;
        }
        if prob < NEG_THRESHOLD && triggered {
            let end = *temp_end.get_or_insert(at);
            if at - end < MIN_SILENCE_SAMPLES {
                continue;
            }
            if end - start > MIN_SPEECH_SAMPLES {
                segments.push(start..end);
            }
            triggered = false;
            temp_end = None;
        }
    }
    if triggered && samples.saturating_sub(start) > MIN_SPEECH_SAMPLES {
        segments.push(start..samples);
    }

    for i in 0..segments.len() {
        if i == 0 {
            segments[0].start = segments[0].start.saturating_sub(SPEECH_PAD_SAMPLES);
        }
        if i + 1 == segments.len() {
            segments[i].end = (segments[i].end + SPEECH_PAD_SAMPLES).min(samples);
            continue;
        }
        let silence = segments[i + 1].start - segments[i].end;
        let pad = if silence < 2 * SPEECH_PAD_SAMPLES {
            silence / 2
        } else {
            SPEECH_PAD_SAMPLES
        };
        segments[i].end = (segments[i].end + pad).min(samples);
        segments[i + 1].start = segments[i + 1].start.saturating_sub(pad);
    }
    segments
}

/// A place a chunk may end: the middle of a pause.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CutCandidate {
    cut: usize,
    gap_seconds: f64,
}

/// One candidate in the middle of each pause, lead-in and tail included.
fn cut_candidates(segments: &[Range<usize>], samples: usize) -> Vec<CutCandidate> {
    let mut candidates = Vec::new();
    let mut silence_start = 0;
    for segment in segments
        .iter()
        .cloned()
        .chain(std::iter::once(samples..samples))
    {
        if segment.start > silence_start {
            candidates.push(CutCandidate {
                cut: (silence_start + segment.start) / 2,
                gap_seconds: (segment.start - silence_start) as f64 / RATE as f64,
            });
        }
        silence_start = silence_start.max(segment.end);
    }
    candidates
}

/// Greedy, left to right from `from`: each cut is the best-scoring pause
/// 5-25 s after the previous one, preferring long pauses near 20 s and ones
/// that leave at least 5 s after them. With no pause, it is a hard cut at 25 s,
/// or halfway when that would leave under 5 s. Audio of 25 s or less is never
/// cut. `candidates` must be in order.
fn plan_cuts(candidates: &[CutCandidate], samples: usize, from: usize) -> Vec<usize> {
    let mut cuts = Vec::new();
    let mut start = from;
    // Each window starts after the last, so earlier candidates never return.
    let mut first = 0;
    while samples - start > MAX_CHUNK_SAMPLES {
        while first < candidates.len() && candidates[first].cut < start + MIN_CHUNK_SAMPLES {
            first += 1;
        }
        // A pause that leaves under 5 s is only used when no other pause will do.
        let mut best: Option<(usize, f64)> = None;
        let mut fallback: Option<(usize, f64)> = None;
        for candidate in candidates[first..]
            .iter()
            .take_while(|candidate| candidate.cut <= start + MAX_CHUNK_SAMPLES)
        {
            let position = (candidate.cut - start) as f64 / RATE as f64;
            let off_target = if position > TARGET_SECONDS {
                0.1 * (position - TARGET_SECONDS)
            } else {
                0.05 * (TARGET_SECONDS - position)
            };
            let score = candidate.gap_seconds.min(1.0) - off_target;
            if samples - candidate.cut >= MIN_CHUNK_SAMPLES
                && best.is_none_or(|(_, best)| score > best)
            {
                best = Some((candidate.cut, score));
            }
            if fallback.is_none_or(|(_, fallback)| score > fallback) {
                fallback = Some((candidate.cut, score));
            }
        }
        let hard_cut = if samples - start < MAX_CHUNK_SAMPLES + MIN_CHUNK_SAMPLES {
            start + (samples - start) / 2
        } else {
            start + MAX_CHUNK_SAMPLES
        };
        start = best.or(fallback).map_or(hard_cut, |(cut, _)| cut);
        cuts.push(start);
    }
    cuts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_pauses(seconds: usize, pauses: &[(f64, f64)]) -> Vec<Range<usize>> {
        let frame = 160;
        let probs: Vec<f32> = (0..seconds * RATE / frame)
            .map(|i| {
                let time = (i * frame) as f64 / RATE as f64;
                if pauses.iter().any(|&(from, to)| time >= from && time < to) {
                    0.0
                } else {
                    1.0
                }
            })
            .collect();
        plan(seconds * RATE, &probs, frame)
    }

    #[test]
    fn upstream_pause_ranking_prefers_long_pauses_near_target() {
        let chunks = plan_pauses(60, &[(18.0, 19.0), (20.0, 20.2), (39.0, 40.0)]);
        assert!((chunks[0].end as f64 / RATE as f64 - 18.5).abs() < 0.1);
        assert!((chunks[1].end as f64 / RATE as f64 - 39.5).abs() < 0.1);
        assert_eq!(chunks.last().unwrap().end, 60 * RATE);
    }

    #[test]
    fn hard_cuts_and_silence_keep_complete_bounded_coverage() {
        for seconds in [0, 5, 25, 26, 30, 60, 97, 180] {
            for chunks in [
                plan(seconds * RATE, &[], 160),
                plan_pauses(seconds, &[]),
                plan(seconds * RATE, &vec![0.0; seconds * 100], 160),
            ] {
                let mut end = 0;
                for chunk in chunks {
                    assert_eq!(chunk.start, end);
                    assert!(chunk.len() <= MAX_CHUNK_SAMPLES);
                    end = chunk.end;
                }
                assert_eq!(end, seconds * RATE);
            }
        }
    }

    #[test]
    fn upstream_fallback_avoids_a_tiny_last_chunk() {
        assert_eq!(
            plan(27 * RATE, &[], 160),
            vec![0..27 * RATE / 2, 27 * RATE / 2..27 * RATE]
        );
    }
}
