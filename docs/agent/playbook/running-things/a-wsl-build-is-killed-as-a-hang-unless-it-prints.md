- **A `cargo build` in WSL is killed as a hang after 90 s, and so is one run in the foreground.**
  Its CPU time is in WSL's VM, so the watch sees only output, and `| tail`, `> log` and `| tr -d '\0'`
  all hold that output back. A process started with `setsid nohup` dies when `wsl.exe` exits. Run
  `(while sleep 30; do echo tick; done) &` beside the build in one `bash -lc`, with nothing
  piped after `wsl.exe`. `wsl.exe` also drops each `$` in its arguments, so put the command
  in a script under `.agent-tmp/` and run `MSYS_NO_PATHCONV=1 wsl.exe -d Ubuntu-24.04 --
  bash /mnt/<drive>/<repo>/.agent-tmp/<script>`. See `tools/looper/src/jobs/hang.ts`.
  `until: gone looper.toml:hang_exempt`
