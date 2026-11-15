# Building and checking

The voice client crate can be tested independently:

```sh
cargo test --workspace
```

On Linux, install the ALSA development headers before building:

```sh
sudo apt-get install libasound2-dev libudev-dev pkg-config
```

The native bundle is checked against a Horizon checkout with the matching
`horizon-mixins` CLI. The CI workflow performs this integration check against
the current upstream main branches and tests the client crate on Linux and
Windows.
