# Building and checking

## Create the Horizon native mod package

Requires Python 3.11 or newer.

    python tools/package.py

This creates target/simple-voice-chat-1.0.1.hnmod. The deterministic archive contains horizon-native.json, all Rust/TOML sources under the mixin bundle (including the nested svc_client crate), applicable data directories and the license notice. The script verifies required crate files and reads the archive back before reporting its SHA-256.

## Validate and compile

The GitHub Actions workflow installs the Horizon-native validator and runs:

    cargo horizon-mod check target/simple-voice-chat-1.0.1.hnmod

The launcher compiles the source package for the selected Horizon instance. The repository workflow does not compile or distribute a separate full Horizon client.

On Linux, standalone client crate tests may require ALSA development headers:

    sudo apt-get install libasound2-dev libudev-dev pkg-config
    cargo test --workspace --locked