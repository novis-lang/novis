- **A `cfg(unix)`-only test cannot satisfy a `cargo`-named acceptance check; the driver reads "did
  not run" as a failure.** A world-writable directory exists on Windows in one line, `icacls <dir>
  /grant *S-1-1-0:(OI)(CI)(M)`, which `nvs_config::trust::check` sees through
  `GetEffectiveRightsFromAclW`; spell the principal as the SID because `icacls` is localized. Assert
  on `Untrusted::Breach` and on the directory's last component, not the canonical path, which is
  `\\?\C:\...` on Windows (`a_world_writable_cache_directory_is_refused` is the shape).
  [until: reviewed 2026-09-06]
