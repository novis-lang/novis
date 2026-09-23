- **Enabling a path leaves `#[expect(dead_code)]` on more functions than its doc lists, and clippy's
  `note:` names the one you already fixed.** The leftover sits on a helper of a helper, and
  `unfulfilled_lint_expectations` prints its reason, so the error appears to be about the function
  whose attribute was just removed. Run `grep -n 'expect(' <the module>` for every reason naming the
  path before the build. [until: reviewed 2026-09-06]
