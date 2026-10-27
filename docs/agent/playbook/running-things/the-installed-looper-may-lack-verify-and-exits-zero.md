- **The installed `looper` program can lack `verify` and `check`, and a backgrounded `looper verify
  2>&1 | tail` then reports exit 0 having run nothing.** The pipe's last command sets the status, so
  the notification says "completed" over an `unrecognized subcommand` line. Run the gate as `bun nv
  affected --run`, or the kit's copy as `bun tools/looper/main.ts verify`, and read the summary line
  before trusting the exit code. See `tools/looper/main.ts`.
  `until: gone tools/looper/main.ts:verify`
