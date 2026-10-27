//! Local ASR hints. Shared by dictation and the audio evaluation harness.
use pinyin::ToPinyin;
use transcribe_cpp::{
    Qwen3AsrRunOptions, RunExtension, RunOptions, Session, Transcript, WhisperRunOptions,
};

pub const MAX_WORDS: usize = 32;
pub const MAX_WORD_CHARS: usize = 50;
pub const MAX_TOTAL_CHARS: usize = 400;

pub fn normalize_word(word: &str) -> Result<String, &'static str> {
    if word
        .chars()
        .any(|c| c.is_control() || matches!(c, '<' | '>'))
    {
        return Err("invalidWord");
    }
    let word = word.split_whitespace().collect::<Vec<_>>().join(" ");
    if word.is_empty()
        || word.chars().count() > MAX_WORD_CHARS
        || !word.chars().any(char::is_alphanumeric)
    {
        return Err("invalidWord");
    }
    Ok(word)
}

pub fn validate_words(words: Vec<String>) -> Result<Vec<String>, &'static str> {
    let mut normalized = Vec::new();
    for word in words {
        let word = normalize_word(&word)?;
        if normalized
            .iter()
            .any(|w: &String| w.to_lowercase() == word.to_lowercase())
        {
            return Err("duplicate");
        }
        normalized.push(word);
    }
    if normalized.len() > MAX_WORDS
        || normalized.iter().map(|w| w.chars().count()).sum::<usize>() > MAX_TOTAL_CHARS
    {
        return Err("wordLimit");
    }
    Ok(normalized)
}

/// Bound legacy settings as well as newly validated entries, without changing
/// persisted data. A malformed/oversized list must not break dictation.
fn bounded_words(words: &[String]) -> Vec<String> {
    let mut selected = Vec::new();
    let mut chars = 0;
    for word in words {
        let Ok(word) = normalize_word(word) else {
            continue;
        };
        if selected.len() >= MAX_WORDS || chars + word.chars().count() > MAX_TOTAL_CHARS {
            break;
        }
        chars += word.chars().count();
        selected.push(word);
    }
    selected
}

pub fn run_extension(arch: &str, words: &[String]) -> Option<RunExtension> {
    let words = bounded_words(words);
    if words.is_empty() {
        return None;
    }
    match arch {
        "qwen3_asr" => Some(RunExtension::Qwen3Asr(Qwen3AsrRunOptions {
            context: Some(format!("词汇：{}", words.join("、"))),
        })),
        "whisper" => Some(RunExtension::Whisper(WhisperRunOptions {
            initial_prompt: Some(words.join(", ")),
            ..Default::default()
        })),
        _ => None,
    }
}

/// A speaker spelling out a name is stronger evidence than a dictionary hint.
/// Normalize quote/space punctuation emitted by ASR before checking cues.
pub fn has_spelling_cue(text: &str) -> bool {
    static CHINESE_SPELLING: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| {
            regex::Regex::new(r"([\p{Han}]{1,8})的([\p{Han}])(?:[，。！？、；：,.!?;:]|$)")
                .expect("constant spelling regex")
        });
    static ENGLISH_SPELLING: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(
        || {
            regex::Regex::new(r"(?i)\b(?:spell(?:ed|ing)?|letter by letter|[a-z] as in|with (?:an? )?(?:uppercase|lowercase|capital|small))\b").expect("constant English spelling regex")
        },
    );
    let compact: String = text
        .chars()
        .filter(|c| {
            !c.is_whitespace() && !matches!(c, '“' | '”' | '「' | '」' | '‘' | '’' | '\'' | '"')
        })
        .collect();
    CHINESE_SPELLING
        .captures_iter(&compact)
        .any(|parts| parts[1].contains(&parts[2]))
        || ENGLISH_SPELLING.is_match(text)
        || [
            "拼写",
            "写作",
            "写成",
            "不是这个字",
            "不叫",
            "字旁",
            "偏旁",
            "三点水",
        ]
        .iter()
        .any(|cue| text.contains(cue))
}

