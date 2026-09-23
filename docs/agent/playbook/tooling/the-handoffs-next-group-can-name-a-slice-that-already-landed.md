- **The handoff's `## Next group` can name a slice that already landed, and the pack still prints it
  as your item in full.** Stage 5's six recording-manager cases were written in `bcdfb3b4a`, two
  sessions before the handoff that listed them as open, so this session's item was work already on
  disk. The driver's failing acceptance check is the discriminator and it costs one call — it names
  the artefact that is actually missing, so `grep -rl 'fn <that name>('` over the crate before
  starting says which items of the group are left to do. [until: reviewed 2026-09-15]
