A `require` whose path is a string literal resolves, and the resolved real path is then compared component-wise
against the path as written. A component that differs **only** in case is
`E_REQUIRE_PATH_CASE_MISMATCH`. So `require 'mailer.nvs';` against a file named `Mailer.nvs` is an
error on Windows and macOS, where it would otherwise have compiled, and it was already
`E_REQUIRE_TARGET_NOT_FOUND` on Linux. The invariant is what matters: **a mis-cased path never compiles
clean on any OS.** Autoload roots are held to the same comparison.

Three properties keep the check honest:

- **It costs no syscalls.** `canonicalize` on Windows and macOS already returns the entry's true
  on-disk spelling, so this is a string compare of two paths the resolver is already holding. On a
  case-sensitive filesystem the comparison can only pass, because a mis-cased path never resolved.
- **It reports only a pure case difference.** The literal's components are replayed onto the requiring
  file's canonical directory — `.` skipped, `..` popped — to line up positionally with the resolved
  path. If a symlink was crossed or the path was absolute, that alignment is gone and the check is
  skipped rather than guessed at.
- **It never reports the entry file's own spelling.** Only components the `require` literal itself
  wrote are compared; how the entry file was named on the command line is not this diagnostic's
  business.
