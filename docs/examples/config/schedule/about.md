Runs a script file at fixed times, or when something happens to the server.

Each `[[schedule]]` entry in `nvs.toml` has a `name`, a `script` file, and either `cron` or `on`.
`cron` is a time expression. `on` is one of four server events:

- `"start"` runs once, after the server starts to accept requests.
- `"stop"` runs when the server begins to stop. It does not run when the server crashes or is killed.
- `"reload"` runs after the server reloads its configuration, or fails to.
- `"failed"` runs when scheduled scripts or queue jobs fail, at most once a minute.

The script runs isolated, like a request. It reads its entry's `name` and its `on` value from
`Core\Script::args()`.

**In plain words:** this is cron for your application, written in the same file as the other
settings.

A program cannot add an entry and cannot read the list. The server checks every entry when it
starts. An entry that can never run stops the start with an error.

**Good to know:** the scheduled file is a normal program, so `nvs run jobs/report.nvs` runs it by
hand.
