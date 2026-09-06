Five refusals, each a rule of its own.

- **Never runs the input.** No `eval`, no autoload execution, no `composer install`, no bootstrap
  file. Conversion is a read of bytes.
- **Never converts `vendor/` by default.** The tool is scoped to an application's own code
  (`rule:programs/no-compatibility-promise`); a dependency is reported against the package registry —
  as a package that exists, one that does not, or a C extension that needs a Tier 1 `.nvsx`, which the
  converter cannot synthesise and says so.
- **Never invents a name binding.** A name that does not resolve under `rule:programs/autoload` is
  reported, not guessed.
- **Never applies a rewrite that is not in the table**, and never asks a model for one. A model is
  non-deterministic by construction (`rule:tooling/convert-is-deterministic`), cannot produce a rule
  id, a tier or a proof, and fails as a confident wrong rewrite — the exact outcome the tiering exists
  to prevent. A model may help a *human* write a rule for the table, where the differential case
  checks it.
- **Never claims compatibility in its own output.** The header states mode, dialect, digests and the
  tier counts; the forbidden phrasings of `rule:programs/no-compatibility-promise` bind the
  converter's own text as much as any other document.
