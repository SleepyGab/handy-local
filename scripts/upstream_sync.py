#!/usr/bin/env python3
"""Prepare a reviewable merge of an official stable Handy release.

Requires Git, GitHub CLI and Python 3.10+. Never merges into the current branch,
opens a PR, force-pushes, runs upstream code, or publishes an app release.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from urllib.parse import urlencode

from check_local_fork import check as check_local_fork

UPSTREAM = "cjpais/Handy"
FORK = "SleepyGab/nibi"
BASE_BRANCH = "nibi"
STATE = ".github/upstream-sync.json"
# Only fork-owned metadata/tooling is kept wholesale. Runtime source is merged
# normally so upstream fixes are retained; conflicts there require human review.
OWNED = [
    ".github/workflows",
    STATE,
    ".github/PULL_REQUEST_TEMPLATE.md",
    "AGENTS.md",
    "README.md",
    "CONTRIBUTING_LOCAL.md",
    "LOCAL_CORRECTIONS.md",
    "NOTICE.md",
    "src-tauri/tauri.corrections.conf.json",
    "docs/UPSTREAM_SYNC.zh-CN.md",
    "scripts/upstream_sync.py",
    "scripts/check_local_fork.py",
    "scripts/test_upstream_sync.py",
]


def run(
    *args: str, cwd: Path, ok: tuple[int, ...] = (0,)
) -> subprocess.CompletedProcess:
    result = subprocess.run(args, cwd=cwd, text=True, capture_output=True, check=False)
    if result.returncode not in ok:
        raise RuntimeError(
            f"{args[0]} {args[1]} failed: {result.stderr.strip() or result.stdout.strip()}"
        )
    return result


def git(root: Path, *args: str, ok: tuple[int, ...] = (0,)) -> str:
    # Do not run checkout hooks or commit hooks while preparing a release.
    return run(
        "git", "-c", "core.hooksPath=/dev/null", *args, cwd=root, ok=ok
    ).stdout.strip()


def version(tag: str) -> tuple[int, int, int]:
    if not isinstance(tag, str) or not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
        raise ValueError(f"Expected a stable vMAJOR.MINOR.PATCH release tag: {tag!r}")
    return tuple(int(part) for part in tag[1:].split("."))


def validate_release(release: dict, state: dict) -> str:
    tag = release.get("tag_name", "")
    version(tag)
    if (
        release.get("draft") is not False
        or release.get("prerelease") is not False
        or not release.get("published_at")
    ):
        raise ValueError(
            "Only published, non-draft, non-prerelease releases are eligible"
        )
    if version(tag) < version(state["release"]):
        raise ValueError("Refusing to downgrade the integrated upstream release")
    return tag


def preserve_fork_files(root: Path, base: str, upstream: str) -> None:
    for path in OWNED:
        if git(root, "ls-files", "--", path) or git(
            root, "ls-tree", "-r", "--name-only", base, "--", path
        ):
            git(
                root,
                "restore",
                f"--source={base}",
                "--staged",
                "--worktree",
                "--",
                path,
            )
    # Refresh upstream references without enabling their CI or release jobs.
    git(root, "rm", "-r", "-f", "--ignore-unmatch", ".github/upstream-workflows")
    files = git(
        root, "ls-tree", "-r", "--name-only", upstream, "--", ".github/workflows"
    ).splitlines()
    for path in files:
        relative = Path(path).relative_to(".github/workflows")
        destination = root / ".github/upstream-workflows" / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(git(root, "show", f"{upstream}:{path}") + "\n")
    (root / "README.upstream.md").write_text(
        git(root, "show", f"{upstream}:README.md") + "\n"
    )
    git(root, "add", "-A", "--", ".github/upstream-workflows", "README.upstream.md")


def prepare(
    root: Path, state: dict, tag: str, upstream: str, branch: str, publish: bool
) -> dict:
    if git(root, "status", "--porcelain"):
        raise RuntimeError(
            "Commit or stash working changes before preparing a sync; upstream:check remains available"
        )
    base = git(root, "rev-parse", "HEAD")
    # A temporary detached worktree keeps the user's checkout and current branch
    # untouched, including when a merge fails. Only successful merges get a ref.
    with tempfile.TemporaryDirectory(prefix="handy-upstream-") as temporary:
        worktree = Path(temporary) / "checkout"
        git(root, "worktree", "add", "--detach", str(worktree), base)
        try:
            merge = run(
                "git",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "user.name=Handy Local Sync",
                "-c",
                "user.email=handy-local-sync@users.noreply.github.com",
                "merge",
                "--no-commit",
                "--no-ff",
                upstream,
                cwd=worktree,
                ok=(0, 1),
            )
            preserve_fork_files(worktree, base, upstream)
            conflicts = git(
                worktree, "diff", "--name-only", "--diff-filter=U"
            ).splitlines()
            if conflicts:
                return {
                    "status": "conflict",
                    "conflicts": conflicts,
                    "message": "Manual resolution required. No sync branch was published; the baseline is unchanged.",
                }
            # An unrelated failure must never be treated as a successful merge.
            if merge.returncode and not git(
                worktree, "rev-parse", "--verify", "MERGE_HEAD", ok=(0, 128)
            ):
                raise RuntimeError(merge.stderr)
            errors = check_local_fork(worktree)
            if errors:
                return {
                    "status": "guard-failed",
                    "errors": errors,
                    "message": "Local correction/privacy checks failed. No sync branch was published.",
                }
            (worktree / STATE).write_text(
                json.dumps({**state, "release": tag, "commit": upstream}, indent=2)
                + "\n"
            )
            git(worktree, "add", STATE)
            git(
                worktree,
                "-c",
                "user.name=Handy Local Sync",
                "-c",
                "user.email=handy-local-sync@users.noreply.github.com",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-m",
                f"chore: integrate Handy {tag} while retaining local corrections",
            )
            commit = git(worktree, "rev-parse", "HEAD")
            # Empty expected old value means creation only, never overwrite work.
            git(root, "update-ref", f"refs/heads/{branch}", commit, "")
            if publish:
                # The lease only permits a new remote branch. It cannot replace
                # a branch someone created since the earlier existence check.
                git(
                    root,
                    "push",
                    f"--force-with-lease=refs/heads/{branch}:",
                    "origin",
                    f"{commit}:refs/heads/{branch}",
                )
            return {
                "status": "ready",
                "commit": commit,
                "message": "Sync branch prepared. Run validation and native smoke tests before merging.",
            }
        finally:
            git(root, "worktree", "remove", "--force", str(worktree))


def pr_body(result: dict) -> str:
    return f"""## Description

