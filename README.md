# Simple Voice Chat for Horizon

Client-side proximity and group voice chat for the Horizon Rust client. Speaks the Simple Voice Chat 2.6.x protocol (compatibility version 20), for compatible Java servers.

This repository produces a native Horizon mod package (.hnmod). Install the package through Horizon's Mods interface; Horizon compiles the Rust sources into an isolated game build for the selected instance. It is not a standalone game executable and must not be placed in a Java mods folder.

## Download and install

1. Download `simple_voice_chat-1.0.12.hnmod` from the latest successful [Simple Voice Chat compatibility workflow](https://github.com/ale5is/cupckake/actions/workflows/compatibility.yml) artifact (`simple-voice-chat-hnmod-1.0.12-pr` while testing this pull request).
2. Import/install the `.hnmod` through Horizon's native Mods interface for the desired instance.
3. Accept Horizon's native-code trust warning.
4. Build the instance from Horizon's Mods page.

Native mods execute with the player's account permissions. Review the source before installing.

## Build the package locally

Requires Python 3.11 or newer.

    python tools/package.py

The output is `target/simple_voice_chat-1.0.12.hnmod`. The packager includes the mixin helpers, the complete `svc_client` Rust crate, Opus sources encoded into `opus_sources.rs` (to comply with the .hnmod source-file allowlist), data folders, license notice and matching native metadata. It verifies the resulting archive and prints its SHA-256.

## Compatibility

- Minecraft: 26.3
- Horizon revision: `6475f62f3c834b7489e1816bf47f47ea5eb2b879`
- Horizon Mixins revision: `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`
- Native hooks API: 359

The package is source-only. The launcher performs the native compilation on the user's computer; GitHub Actions packages and validates the .hnmod and checks the mixin against the pinned Horizon source. This is not a complete precompiled Horizon client. The automated checks do not replace an in-game microphone/server test.

## Source and license

Clean-room implementation based on the Simple Voice Chat wire protocol. No Java mod code or assets are included. Protocol credit: henkelmax, https://modrinth.com/mod/simple-voice-chat

MIT licensed; see LICENSE.
