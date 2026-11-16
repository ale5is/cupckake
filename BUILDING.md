# Building and checking

## Create the Horizon native mod package

Requires Python 3.11 or newer.

    python tools/package.py

This creates `target/simple_voice_chat-1.0.6.hnmod`. The deterministic archive contains `horizon-native.json`, the Rust/TOML mixin sources, the nested `svc_client` crate, applicable data directories and the license notice. Opus C sources are hex-encoded into `opus_sources.rs` because the native package format only accepts its source-file allowlist. The script verifies required crate files and reads the archive back before reporting its SHA-256.

## Validate and compile

The packaging workflow installs the Horizon-native validator from the pinned Horizon revision and runs:

    cargo horizon-mod check target/simple_voice_chat-1.0.6.hnmod

The separate compatibility workflow checks the mixin against the pinned Horizon and Horizon Mixins sources and runs the `svc_client` unit tests on Windows and Linux. The launcher compiles the source package for the selected Horizon instance. The repository workflow does not compile or distribute a separate full Horizon client.

Automated checks are not a substitute for testing the mod in Horizon with a real Simple Voice Chat server, microphone and speakers.

On Linux, standalone client crate tests may require ALSA development headers:

    sudo apt-get install libasound2-dev libudev-dev pkg-config
    cargo test --workspace --locked
