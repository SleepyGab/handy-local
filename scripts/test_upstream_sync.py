"""Exercise release policy and real Git merges without GitHub or app data."""

import argparse
import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import upstream_sync as sync
from check_local_fork import check


class ReleasePolicyTests(unittest.TestCase):
    def test_accepts_only_published_stable_release_without_downgrade(self):
        state = {"release": "v0.9.8"}
        release = {
            "tag_name": "v0.10.0",
            "draft": False,
            "prerelease": False,
            "published_at": "2026-11-01",
        }
        self.assertEqual(sync.validate_release(release, state), "v0.10.0")
        for changed in [
            {"draft": True},
            {"prerelease": True},
            {"published_at": None},
            {"tag_name": "v0.9.9-beta.1"},
            {"tag_name": "main"},
            {"tag_name": "v0.9.7"},
        ]:
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                sync.validate_release({**release, **changed}, state)

    def test_pr_draft_matches_template_without_a_human_written_requirement(self):
        body = sync.pr_body({"tag": "v0.10.0"})
        template = (
            Path(__file__).resolve().parents[1] / ".github/PULL_REQUEST_TEMPLATE.md"
        ).read_text()
        for line in template.splitlines():
            if line.startswith("## "):
                self.assertIn(line, body)
        self.assertNotIn("Human Written Description", body)
        self.assertNotIn("TODO: The maintainer must write", body)
        self.assertIn("- [ ] Native Mac smoke test", body)
        self.assertIn("- [x] AI was used", body)


class MergeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name) / "repo"
        self.root.mkdir()
        self.git("init", "-b", sync.BASE_BRANCH)
        self.git("config", "user.name", "Fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("remote", "add", "origin", str(self.root))
        self.write("README.md", "official old\n")
        self.write("engine.txt", "old engine\n")
        self.write("shared.txt", "base line\n")
        self.write(".github/workflows/release.yml", "official old release\n")
        self.commit("official base")
        self.baseline = self.git("rev-parse", "HEAD")
        self.git("branch", "upstream-fixture")
        self.git("mv", ".github/workflows", ".github/upstream-workflows")
        self.write(".github/workflows/upstream-sync.yml", "fork sync\n")
        self.write(".github/workflows/local-validation.yml", "fork validation\n")
        self.write("README.md", "fork readme\n")
        self.write("local-corrections.txt", "confirmed dictionary only\n")
        self.write("shared.txt", "fork line\n")
        self.state = {
            "repository": sync.UPSTREAM,
            "release": "v0.9.8",
            "commit": self.baseline,
        }
        self.write(sync.STATE, json.dumps(self.state))
        self.commit("fork features")
        self.base = self.git("rev-parse", "HEAD")

    def tearDown(self):
        self.temporary.cleanup()

    def git(self, *args):
        return sync.git(self.root, *args)

    def write(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def commit(self, message):
        self.git("add", "-A")
        self.git("commit", "-m", message)

    def release(self, conflict=False):
        self.git("switch", "upstream-fixture")
        self.write("engine.txt", "fixed upstream engine\n")
        self.write("README.md", "official new\n")
        self.write(".github/workflows/release.yml", "official new release\n")
        self.write(".github/workflows/new-publish.yml", "must stay archived\n")
        if conflict:
            self.write("shared.txt", "upstream competing line\n")
        self.commit("official next release")
        commit = self.git("rev-parse", "HEAD")
        self.git("switch", sync.BASE_BRANCH)
        return commit

    def prepare(self, upstream):
        return sync.prepare(
            self.root, self.state, "v0.10.0", upstream, "codex/upstream-v0.10.0", False
        )

    def synchronize(self, upstream, tag="v0.10.0", publish=False):
        release = {
            "tag_name": tag,
            "draft": False,
            "prerelease": False,
            "published_at": "2026-11-01",
        }
        actual_git, actual_run = sync.git, sync.run

        def fake_git(root, *args, **kwargs):
            if args[:3] == (
                "fetch",
                "--no-tags",
                "https://github.com/cjpais/Handy.git",
            ):
                self.write(".git/FETCH_HEAD", upstream + "\n")
                return ""
            return actual_git(root, *args, **kwargs)

        def fake_run(*args, **kwargs):
            if args[0] == "gh":
                return subprocess.CompletedProcess(args, 0, json.dumps(release), "")
            return actual_run(*args, **kwargs)

        with (
            patch.object(sync, "git", side_effect=fake_git),
            patch.object(sync, "run", side_effect=fake_run),
        ):
            return sync.synchronize(
                self.root, argparse.Namespace(tag=None, prepare=True, publish=publish)
            )

    def test_existing_branch_is_reused_and_manual_edits_survive(self):
        upstream = self.release()
        with patch.object(sync, "check_local_fork", return_value=[]):
            prepared = self.prepare(upstream)
        self.git("switch", "codex/upstream-v0.10.0")
        self.write("review.txt", "maintainer reviewed\n")
        self.commit("manual review")
        reviewed = self.git("rev-parse", "HEAD")
        self.git("switch", sync.BASE_BRANCH)
        result = self.synchronize(upstream)
        self.assertEqual(result["status"], "existing")
        self.assertEqual(result["commit"], reviewed)
        self.assertNotEqual(result["commit"], prepared["commit"])
        self.assertEqual(
            self.git("show", f"{reviewed}:review.txt"), "maintainer reviewed"
        )

    def test_existing_branch_must_include_new_base_changes_before_validation(self):
        upstream = self.release()
        with patch.object(sync, "check_local_fork", return_value=[]):
            self.prepare(upstream)
        self.write("base-fix.txt", "important local change\n")
        self.commit("new base change")
        with self.assertRaisesRegex(RuntimeError, "behind the current base"):
            self.synchronize(upstream)

    def test_release_tag_cannot_move_silently(self):
        with self.assertRaisesRegex(RuntimeError, "release tag moved"):
            self.synchronize(self.release(), tag="v0.9.8")

    def test_publish_rejects_an_unexpected_push_destination(self):
        with self.assertRaisesRegex(RuntimeError, "outside the Handy Local fork"):
            self.synchronize(self.release(), publish=True)

    def test_merge_retains_both_features_archives_ci_and_records_ancestry(self):
        upstream = self.release()
        with patch.object(sync, "check_local_fork", return_value=[]):
            result = self.prepare(upstream)
        self.assertEqual(result["status"], "ready")
        ref = result["commit"]
        self.assertEqual(self.git("show", f"{ref}:engine.txt"), "fixed upstream engine")
        self.assertEqual(
            self.git("show", f"{ref}:local-corrections.txt"),
            "confirmed dictionary only",
        )
        self.assertEqual(self.git("show", f"{ref}:README.md"), "fork readme")
        self.assertEqual(self.git("show", f"{ref}:README.upstream.md"), "official new")
        self.assertEqual(
            self.git("show", f"{ref}:.github/upstream-workflows/new-publish.yml"),
            "must stay archived",
        )
        active = self.git(
            "ls-tree", "-r", "--name-only", ref, "--", ".github/workflows"
        ).splitlines()
        self.assertEqual(
            active,
            [
                ".github/workflows/local-validation.yml",
                ".github/workflows/upstream-sync.yml",
            ],
        )
        self.assertEqual(
            json.loads(self.git("show", f"{ref}:{sync.STATE}"))["commit"], upstream
        )
        self.git("merge-base", "--is-ancestor", upstream, ref)
        self.git("merge-base", "--is-ancestor", self.base, ref)
        self.assertEqual(self.git("rev-parse", "HEAD"), self.base)
        self.assertEqual(self.git("status", "--porcelain"), "")
        self.assertEqual(
            self.git("worktree", "list", "--porcelain").count("worktree "), 1
        )

    def test_conflict_leaves_checkout_baseline_and_branch_untouched(self):
        result = self.prepare(self.release(conflict=True))
        self.assertEqual(result["status"], "conflict")
        self.assertEqual(result["conflicts"], ["shared.txt"])
        self.assertEqual(self.git("rev-parse", "HEAD"), self.base)
        self.assertEqual(self.git("status", "--porcelain"), "")
        self.assertEqual(json.loads((self.root / sync.STATE).read_text()), self.state)
        self.assertEqual(self.git("branch", "--list", "codex/upstream-*"), "")
        self.assertEqual(
            self.git("worktree", "list", "--porcelain").count("worktree "), 1
        )

    def test_guard_failure_prevents_branch_creation(self):
        upstream = self.release()
        with patch.object(
            sync, "check_local_fork", return_value=["local-only removed"]
        ):
            result = self.prepare(upstream)
        self.assertEqual(result["status"], "guard-failed")
        self.assertEqual(self.git("branch", "--list", "codex/upstream-*"), "")

    def test_dirty_checkout_is_rejected_without_losing_user_changes(self):
        upstream = self.release()
        self.write("user-notes.txt", "unfinished work\n")
        with self.assertRaisesRegex(RuntimeError, "Commit or stash"):
            self.prepare(upstream)
        self.assertEqual(
            (self.root / "user-notes.txt").read_text(), "unfinished work\n"
        )

    def test_repeated_prepare_cannot_overwrite_existing_branch(self):
        upstream = self.release()
        self.git("branch", "codex/upstream-v0.10.0")
        with (
            patch.object(sync, "check_local_fork", return_value=[]),
            self.assertRaises(RuntimeError),
        ):
            self.prepare(upstream)
        self.assertEqual(self.git("rev-parse", "codex/upstream-v0.10.0"), self.base)

    def test_already_integrated_release_never_prepares_a_branch(self):
        release = {
            "tag_name": "v0.9.8",
            "draft": False,
            "prerelease": False,
            "published_at": "2026-10-03",
        }
        self.write(".git/FETCH_HEAD", self.baseline + "\n")
        actual_git = sync.git
        actual_run = sync.run

        def fake_git(root, *args, **kwargs):
            if args[0] == "fetch":
                return ""
            return actual_git(root, *args, **kwargs)

        def fake_run(*args, **kwargs):
            if args[0] == "gh":
                self.assertEqual(args[2], "repos/cjpais/Handy/releases/latest")
                return subprocess.CompletedProcess(args, 0, json.dumps(release), "")
            return actual_run(*args, **kwargs)

        with (
            patch.object(sync, "git", side_effect=fake_git),
            patch.object(sync, "run", side_effect=fake_run),
        ):
            result = sync.synchronize(
                self.root, argparse.Namespace(tag=None, prepare=True, publish=False)
            )
        self.assertEqual(result["status"], "current")
        self.assertEqual(self.git("branch", "--list", "codex/upstream-*"), "")


class ForkGuardTests(unittest.TestCase):
    def test_real_fork_passes_and_identity_or_integration_loss_fails(self):
        root = Path(__file__).resolve().parents[1]
        self.assertEqual(check(root), [])
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Path(temporary)
            # Only copy text sources needed by guard checks, never app data.
            for relative in [
                "src-tauri/tauri.corrections.conf.json",
                "package.json",
                "src-tauri/Cargo.toml",
                "src-tauri/build.rs",
                "src-tauri/src/lib.rs",
                "src-tauri/src/actions.rs",
                "src-tauri/src/settings.rs",
                "src-tauri/src/llm_client.rs",
                "src-tauri/src/cloud_polish.rs",
                "src-tauri/src/vocabulary.rs",
                "src-tauri/src/managers/transcription.rs",
                "src-tauri/vendor/transcribe-cpp-sys/src/arch/qwen3_asr/model.cpp",
                "src-tauri/src/local_corrections.rs",
                "src-tauri/src/corrections_transfer.rs",
                "src-tauri/native/local_corrections.m",
                "src/corrections/Corrections.tsx",
                "src/corrections/DictionaryTransfer.tsx",
                "tests/corrections.spec.ts",
                "tests/dictionary-transfer.spec.ts",
                "src/components/Sidebar.tsx",
                "src/i18n/locales/en/translation.json",
                "src/i18n/locales/zh/translation.json",
                "src/i18n/locales/zh-TW/translation.json",
                ".github/workflows/local-validation.yml",
                ".github/workflows/upstream-sync.yml",
            ]:
                destination = fixture / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(root / relative, destination)
            self.assertEqual(check(fixture), [])
            settings = fixture / "src-tauri/src/settings.rs"
            original_settings = settings.read_text()
            settings.write_text(original_settings.replace("cloud_polish_enabled: false", "cloud_polish_enabled: true"))
            self.assertTrue(any("cloud_polish_enabled" in error for error in check(fixture)))
            settings.write_text(original_settings)
            package = fixture / "package.json"
            original_package = package.read_text()
            package.write_text(original_package.replace("python3 scripts/with_macos_signing.py -- ", ""))
            self.assertTrue(any("stable macOS signing" in error for error in check(fixture)))
            package.write_text(original_package)
            config = fixture / "src-tauri/tauri.corrections.conf.json"
            data = json.loads(config.read_text())
            data["identifier"] = "com.pais.handy"
            config.write_text(json.dumps(data))
            (fixture / "src-tauri/src/actions.rs").write_text("// integration lost\n")
            (fixture / ".github/workflows/release.yml").write_text("enabled\n")
            errors = check(fixture)
            self.assertTrue(any("identity" in error for error in errors))
            self.assertTrue(any("apply(" in error for error in errors))
            self.assertTrue(any("workflows" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
