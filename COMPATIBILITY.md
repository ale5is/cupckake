# Compatibility target

The native bundle is pinned to Horizon revision
`4b08a43f1d32c3946f2638850f9d120bc7ff4a26` and horizon-mixins revision
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.

The bundle uses the named hooks `world_lifecycle`, `net_from_server`,
`player_submit`, `input_key`, `command`, and `frame`. CI checks the
bundle against the pinned Horizon main revision and runs the client tests on
Linux and Windows.