Merge official Handy **{result["tag"]}** into `{BASE_BRANCH}`, preserving local
corrections, local-only behavior and the fork's app identity.
Release: https://github.com/{UPSTREAM}/releases/tag/{result["tag"]}
Upstream commit: `{result.get("upstream_commit", "")}`

This is a release sync into **{FORK}**. Review the prepared merge before submission.

## Related Issues/Discussions

Scheduled stable-release maintenance. Add relevant bug links after reviewing the release.

## Testing

- [ ] Both Handy Local validation jobs passed for the current branch commit.
- [ ] Native Mac smoke test: Fn recording, paste, explicit confirmation, persistence.
- [ ] Dictionary import/export, conflict preview and backup verified.
- [ ] Independent app/data identity and local-only restrictions preserved.

## Screenshots/Videos (if applicable)

Attach evidence if the update changes the UI; otherwise mark not applicable.

## AI Assistance

- [ ] No AI was used in this PR
- [x] AI was used (describe the tools and scope below)

Tools used: Codex helped implement the synchronization workflow; Git prepared this merge.
Release selection and merge preparation are automated. Review, native smoke tests
and release publication remain pending.

## Checklist

- [ ] I have checked existing nibi issues and PRs for related work.
- [ ] I have followed AGENTS.md and CONTRIBUTING_LOCAL.md.
- [ ] Public text and attached evidence contain no private recordings, settings,
      dictionary contents, credentials, or machine-specific details.
