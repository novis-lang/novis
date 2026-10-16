Runs a script file at fixed times, without a request.

Each `[[schedule]]` entry in `nvs.toml` has a `name`, a `cron` time expression and a `script` file.
The server runs that file at those times. The script is isolated in the same way as a request and
shares nothing with other work. `scope` says whether the entry runs once for all hosts together or
once on every host. `timezone` sets the time zone of the `cron` expression. `overlap` says what to
do when the last run is still in progress.

**In plain words:** this is cron for your application, written in the same file as the other
settings.

A program cannot add an entry and cannot read the list. There is no class and no attribute for it.
The server checks every entry when it starts. An entry that can never run stops the start with an
error.

**Good to know:** the scheduled file is a normal program, so `nvs run jobs/report.nvs` runs it by
hand.
