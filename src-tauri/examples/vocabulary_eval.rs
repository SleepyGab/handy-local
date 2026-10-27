//! Local WAV evaluation using exactly the dictionary prompt used by dictation.
use handy_app_lib::audio_toolkit;
#[path = "../src/vocabulary.rs"]
#[allow(dead_code)]
mod vocabulary;

use std::io::Write;
use std::time::Instant;
use transcribe_cpp::{Model, RunOptions};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 4,
        "usage: vocabulary_eval MODEL CASES_JSON OUTPUT_JSONL"
    );
    transcribe_cpp::init_backends_default()?;
    let started = Instant::now();
    let model = Model::load(&args[1])?;
    anyhow::ensure!(model.arch() == "qwen3_asr", "evaluation requires Qwen3-ASR");
    let mut session = model.session()?;
    let cases: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[2])?)?;
    let mut output = std::fs::File::create(&args[3])?;
    writeln!(
        output,
        "{}",
        serde_json::json!({"type":"load", "seconds":started.elapsed().as_secs_f64(), "backend":model.backend()})
    )?;
    for (index, case) in cases
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("cases must be array"))?
        .iter()
        .enumerate()
    {
        let wav = case["wav"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing wav"))?;
        let wav = std::path::Path::new(&args[2])
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(wav);
        let mut reader = hound::WavReader::open(wav)?;
        let spec = reader.spec();
        anyhow::ensure!(
            spec.channels == 1 && spec.sample_rate == 16000 && spec.bits_per_sample == 16,
            "expected 16k mono int16 WAV"
        );
        let audio = reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / 32768.0))
            .collect::<Result<Vec<_>, _>>()?;
        let words: Vec<String> = serde_json::from_value(case["words"].clone())?;
        let words = vocabulary::validate_words(words).map_err(anyhow::Error::msg)?;
        // Alternate order to limit warm-up/order bias. Reuse one session to
        // expose context leaking into a subsequent no-hint invocation.
        for hinted in if index % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let options = RunOptions::default();
            let start = Instant::now();
            let result = vocabulary::transcribe(
                &mut session,
                &audio,
                &options,
                if hinted { &words } else { &[] },
            );
            let row = match result {
                Ok(transcript) => {
                    serde_json::json!({"id":case["id"],"hinted":hinted,"seconds":start.elapsed().as_secs_f64(),"audio_seconds":audio.len() as f64 / 16000.0,"text":transcript.text})
                }
                Err(error) => {
                    serde_json::json!({"id":case["id"],"hinted":hinted,"seconds":start.elapsed().as_secs_f64(),"error":error.to_string()})
                }
            };
            writeln!(output, "{row}")?;
            output.flush()?;
            println!("{row}");
        }
    }
    Ok(())
}
