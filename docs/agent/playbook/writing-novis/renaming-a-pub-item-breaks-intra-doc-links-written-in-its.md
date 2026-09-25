- **Renaming a `pub` item breaks intra-doc links written in its neighbours, and the only step that
  says so is `nv verify`'s last one.** A sibling field's doc comment saying "exactly as
  [`Self::query`] does" builds, tests, clippies and formats cleanly and then fails `cargo doc` with
  `-D rustdoc::broken_intra_doc_links` — a whole verify run spent on a four-character edit. Before
  renaming, `grep -n "Self::<oldname>\|\[\`<oldname>\`\]"` over the crate; an intra-doc link is
  invisible to every other tool in the gate. [until: gone tools/nv/cmd/verify.ts:rustdoc]
