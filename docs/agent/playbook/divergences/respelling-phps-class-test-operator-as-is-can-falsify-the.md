- **Respelling PHP's class-test operator as `is` can falsify the sentence around it, not just the
  word in it.** `crates/nvs-stdlib/src/debug.rs` and `docs/rules/errors/debug-dump.md` both argued a
  bound from "a program cannot act on that union: a class test against a `Core` class is `E0496`",
  which `is` makes untrue — `Core\Cli\Text|Core\Html\Markup $r = …; if ($r is Core\Html\Markup)`
  compiles and
  narrows. Run the two lines through `target/debug/nvs.exe run` before swapping the word, and rewrite
  the claim in the rule as well as in the comment. [until: reviewed 2026-09-18]
