- **A file a build step wrote — `Cargo.lock`, `docs/novis.md` — is dirty after `bun nv session --wrap`,
  and the wrap's sweep will not catch it.** The sweep covers files the *wrap* wrote, while `cargo`
  rewrites the lock during the work and `nv verify`'s reference step regenerates `docs/novis.md`
  from the registry's cards; the only place it shows is the tail's `uncommitted after the wrap`
  line, which reads as someone else's edit. Name `Cargo.lock` in the `## commit:` of the slice that
  touched a manifest, and `docs/novis.md` in the one that adds or rewords a `Core` member's card.
  [until: gone tools/nv/cmd/session.ts:uncommitted after the wrap]
