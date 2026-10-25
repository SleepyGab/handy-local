#!/usr/bin/env python3
"""Fail closed if an upstream merge removes Handy Local's integration points.

These structural checks complement the Rust and browser behavior tests. They
deliberately require review when upstream refactors a protected integration.
"""

import json
import re
import sys
from pathlib import Path


def check(root: Path) -> list[str]:
    errors = []

    def require(condition: bool, message: str) -> None:
        if not condition:
            errors.append(message)

    def source(path: str) -> str:
        file = root / path
        if not file.is_file() or file.is_symlink():
            errors.append(f"Missing regular file: {path}")
            return ""
        return file.read_text()

    def contains(path: str, *snippets: str) -> None:
        text = source(path)
        for snippet in snippets:
            require(snippet in text, f"Review {path}: missing {snippet!r}")

    def document(path: str) -> dict:
        try:
            return json.loads(source(path))
        except (ValueError, TypeError):
            errors.append(f"Invalid JSON: {path}")
            return {}

    config = document("src-tauri/tauri.corrections.conf.json")
    require(config.get("productName") == "Handy Local", "App name changed")
    require(
        config.get("identifier") == "com.gabrielle.handylocal",
        "App/data identity changed",
    )
    require(
        config.get("bundle", {}).get("createUpdaterArtifacts") is False,
        "Official updater artifacts must stay disabled",
    )
    csp = config.get("app", {}).get("security", {}).get("csp", "")
    require(
        "connect-src ipc: http://ipc.localhost;" in csp,
        "Review local webview network policy",
    )
    package = document("package.json")
    build = package.get("scripts", {}).get("build:local", "")
    require(
        "--features local-only" in build
        and "--config src-tauri/tauri.corrections.conf.json" in build,
        "build:local must use the local feature and identity",
    )
    contains("src-tauri/Cargo.toml", "local-only = []")
    contains("src-tauri/Cargo.toml", 'path = "vendor/transcribe-cpp"', 'path = "vendor/transcribe-cpp-sys"')
    contains("src-tauri/src/managers/transcription.rs", "crate::vocabulary::transcribe(")
    contains("src-tauri/src/vocabulary.rs", "has_spelling_cue", "RunExtension::Qwen3Asr", "session.run(audio, &plain)")
    contains("src/corrections/Corrections.tsx", "<Vocabulary />")
    contains("src-tauri/vendor/transcribe-cpp-sys/src/arch/qwen3_asr/model.cpp", "encode_context", "accepts_context")
    contains("src-tauri/build.rs", "build_local_corrections_bridge();")
    contains(
        "src-tauri/src/lib.rs",
        "mod local_corrections;",
        "local_corrections::initialize(app_handle)",
        "local_corrections::corrections_state",
        "local_corrections::corrections_action",
        "local_corrections::transfer::corrections_transfer",
    )
    contains(
        "src-tauri/src/actions.rs",
        'post_process && !cfg!(feature = "local-only")',
        "local_corrections::apply(",
        "local_corrections::capture(",
        "local_corrections::begin(",
        "local_corrections::clear(",
    )
    settings = source("src-tauri/src/settings.rs")
    require(
        bool(
            re.search(
                r'fn update_checks_forced_disabled\(\).*?if cfg!\(feature = "local-only"\)\s*\{\s*return true;',
                settings,
                re.DOTALL,
            )
        ),
        "local-only must force app update checks off",
    )
    contains(
        "src-tauri/src/llm_client.rs",
        'cfg!(feature = "local-only")',
        "Cloud processing is disabled in Handy Local",
    )
    contains(
        "src-tauri/src/local_corrections.rs",
        "corrections_are_non_cascading_and_word_bounded",
        "dictionary_persists_only_confirmed_rules",
        "local_only_rejects_cloud_text_before_creating_a_request",
        "temporary_ax_gaps_are_bounded_and_focus_changes_stop_immediately",
    )
    for path in [
        "src-tauri/src/corrections_transfer.rs",
        "src-tauri/native/local_corrections.m",
        "src/corrections/Corrections.tsx",
        "src/corrections/DictionaryTransfer.tsx",
        "tests/corrections.spec.ts",
        "tests/dictionary-transfer.spec.ts",
    ]:
        require(bool(source(path)), f"Local correction component/test removed: {path}")
    contains("src/components/Sidebar.tsx", "corrections")
    for locale in ["en", "zh", "zh-TW"]:
        contains(f"src/i18n/locales/{locale}/translation.json", '"corrections"')
    workflows = root / ".github/workflows"
    allowed = {"upstream-sync.yml", "local-validation.yml"}
    actual = {p.name for p in workflows.iterdir()} if workflows.is_dir() else set()
    require(
        actual == allowed,
        f"Review active workflows (upstream workflows must remain archived): {sorted(actual)}",
    )
    return errors


if __name__ == "__main__":
    root = (
        Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[1]
    )
    errors = check(root)
    if errors:
        sys.exit("\n".join(errors))
    print("Handy Local integration and privacy guards passed.")
