- **A thread-local whose `Drop` joins threads deadlocks on Windows, and the symptom is a test that
  runs its whole body and then never reports.** A pool reached through a `thread_local!` drops from
  a TLS destructor, which Windows runs under the loader lock — the lock the joined thread needs to
  exit — so `cargo test` sits until killed, and leftover `nvs_host-*.exe` processes say the binary
  hung rather than the build. Read the last line the test body printed before suspecting the code
  under test; detach the handles and let the threads see a shutdown flag.
  [until: reviewed 2026-09-06]
