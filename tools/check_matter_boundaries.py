"""Check Matter dependency and namespace boundaries."""

from __future__ import annotations

import sys
from pathlib import Path


FORBIDDEN_CARGO_TERMS = {
    "rusty-xr",
    "rusty_xr",
    "rusty-dope",
    "rusty_dope",
    "makepad",
    "openxr",
    "vulkan",
    "android",
    "quest",
}

FORBIDDEN_DEFAULT_NAMESPACES = {
    "rusty.xr.",
    "debug.rustyxr.",
    "/rustyxr/v1/",
}

SCAN_EXTENSIONS = {".rs", ".toml", ".json"}


def main() -> int:
    repo = Path(__file__).resolve().parents[1]
    failures: list[str] = []

    for cargo_toml in repo.rglob("Cargo.toml"):
        text = cargo_toml.read_text(encoding="utf-8")
        lower_text = text.lower()
        for term in FORBIDDEN_CARGO_TERMS:
            if term in lower_text:
                failures.append(f"{cargo_toml}: forbidden cargo boundary term {term!r}")

    for path in list(repo.joinpath("crates").rglob("*")) + list(repo.joinpath("schemas").rglob("*")):
        if not path.is_file() or path.suffix.lower() not in SCAN_EXTENSIONS:
            continue
        text = path.read_text(encoding="utf-8")
        for term in FORBIDDEN_DEFAULT_NAMESPACES:
            if term in text:
                failures.append(f"{path}: forbidden default namespace {term!r}")

    if failures:
        for failure in failures:
            print(f"[FAIL] {failure}")
        return 1

    print("[PASS] Matter dependency and namespace boundaries")
    return 0


if __name__ == "__main__":
    sys.exit(main())
