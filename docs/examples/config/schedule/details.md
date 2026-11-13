`scope` says whether an entry runs once for all hosts together (`"fleet"`) or once on every host
(`"host"`). An `on` entry needs `scope = "host"` and cannot have a `timezone`. `timezone` sets the
time zone of the `cron` expression. `overlap` says what to do when the last run is still in
progress.

The server waits for a `stop` run, up to `[server] drain_timeout`. A `stop` run does not happen
when the server crashes, is killed or loses power. To learn that a server is down, you still need a
monitor outside it.

`Core\Script::args()` returns a map. It has the entry's `name`, and the event in `on`. A `cron`
run has `"on" => "cron"`. A `reload` run also has `outcome`, which is `"applied"` or `"refused"`.
It also has `restart`, the names of the settings that need a restart. A `failed` run also has
`schedule`, the names of the entries that failed. It also has `dead_letter`, the number of queue
jobs that were moved to the dead-letter table.

Only `nvs serve` runs entries. `nvs run` and `nvs check` do not.