/// Local fallback for models without vocabulary conditioning. Mandarin terms
/// require identical pronunciation and substantial spelling agreement; ties
/// are left alone. The same spelling-cue protection applies to every model.
pub fn correct_fallback(
    text: &str,
    words: &[String],
    threshold: f64,
    language: Option<&str>,
) -> String {
    if words.is_empty() || has_spelling_cue(text) {
        return text.to_string();
    }
    let english = correct_ascii_spans(text, words, threshold);
    if !language.is_some_and(|language| language == "zh" || language.starts_with("zh-")) {
        return english;
    }
    correct_mandarin_terms(&english, words)
}

fn correct_ascii_spans(text: &str, words: &[String], threshold: f64) -> String {
    let mut result = String::with_capacity(text.len());
    let mut start = None;
    // ASCII terms may be embedded directly in Chinese sentences. Correct
    // each Latin span without making whitespace a requirement at CJK edges.
    let mut spans = Vec::new();
    for (at, c) in text.char_indices() {
        if c.is_ascii() {
            start.get_or_insert(at);
        } else if let Some(from) = start.take() {
            spans.push(from..at);
        }
    }
    if let Some(from) = start {
        spans.push(from..text.len());
    }
    let mut copied = 0;
    for span in spans {
        result.push_str(&text[copied..span.start]);
        let source = &text[span.clone()];
        let trimmed = source.trim();
        if trimmed.is_empty() {
            result.push_str(source);
        } else {
            let from = source.len() - source.trim_start().len();
            result.push_str(&source[..from]);
            result.push_str(&crate::audio_toolkit::apply_custom_words(
                trimmed, words, threshold,
            ));
            result.push_str(&source[source.trim_end().len()..]);
        }
        copied = span.end;
    }
    result.push_str(&text[copied..]);
    result
}

fn han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x9fff | 0x20000..=0x323af)
}

fn pronunciation(chars: &[char]) -> Option<Vec<&'static str>> {
    chars
        .iter()
        .map(|c| c.to_pinyin().map(|p| p.plain()))
        .collect()
}

fn same_mandarin_sounds(candidate: &[&str], expected: &[&str]) -> bool {
    let mut nasal_differences = 0;
    for (actual, target) in candidate.iter().zip(expected) {
        if actual == target {
            continue;
        }
        // Front/back nasal confusion is a common ASR spelling error. Permit
        // one such syllable only in a longer term; all other syllables must
        // match exactly. Do not use generic edit distance on pronunciations.
        let normalize = |sound: &str| {
            sound
                .strip_suffix('g')
                .filter(|_| sound.ends_with("ng"))
                .unwrap_or(sound)
                .to_string()
        };
        if candidate.len() < 3 || normalize(actual) != normalize(target) {
            return false;
        }
        nasal_differences += 1;
    }
    nasal_differences <= 1
}

