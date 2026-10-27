//! Evaluate installed models with the dictionary and live path used by dictation.
//! Usage: cargo run --example dictation_eval -- MODEL LANGUAGE WORDS_JSON WAV...

use handy_app_lib::audio_toolkit;
#[path = "../src/long_form.rs"]
#[allow(dead_code)]
mod long_form;
#[path = "../src/vocabulary.rs"]
#[allow(dead_code)]
mod vocabulary;

use std::time::Instant;
use transcribe_cpp::{CancelToken, Model, RunOptions, StreamOptions};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    anyhow::ensure!(
        args.len() >= 5,
        "usage: dictation_eval MODEL LANGUAGE WORDS_JSON WAV..."
    );
    let words: Vec<String> = serde_json::from_str(&std::fs::read_to_string(&args[3])?)?;
    let words = vocabulary::validate_words(words).map_err(anyhow::Error::msg)?;
    transcribe_cpp::init_backends_default()?;
    let model = Model::load(&args[1])?;
    let streaming = model.capabilities().supports_streaming;
    let prompted = model.arch() == "qwen3_asr";
    let mut session = model.session()?;
    let options = RunOptions {
        language: Some(args[2].clone()),
        ..Default::default()
    };
    for wav in &args[4..] {
        let mut reader = hound::WavReader::open(wav)?;
        let spec = reader.spec();
        anyhow::ensure!(
            spec.channels == 1 && spec.sample_rate == 16_000 && spec.bits_per_sample == 16,
            "expected 16 kHz mono int16 WAV"
        );
        let audio = reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / 32768.0))
            .collect::<Result<Vec<_>, _>>()?;
        for live in [false, true] {
            let started = Instant::now();
            let mut updates = 0;
            let tail_started;
            let raw = if !live {
                tail_started = Instant::now();
                session.run(&audio, &options)?.text
            } else if streaming {
                let mut stream = session.stream(&options, &StreamOptions::default())?;
                for chunk in audio.chunks(1600) {
                    let update = stream.feed(chunk)?;
                    updates += usize::from(update.committed_changed || update.tentative_changed);
                }
                tail_started = Instant::now();
                stream.finalize()?;
                stream.text().full
            } else {
                let mut decoder = long_form::LiveDecoder::default();
                for chunk in audio.chunks(1600) {
                    updates += usize::from(decoder.feed(chunk, &mut session, &options, &words)?);
                }
                anyhow::ensure!(decoder.samples == audio.len(), "live sample count mismatch");
                tail_started = Instant::now();
                decoder.finish(&mut session, &options, &words)?;
                decoder.text().to_string()
            };
            let finish_seconds = tail_started.elapsed().as_secs_f64();
            let text = if live && !prompted {
                vocabulary::correct_fallback(&raw, &words, 0.18, Some(&args[2]))
            } else {
                raw.clone()
            };
            println!(
                "{}",
                serde_json::json!({"wav":wav, "live":live, "arch":model.arch(),
                "audio_seconds":audio.len() as f64 / 16_000.0, "seconds":started.elapsed().as_secs_f64(),
                "finish_seconds":finish_seconds, "updates_before_stop":updates, "raw":raw,"text":text})
            );
        }
        // Prove cancellation reaches the native engine and clearing its token
        // permits the following dictation to run on the same loaded session.
        if model.supports(transcribe_cpp::Feature::Cancellation) {
            let token = CancelToken::new();
            session.set_cancel_token(&token);
            token.cancel();
            anyhow::ensure!(
                matches!(
                    session.run(&audio[..audio.len().min(16000)], &options),
                    Err(transcribe_cpp::Error::Aborted { .. })
                ),
                "cancel was not observed"
            );
            session.clear_cancel_token();
            session.run(&audio[..audio.len().min(3 * 16000)], &options)?;
        }
    }
    Ok(())
}
