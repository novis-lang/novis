- **The installed `looper` program has no `decision` subcommand, so `looper decision ask` fails with
  `unrecognized subcommand`.** The verb is in the project's synced copy alone. Run `bun
  tools/looper/main.ts decision ask <question> --option <text>… --recommend <n> --built-on <n>`, and
  keep each `--option` under 200 bytes or the call is refused. See `tools/looper/src/loop/decisions.ts`.
  `until: gone tools/looper/src/loop/decisions.ts:takes at most ${max} bytes`