"""


def report(result: dict, directory: Path) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    # Do not leave an old release's PR draft in a later no-update/error report.
    (directory / "pull-request.md").unlink(missing_ok=True)
    (directory / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    lines = [
        "# Handy Local upstream sync",
        "",
        result["message"],
        "",
        f"Status: **{result['status']}**",
    ]
    if result.get("tag"):
        lines += [
            f"Official release: [{result['tag']}](https://github.com/{UPSTREAM}/releases/tag/{result['tag']})"
        ]
    if result.get("branch") and result["status"] in ("ready", "existing"):
        lines += [
            f"Branch: `{result['branch']}`",
            f"[Review / submit a PR on GitHub]({result['compare_url']})",
            "",
            "Review the prepared description and testing checklist before submitting. No PR or app release was published.",
        ]
        (directory / "pull-request.md").write_text(pr_body(result))
    for key in ("conflicts", "errors"):
        if result.get(key):
            lines += [
                "",
                f"{key.title()}:",
                "",
                *[f"- `{item}`" for item in result[key]],
            ]
    summary = "\n".join(lines) + "\n"
    (directory / "summary.md").write_text(summary)
    print(summary)
    print(f"Report: {directory}")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as handle:
            handle.write(summary)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as handle:
            for key in ("status", "commit"):
                handle.write(f"{key}={result.get(key, '')}\n")


def synchronize(root: Path, args: argparse.Namespace) -> dict:
    state = json.loads((root / STATE).read_text())
    if state["repository"] != UPSTREAM or not re.fullmatch(
        r"[0-9a-f]{40}", state["commit"]
    ):
        raise ValueError("Review the recorded upstream repository/commit")
    if run(
        "git",
        "merge-base",
        "--is-ancestor",
        state["commit"],
        "HEAD",
        cwd=root,
        ok=(0, 1),
    ).returncode:
        raise RuntimeError(
            "Recorded upstream commit is not integrated in this checkout"
        )
    endpoint = f"repos/{UPSTREAM}/releases/latest"
    if args.tag:
        version(args.tag)
        endpoint = f"repos/{UPSTREAM}/releases/tags/{args.tag}"
    release = json.loads(run("gh", "api", endpoint, cwd=root).stdout)
    tag = validate_release(release, state)
    git(
        root,
        "fetch",
        "--no-tags",
        f"https://github.com/{UPSTREAM}.git",
        f"refs/tags/{tag}",
    )
    commit = git(root, "rev-parse", "FETCH_HEAD^{commit}")
    result = {"tag": tag, "upstream_commit": commit}
    if tag == state["release"]:
        if commit != state["commit"]:
            raise RuntimeError(
                "The recorded release tag moved. Manual investigation required"
            )
        return {
            **result,
            "status": "current",
            "message": f"Already integrated official stable release {tag}. Unreleased main commits are ignored.",
        }
    if run(
        "git",
        "merge-base",
        "--is-ancestor",
        state["commit"],
        commit,
        cwd=root,
        ok=(0, 1),
    ).returncode:
        raise RuntimeError(
            "Release history diverged from the recorded baseline; review before merging"
        )
    if not args.prepare:
        return {
            **result,
            "status": "available",
            "message": f"Official stable release {tag} is available. Run bun run upstream:prepare to prepare a merge.",
        }
    branch = f"codex/upstream-{tag}"
    title = f"chore: sync official Handy {tag}"
    query = urlencode({"expand": "1", "title": title})
    result.update(
        branch=branch,
        compare_url=f"https://github.com/{FORK}/compare/{BASE_BRANCH}...{branch}?{query}",
    )
    if args.publish:
        origin = git(root, "remote", "get-url", "--push", "origin")
        if origin not in (
            f"https://github.com/{FORK}.git",
            f"https://github.com/{FORK}",
            f"git@github.com:{FORK}.git",
        ):
            raise RuntimeError("Refusing to publish outside the Handy Local fork")
        remote_refs = git(
            root, "ls-remote", "--heads", "origin", f"refs/heads/{BASE_BRANCH}"
        ).split()
        if not remote_refs:
            raise RuntimeError(f"The fork's {BASE_BRANCH} base branch was not found")
        remote_base = remote_refs[0]
        if remote_base != git(root, "rev-parse", "HEAD"):
            raise RuntimeError(
                f"Checkout must match the current remote {BASE_BRANCH} branch before publishing"
            )
    remote = git(root, "ls-remote", "--heads", "origin", f"refs/heads/{branch}")
    local = git(root, "rev-parse", "--verify", f"refs/heads/{branch}", ok=(0, 128))
    if remote or local:
        if remote:
            git(root, "fetch", "--no-tags", "origin", f"refs/heads/{branch}")
            existing = git(root, "rev-parse", "FETCH_HEAD^{commit}")
        else:
            existing = local
        existing_state = json.loads(git(root, "show", f"{existing}:{STATE}"))
        if (
            existing_state.get("release") != tag
            or existing_state.get("commit") != commit
        ):
            raise RuntimeError(
                "Existing sync branch has a different baseline; inspect it manually (not overwritten)"
            )
        if run(
            "git", "merge-base", "--is-ancestor", "HEAD", existing, cwd=root, ok=(0, 1)
        ).returncode:
            raise RuntimeError(
                f"Existing sync branch is behind the current base. Merge {BASE_BRANCH} into it and revalidate"
            )
        if args.publish and not remote:
            git(
                root,
                "push",
                f"--force-with-lease=refs/heads/{branch}:",
                "origin",
                f"{existing}:refs/heads/{branch}",
            )
        return {
            **result,
            "status": "existing",
            "commit": existing,
            "message": "A sync branch already exists; it was not overwritten. Review its current changes and validation results.",
        }
    return {**result, **prepare(root, state, tag, commit, branch, args.publish)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--prepare", action="store_true", help="Prepare a separate local sync branch"
    )
    parser.add_argument(
        "--publish",
        action="store_true",
        help="Also publish the new branch to this fork (requires --prepare)",
    )
    parser.add_argument(
        "--tag", help="Select an explicit published stable release instead of latest"
    )
    parser.add_argument(
        "--report-dir",
        type=Path,
        help="Directory for summary, result JSON and PR draft",
    )
    args = parser.parse_args()
    if args.publish and not args.prepare:
        parser.error("--publish requires --prepare")
    root = Path(git(Path.cwd(), "rev-parse", "--show-toplevel"))
    directory = args.report_dir or Path(
        git(root, "rev-parse", "--git-path", "upstream-sync")
    )
    if not directory.is_absolute():
        directory = root / directory
    try:
        result = synchronize(root, args)
    except (RuntimeError, ValueError, OSError, KeyError) as error:
        result = {"status": "error", "message": str(error)}
    report(result, directory)
    return 1 if result["status"] in ("error", "conflict", "guard-failed") else 0


if __name__ == "__main__":
    sys.exit(main())
