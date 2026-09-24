- **`nvs-cli`'s `sd_notify_messages_are_ready_…_then_stopping` can fail with foreign
  `READY=1`/`STOPPING=1` lines ahead of its own**, then pass alone and on the next full run.
  `Notify::install` writes a process-global `OnceLock`, so this case's recorder also collects what a
  served life running beside it in the same binary reports. Re-run `bun nv verify` before
  reading it as yours; fixing it means scoping that global, not editing the case.
  [until: gone crates/nvs-cli/src/service.rs:fn install]
