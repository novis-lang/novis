- **A doc comment that *teaches* the `rule:` citation shape is itself a citation, and a made-up id
  turns `bun nv rules --check` red.** `crates/nvs-cli/src/agent.rs`'s citation stripper
  spelled its example as the token plus a topic and a name, so the floor reported a dangling rule in
  a file whose every other citation is real. `tools/nv/cmd/rules.ts`'s `rules-py:examples` marker is the other
  way out and is whole-file, which would stop checking those too — write the token with no topic and
  no name, since the regex needs both, and keep the marker for a file that is only about the format.
  [until: gone tools/nv/cmd/rules.ts:rules-py:examples]
