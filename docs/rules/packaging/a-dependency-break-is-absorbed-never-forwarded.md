A dependency change that would land in the last rows of
`rule:packaging/who-can-see-it-decides-the-release-slot` does not go there directly. Work down this
ladder and stop at the first step that holds; every step above the last is preferable to shipping
the break, and the ladder is not skippable because a lower step looks tidier.

1. **Adapt our call sites.** The default outcome, and usually the whole story.
2. **Absorb it behind an adapter we own.** Diagnostic layout is hand-written precisely so the
   language's UI does not shift when a rendering crate does; widen that pattern rather than
   exporting the change.
3. **Compensate at the boundary** so the Novis-visible behaviour is bit-for-bit what it was — the
   right answer wherever the dependency implements something Novis has *its own* specification for:
   regex semantics, decimal rounding, the SQL type map.
4. **Hold the old version, with an expiry.** Written as `# HOLD (revisit YYYY-MM-DD): reason` on the
   dependency's own line in `[workspace.dependencies]`, or as a dated entry in `deny.toml`'s
   `advisories.ignore` for an advisory that provably does not apply. A hold with no date is a bug in
   the hold, and a hold never covers a live advisory that *does* apply.
5. **Fork or vendor.** We then own its maintenance and security response, it passes `deny.toml` and
   the attribution generator like anything else, and it needs the user's agreement before the commit
   lands.
6. **Replace the dependency**, re-running the admissibility questions for the replacement; a swap
   that quietly admits C into the trusted core is not a swap (`rule:security/no-ffi`).
7. **Ship the break.** Requires all of: the user's explicit agreement, a deprecation cycle where one
   is possible (`rule:packaging/a-forced-break-is-announced-before-it-lands`), a migration note
   naming the old spelling and the new one, and a major release to ride on.

Steps 2 and 3 mean carrying adapters with no feature to their name — simplicity of the
implementation spent for compatibility of the program, deliberately. A dependency that repeatedly
forces the ladder is a design problem, not an update problem; the hold dates are the evidence.
