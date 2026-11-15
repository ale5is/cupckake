#!/usr/bin/env python3
import hashlib
import json
import sys
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "mixins" / "simple-voice-chat"
TARGET = ROOT / "target"
GAME = "26.3"


def main():
    compatibility = json.loads((ROOT / "compatibility.json").read_text(encoding="utf-8"))
    mixin_path = BUNDLE / "horizon-mixin.toml"
    mixin = tomllib.loads(mixin_path.read_text(encoding="utf-8"))
    mod = mixin["mod"]
    mod_id = mod["id"]
    version = mod["version"]

    if BUNDLE.name != mod_id:
        raise SystemExit(f"Bundle folder must be named {mod_id}")
    if mixin.get("compatibility", {}).get("horizon_revision") != compatibility["horizon_revision"]:
        raise SystemExit("horizon-mixin.toml and compatibility.json pin different Horizon revisions")
    if compatibility["game"] != GAME:
        raise SystemExit(f"This bundle targets Minecraft {GAME}, not {compatibility['game']}")
    if not (BUNDLE / "runtime.rs").is_file():
        raise SystemExit("Missing runtime.rs")
    for crate in mixin.get("crate", []):
        crate_root = BUNDLE / crate["path"]
        if not (crate_root / "Cargo.toml").is_file():
            raise SystemExit(f"Missing Cargo.toml for native crate: {crate['path']}")

    manifest = {
        "schema": 1,
        "id": mod_id,
        "version": version,
        "game": GAME,
        "horizon": compatibility["horizon_revision"],
        "mixins": compatibility["horizon_mixins_revision"],
        "hooks": compatibility["hooks"],
    }
    files = {
        "horizon-native.json": (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()
    }
    data_dirs = [Path("data")] + [Path(c["path"]) / "data" for c in mixin.get("crate", [])]
    for path in sorted(BUNDLE.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(BUNDLE)
        include = path.suffix in (".rs", ".toml")
        include = include or any(relative.is_relative_to(folder) for folder in data_dirs)
        if include:
            files[f"mixins/{mod_id}/{relative.as_posix()}"] = path.read_bytes()

    license_path = ROOT / "LICENSE"
    if license_path.is_file():
        files[f"mixins/{mod_id}/LICENSE"] = license_path.read_bytes()

    required = [
        f"mixins/{mod_id}/horizon-mixin.toml",
        f"mixins/{mod_id}/runtime.rs",
        f"mixins/{mod_id}/crates/svc_client/Cargo.toml",
    ]
    missing = [name for name in required if name not in files]
    if missing:
        raise SystemExit("Package is missing required sources: " + ", ".join(missing))

    TARGET.mkdir(parents=True, exist_ok=True)
    output = TARGET / f"{mod_id}-{version}.hnmod"
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            info.create_system = 3
            archive.writestr(info, content)

    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    with zipfile.ZipFile(output) as archive:
        names = set(archive.namelist())
        if not set(required).issubset(names) or "horizon-native.json" not in names:
            raise SystemExit("Archive verification failed")
        embedded = json.loads(archive.read("horizon-native.json"))
        if embedded != manifest:
            raise SystemExit("Embedded horizon-native.json does not match compatibility metadata")

    print(json.dumps({
        "file": str(output),
        "size": output.stat().st_size,
        "sha256": digest,
        "entries": len(files),
        "id": mod_id,
        "version": version,
        "game": GAME,
        "format": ".hnmod",
    }, indent=2))


if __name__ == "__main__":
    main()
