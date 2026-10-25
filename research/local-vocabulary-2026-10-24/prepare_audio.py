"""Regenerate synthetic WAVs locally on macOS; no microphone or cloud TTS."""
import json
import subprocess
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent
    cases = json.loads((root / "final-cases.json").read_text())
    for case in cases:
        wav = root / case["wav"]
        wav.parent.mkdir(parents=True, exist_ok=True)
        script = wav.with_suffix(".txt")
        script.write_text(case["spoken"])
        if not wav.exists():
            subprocess.run([
                "say", "-v", case["voice"], "-r", "175", "-f", str(script),
                "-o", str(wav), "--file-format=WAVE", "--data-format=LEI16@16000",
            ], check=True)
    print(f"Prepared {len(cases)} synthetic local WAVs")


if __name__ == "__main__":
    main()
