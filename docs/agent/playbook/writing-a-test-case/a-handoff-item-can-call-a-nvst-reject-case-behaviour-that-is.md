- **A handoff item can call a `.nvst` reject case "behaviour that is already landed" when the refusal
  it pins does not exist yet.** Stage 4's `a-catch-arm-naming-the-finish-marker-is-refused.nvst` needed
  a diagnostic no crate had written, and `catch (Core\Script\Finished $e)` compiled and ran instead.
  One `target/debug/nvs.exe run` over a three-line probe in `.agent-tmp/` settles it before any case is
  written — the binary is already built at the commit the session opens on, so the probe costs one call
  and no build. [until: reviewed 2026-09-12]
