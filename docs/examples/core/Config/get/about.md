Gives the value of one setting for this request.

The name is the dotted name a configuration file uses, such as `log.level`. A limit also has a short
name: `memory`, `cpu_time`, `wall_time`, `max_tasks`, `max_output` and `max_regex_steps` each name
one entry of the `[limits]` group. The value is always text, written the way the configuration file
writes it, so a number comes back as `"64"` and a yes-or-no setting as `"true"`. A setting your
program changed with `Core\Config::set` comes back with the new value.

A setting the configuration file does not write gives its built-in default. `memory` gives `"128M"`,
and `max_regex_steps` gives `"1000000"`.

**Good to know:** the result is `null` for a setting that has no default, such as `wall_time`. It is
also `null` for a name that has a group of settings or a list, because neither of those is one value.
