# CI checks

The workflow runs the standalone client tests on Linux and Windows, then checks
the native mixin against the pinned Horizon and horizon-mixins revisions.

Linux runners need the ALSA and udev development headers plus pkg-config.
The vendored Opus build uses C99 variable arrays on GCC/Clang and _alloca
on MSVC, which does not implement C99 variable-length arrays.
