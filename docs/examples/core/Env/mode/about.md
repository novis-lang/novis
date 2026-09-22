Returns which mode the program runs in: `Core\Env\Mode::Production` or `Core\Env\Mode::Development`.

The mode is set in `nvs.toml`, with `default` in the `[mode]` section. No environment variable
changes it, so a setting such as `APP_ENV` has no effect. When nothing is configured, the mode is
`Production`. A deployment is then the careful one until somebody chooses otherwise.

Use it to decide how much your program shows when something goes wrong, or whether it loads test
data. Each mode also changes the default of a few settings, and each of those settings can still be
set on its own.
