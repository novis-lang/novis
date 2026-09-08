# Handoff

## State

**Goal 18, stages 1 to 4 are complete.** `Core\Arr::shapeAs<T>`, `Core\Request::queryAs<T>` and
`Core\Request::postAs<T>` each have a registry row, a reference card, a body and an `address()` arm,
and `crates/nvs-stdlib/src/registry.rs:2389`'s `WRITTEN_CLASS_MEMBERS` opens the type-argument door
for all five spellings on it.

**The two request members are one walk and one bag.** Each is `crate::json::hydrate` under
`crate::json::Reading::Values` over the array `query` and `post` already index into, so the field
list, the presence column, the dotted paths and the one collected `ParseError` are stage 3's.
`shaped_subject` in `crates/nvs-stdlib/src/request.rs` answers the `{name?: string}` bag both share:
with no name the subject is the whole parsed set, with one it is the subtree that name reaches under
`crate::uri::place`'s bracket convention, and a name the set never carried reaches `null`, which is
not a set of fields and is refused as one. `rule:core-api/shape-rules` R15 is why that is one row
rather than two. **Neither member opens a public `post(): array<mixed>`** — the whole set is
reachable only through a declared shape.

**`form_of`, `multipart_form` and `urlencoded_form` now carry the caller's member name**, so a
`postAs` refusal names `postAs`. It is `&'static str` because `claim_body` keeps it; the playbook's
*Writing Novis itself* owns that.

Spec § 15's roster and `docs/reference/core/Request.md` list both members. The failing acceptance
check (`examples/input-shapes.nvs`) is stage 5's fixture: an unwritten artefact, not a regression.

## Next group

**Stage 5: the proofs** — one file set: `examples/`, `docs/reference/core/Arr.md`,
`tests/conformance/reject/`. loop-goal.md § *Stage 5* is the specification, and the first item is the
acceptance check that has been failing since the goal opened.

- [ ] **`examples/input-shapes.nvs`, with the request fixture beside it** — the acceptance check names
      the `.nvs` file. `examples/request-fields.nvs:1` is the shape of a request-reading example and
      `examples/json-body.nvsr:1` is the `.nvsr` fixture that feeds one, since a program calling
      `queryAs`/`postAs` throws `LogicError` with no request under it. It should write all three
      members: the general `Core\Arr::shapeAs` and the two wrappers.
- [ ] **`Core\Arr::shapeAs`'s paragraph on `docs/reference/core/Arr.md:6`** — that page is
      hand-written prose rather than a render of the cards (playbook, *Tooling*), and it still
      describes the class without the member. `docs/reference/core/Request.md:20` is the paragraph
      this session wrote for the other two, and the shape to follow.
- [ ] **The three `--EXPECTF-ERROR--` cases stage 2 owes** — an unqualified shape at a tainted call
      site names the shape, a missing required key names the key, and a shape whose `tainted`
      promises nothing is refused. `rule:security/tainted-qualifier` specifies them and
      `tests/conformance/reject/a-core-io-path-refuses-a-tainted-argument.nvst:28` is the section
      shape, whose indentation widens with the line number.

## Backlog

- Spec §§ 2 and 6 still do not list `shapeAs` or `decodeAs`; § 15 got its two this session —
  loop-goal.md § *Standing decisions*, and `crates/nvs-stdlib/src/arr.rs:703`'s comment goes with it.
- `Reading::Values` over a *nested* class field is threaded but has no case; an inline shape nested
  in a shape still erases to `CodecTy::Opaque` — `crates/nvs-stdlib/src/json.rs` § *Known gaps*.
- `crates/nvs-stdlib/src/json.rs:1425`'s refusal reads "decodes from a JSON object" under
  `Reading::Values`, where the subject is a form rather than a document. Wording only, nothing pins it.
