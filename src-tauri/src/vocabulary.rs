//! Local ASR hints. Shared by dictation and the audio evaluation harness.
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
