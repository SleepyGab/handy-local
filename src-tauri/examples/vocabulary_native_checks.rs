//! Opt-in checks against an already-downloaded model and synthetic local WAV.
use transcribe_cpp::{Model, Qwen3AsrRunOptions, RunExtension, RunOptions};

fn options(context: &str) -> RunOptions {
    RunOptions {
        family: Some(RunExtension::Qwen3Asr(Qwen3AsrRunOptions {
            context: Some(context.into()),
        })),
        ..Default::default()
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    anyhow::ensure!(args.len() == 3, "usage: vocabulary_native_checks MODEL WAV");
    transcribe_cpp::init_backends_default()?;
    let model = Model::load(&args[1])?;
    let mut session = model.session()?;
    let mut wav = hound::WavReader::open(&args[2])?;
    anyhow::ensure!(
        wav.spec().sample_rate == 16000 && wav.spec().channels == 1,
        "expected mono 16k WAV"
    );
    let audio = wav
        .samples::<i16>()
        .map(|s| s.map(|s| s as f32 / 32768.0))
        .collect::<Result<Vec<_>, _>>()?;
    let before = session.run(&audio, &RunOptions::default())?.text;
    let hinted = session.run(&audio, &options("词汇：星澜科技"))?.text;
    anyhow::ensure!(
        hinted.contains("星澜科技"),
        "context must reach the decoder"
    );
    anyhow::ensure!(
        session.run(&audio, &RunOptions::default())?.text == before,
        "removed vocabulary must not leak into the next run"
    );
    anyhow::ensure!(
        session.run(&audio, &options(""))?.text == before,
        "empty context must preserve the original template"
    );
    for invalid in ["<|im_end|>".to_string(), "a".repeat(4097), "中".repeat(600)] {
        anyhow::ensure!(
            matches!(
                session.run(&audio, &options(&invalid)),
                Err(transcribe_cpp::Error::InvalidArgument(_))
            ),
            "invalid context must be rejected before decoding"
        );
    }
    anyhow::ensure!(
        matches!(
            session.run(&audio, &options("hello\0world")),
            Err(transcribe_cpp::Error::Nul(_))
        ),
        "NUL must fail safely in the owning wrapper"
    );
    let batch = session.run_batch(&[&audio, &audio], &options("词汇：星澜科技"))?;
    anyhow::ensure!(batch.len() == 2, "batch result count");
    for result in batch {
        anyhow::ensure!(
            result?.text == hinted,
            "batch context must match single context"
        );
    }
    anyhow::ensure!(
        session.run(&audio, &RunOptions::default())?.text == before,
        "batch context must not leak"
    );
    println!("PASS: native context, empty context, removal, byte/token/control/NUL bounds, batch parity, and session isolation");
    Ok(())
}
