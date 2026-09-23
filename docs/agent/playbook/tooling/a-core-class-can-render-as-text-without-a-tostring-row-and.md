- **A `Core` class can render as text without a `toString` row, and the second rule is
  `nvs_runtime::is_carrier`.** `Core\Cli\Text` and `Core\Html\Markup` are sink carriers, so
  `rule:security/capture-answers-the-carrier` renders them as the bytes they hold —
  `value_to_string`'s `Tag::Object` arm does it, asking for no member — and a refusal written off
  the registry's member rosters alone turns `Core\Out::capture` cases red at the full verify.
  `nvs_stdlib::registry::class_renders` joins the two rosters; a rule stated over `registry.rs`'s
  rows is not the whole rule where `nvs_runtime` answers for a class itself.
  [until: reviewed 2026-09-06]
