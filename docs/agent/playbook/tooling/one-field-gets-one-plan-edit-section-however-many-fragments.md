- **One field gets one `## plan-edit:` section, however many fragments it moves in.** A second `##
  plan-edit: Open now` further down the wrap file is not merged with the first and its fragments are
  silently not applied, which looks like the size refusal repeating with the byte count unchanged
  after you added more cuts to buy the space. Repeat `--- old`/`--- new` *inside* the one section;
  if a rejection's numbers do not move after an edit, check that the section it names appears once.
  [until: gone tools/nv/cmd/session.ts:plan-edit]
