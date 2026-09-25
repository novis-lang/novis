- **Every name in a `cargo-named` check has to run on the platform the driver runs `cargo` on, so
  the `#[cfg(unix)]` a permission claim wants fails the check forever.** A `#[cfg(unix)]` test reads
  as green locally and leaves the driver reporting `did not run` on Windows. Take the verdict as a
  parameter, inject a `Breach` everywhere, and assert the platform's own spelling on the other side
  (a mode on Unix, `nvs_config::trust::exposure` on Windows). [until: gone crates/nvs-config/src/trust.rs:Breach]