fn correct_mandarin_terms(text: &str, words: &[String]) -> String {
    let terms: Vec<_> = bounded_words(words)
        .into_iter()
        .filter_map(|word| {
            let chars: Vec<char> = word.chars().collect();
            if chars.len() < 2 || !chars.iter().all(|&c| han(c)) {
                return None;
            }
            let sounds = pronunciation(&chars)?;
            Some((word, chars, sounds))
        })
        .collect();
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        let mut best: Option<(usize, usize, &str)> = None;
        let mut ambiguous = false;
        for (word, spelling, sounds) in &terms {
            let length = spelling.len();
            let Some(candidate) = chars.get(at..at + length) else {
                continue;
            };
            // Preserve an exact dictionary spelling even when another entry
            // shares its pronunciation (e.g. two people with homophone names).
            if candidate == spelling.as_slice() {
                best = Some((length, 0, word));
                ambiguous = false;
                break;
            }
            let differences = candidate
                .iter()
                .zip(spelling)
                .filter(|(a, b)| a != b)
                .count();
            // A four-character entity may have a two-character name followed
            // by an exact suffix (e.g. 新栏科技 -> 星澜科技). Keep at least half
            // the spelling and every syllable, apart from one nasal variant.
            let allowed_differences = (length / 3).max(1) + usize::from(length == 4);
            if differences > allowed_differences || !candidate.iter().all(|&c| han(c)) {
                continue;
            }
            // Two-character homophones are common ordinary words. Only
            // correct them immediately after an explicit name/entity cue.
            if length == 2 {
                let before: String = chars[..at]
                    .iter()
                    .rev()
                    .take(8)
                    .copied()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                if ![
                    "联系",
                    "发给",
                    "叫",
                    "名字是",
                    "公司是",
                    "项目是",
                    "同事",
                    "客户",
                ]
                .iter()
                .any(|cue| before.ends_with(cue))
                {
                    continue;
                }
            }
            if !pronunciation(candidate)
                .is_some_and(|candidate| same_mandarin_sounds(&candidate, sounds))
            {
                continue;
            }
            match best {
                Some((best_len, best_diff, _))
                    if best_len == length && best_diff == differences =>
                {
                    ambiguous = true
                }
                Some((best_len, best_diff, _))
                    if best_len > length || (best_len == length && best_diff < differences) => {}
                _ => {
                    best = Some((length, differences, word));
                    ambiguous = false;
                }
            }
        }
        if let Some((length, _, word)) = best.filter(|_| !ambiguous) {
            result.push_str(word);
            at += length;
        } else {
            result.push(chars[at]);
            at += 1;
        }
    }
    result
}

