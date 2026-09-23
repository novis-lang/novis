- **A full diagnostic band's max-plus-one belongs to another phase, and `brief.py`'s "next free"
  line prints it anyway.** The types band has filled twice — at `E0499` and at `E0799` — and each
  time the printed next number (`E0500`, `E0800`) read as another phase's band or as none, so a new
  band was opened instead. Do not take the printed number when the band's last code says it is full;
  opening a band is a project-level decision recorded in `docs/adr/README.md` § *Decisions taken at
  project start*, and the band table atop `crates/nvs-diagnostics/src/lib.rs` is the current state.
  [until: reviewed 2026-09-06]
