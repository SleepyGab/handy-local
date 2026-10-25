#!/usr/bin/env python3
"""Keep local macOS builds on a certificate identity instead of per-build ad-hoc hashes."""

import json
import os
import re
import subprocess
import sys


def select_identity(output: str, requested: str | None, allow_adhoc: bool) -> str:
    identities = re.findall(r'^\s*\d+\)\s+([0-9A-Fa-f]{40})\s+"([^"]+)"\s*$', output, re.M)
    if requested == "-":
        if allow_adhoc:
            return "-"
        raise ValueError("Ad-hoc signing changes permission identity on every build. Set HANDY_ALLOW_ADHOC_SIGNING=1 only for a disposable build.")
    if requested:
        matches = [fingerprint for fingerprint, name in identities if requested in (fingerprint, name)]
        if len(matches) == 1:
            return matches[0]
        raise ValueError("The requested signing identity is missing or ambiguous in the keychain.")
    if len(identities) == 1:
        return identities[0][0]
    if not identities and allow_adhoc:
        return "-"
    raise ValueError("Select a stable certificate with HANDY_MACOS_SIGNING_IDENTITY (name or SHA-1 fingerprint). Run security find-identity -v -p codesigning to list available identities. See docs/INSTALL.en.md.")


def main() -> int:
    command = sys.argv[1:]
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        raise ValueError("Expected a Tauri build command after --")
    build_env = os.environ.copy()
    if sys.platform == "darwin":
        found = subprocess.run(["/usr/bin/security", "find-identity", "-v", "-p", "codesigning"], check=True, capture_output=True, text=True)
        requested = os.environ.get("HANDY_MACOS_SIGNING_IDENTITY") or os.environ.get("APPLE_SIGNING_IDENTITY")
        identity = select_identity(found.stdout, requested, os.environ.get("HANDY_ALLOW_ADHOC_SIGNING") == "1")
        if "--no-sign" in command:
            raise ValueError("Local macOS builds must be signed; --no-sign would break the permission identity.")
        # Override the upstream config's explicit '-' without modifying upstream
        # defaults or storing a developer's certificate/private key in the repo.
        command += ["--config", json.dumps({"bundle": {"macOS": {"signingIdentity": identity}}})]
        build_env["APPLE_SIGNING_IDENTITY"] = identity
        print("Handy Local signing: " + ("ad-hoc (permissions may need reauthorization)" if identity == "-" else identity), flush=True)
    return subprocess.run(command, check=False, env=build_env).returncode


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, subprocess.CalledProcessError) as error:
        print(f"Handy Local build: {error}", file=sys.stderr)
        sys.exit(1)