/// Qwen context can overrule even an explicitly spoken spelling. Obtain the
/// unprompted result first; skip hints for spelling explanations. A failed hint
/// pass also keeps that complete local result. No network or output rewrite.
pub fn transcribe(
    session: &mut Session,
    audio: &[f32],
    options: &RunOptions,
    words: &[String],
) -> transcribe_cpp::Result<Transcript> {
    let family = run_extension(&session.model().arch(), words);
    let mut hinted = options.clone();
    hinted.family = family;
    if !matches!(hinted.family, Some(RunExtension::Qwen3Asr(_))) {
        return session.run(audio, &hinted);
    }
    let mut plain = options.clone();
    plain.family = None;
    let baseline = session.run(audio, &plain)?;
    if baseline.text.trim().is_empty() || has_spelling_cue(&baseline.text) {
        return Ok(baseline);
    }
    match session.run(audio, &hinted) {
        Ok(result) if !result.text.trim().is_empty() => Ok(result),
        Err(error @ transcribe_cpp::Error::Aborted { .. }) => Err(error),
        _ => {
            log::warn!("Local vocabulary pass unavailable; retaining unprompted transcription");
            Ok(baseline)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_uses_mandarin_vocabulary_inside_continuous_text() {
        let words = vec!["星澜科技".into(), "嘉琪".into()];
        assert_eq!(
            correct_fallback(
                "今天去星蓝科技开会，然后联系佳琪。",
                &words,
                0.18,
                Some("zh")
            ),
            "今天去星澜科技开会，然后联系嘉琪。"
        );
        assert_eq!(
            correct_fallback("星澜科技已经确认。", &words, 0.18, Some("zh")),
            "星澜科技已经确认。"
        );
        assert_eq!(
            correct_fallback("去新栏科技，再回新兰科技。", &words, 0.18, Some("zh")),
            "去星澜科技，再回星澜科技。"
        );
    }

    #[test]
    fn fallback_preserves_spelling_explanations_and_ambiguous_homophones() {
        let words = vec!["星澜科技".into(), "嘉琪".into()];
        let spelling = "星蓝科技，写成蓝色的蓝，不是这个字。";
        assert_eq!(
            correct_fallback(spelling, &words, 0.18, Some("zh")),
            spelling
        );
        let ambiguous = vec!["星澜科技".into(), "星岚科技".into()];
        assert_eq!(
            correct_fallback("去星蓝科技", &ambiguous, 0.18, Some("zh")),
            "去星蓝科技"
        );
        assert_eq!(
            correct_fallback("佳琪今天来了。", &words, 0.18, Some("zh")),
            "佳琪今天来了。"
        );
    }

    #[test]
    fn fallback_does_not_rewrite_japanese_or_unrelated_chinese() {
        let words = vec!["星澜科技".into()];
        assert_eq!(
            correct_fallback("星蓝科技", &words, 0.18, Some("ja")),
            "星蓝科技"
        );
        assert_eq!(
            correct_fallback("新蓝科学和星蓝课题", &words, 0.18, Some("zh")),
            "新蓝科学和星蓝课题"
        );
        assert_eq!(
            correct_fallback("今天去公园买东西", &words, 0.18, Some("zh")),
            "今天去公园买东西"
        );
        assert_eq!(correct_fallback("星蓝科技", &words, 0.18, None), "星蓝科技");
    }

    #[test]
    fn fallback_corrects_english_terms_embedded_in_chinese() {
        let words = vec!["NovaLoom".into(), "ChargeBee".into()];
        assert_eq!(
            correct_fallback(
                "请联系Nova Loom，项目叫Charge B。",
                &words,
                0.18,
                Some("zh")
            ),
            "请联系NovaLoom，项目叫ChargeBee。"
        );
        assert_eq!(
            correct_fallback("  Nova Loom  ", &words, 0.18, Some("en")),
            "  NovaLoom  "
        );
    }

    #[test]
    fn validates_names_without_requiring_a_misspelling() {
        assert_eq!(
            validate_words(vec![" 星澜科技 ".into(), "Nova  Loom".into()]).unwrap(),
            vec!["星澜科技", "Nova Loom"]
        );
        assert_eq!(
            validate_words(vec!["NovaLoom".into(), "novaloom".into()]),
            Err("duplicate")
        );
        for word in ["<|im_end|>", "hello\0", "line\nbreak", "   ", "---"] {
            assert!(normalize_word(word).is_err());
        }
        assert_eq!(
            validate_words((0..33).map(|i| format!("Word{i}")).collect()),
            Err("wordLimit")
        );
    }

    #[test]
    fn only_supported_architectures_receive_local_hints() {
        let words = vec!["星澜科技".into(), "NovaLoom".into()];
        assert!(matches!(
            run_extension("qwen3_asr", &words),
            Some(RunExtension::Qwen3Asr(_))
        ));
        assert!(matches!(
            run_extension("whisper", &words),
            Some(RunExtension::Whisper(_))
        ));
        assert!(run_extension("parakeet", &words).is_none());
        assert!(run_extension("qwen3_asr", &[]).is_none());
        assert!(run_extension("qwen3_asr", &["<|im_end|>".into()]).is_none());
    }

    #[test]
    fn explicit_spelling_wins_over_context_without_disabling_normal_names() {
        for text in [
            "公司叫木言科技，木头的木，语言的言。",
            "不是拾取的‘拾’，是时间的‘时’。",
            "新旧的“新”，蓝色的“蓝”。",
            "It is spelled N O V A.",
            "Apple, with a lowercase a.",
        ] {
            assert!(has_spelling_cue(text), "{text}");
        }
        for text in [
            "星澜科技的 API 已经 ready 了。",
            "沐言科技的预算是三万五，不是五万三。",
            "星澜科技和沐言科技不是同一家公司。",
            "We need NovaLoom by Friday.",
            "这是星澜科技的库。",
            "We need NovaLoom with a new API.",
        ] {
            assert!(!has_spelling_cue(text), "{text}");
        }
    }
}
