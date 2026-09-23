- **`nvs-cli`'s `sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping` fails under load
  with two extra states in front of the ones it wants, and passes on the next run.** The recorder it
  reads is process-wide, so a sibling test in the same binary that reports `READY=1`/`STOPPING=1`
  lands in its list; `verify.py`'s own failure text names the shape. A `verify.py` run that is red on
  this test alone is re-run rather than diagnosed, and a session that touched no Rust has not caused
  it. [until: gone crates/nvs-cli/src/serve.rs:sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping]
