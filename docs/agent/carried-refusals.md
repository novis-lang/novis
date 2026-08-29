# Carried refusals — the holes an earlier milestone left, and who owns them now

`python tools/holes.py` attributes every refusal site to the goal item that anchors its file, and
[crates/nvs-ir/tests/refusals.rs](../../crates/nvs-ir/tests/refusals.rs) fails on a site nothing claims.
That works while the goal that opened a hole is still running. It stops working the moment the chain
advances: `tools/goal-switch.py` carries the outgoing goal's `[[check]]` blocks forward as the next goal's
floor and its unclosed *items* not at all, so a carried check arrives without the item list that made it
green — and seventeen sites nobody had touched became unattributed twice, once per switch.

**This file is that item list, kept where a goal switch cannot reach it.** `holes.py` reads it alongside
`loop-goal.md` and numbers what it finds from 900, so `python tools/holes.py --item 901` prints the entry
below and no goal's own numbering ever collides with it. Nothing here has to be inherited by hand, and a
goal's `.md` should not restate it.

An entry leaves this file exactly one way: the refusal is closed, in the goal that writes that crate
again. This is not an exemption list — `refusals.rs`'s `ALLOWLIST` is the only one of those, it is empty,
and adding to it is the move that gate forbids outright.

901. **M4's seventeen lowering refusals.**
    `nvs-ir` type-checks each of these shapes and then refuses it. M4's own item list anchored every one,
    and no goal of the parity program writes `nvs-ir` lowering, so none of them can claim one.

    - `crates/nvs-ir/src/lower/call.rs:773` and `:1104` — an argument list through a `callable` that is
      not plain positional, and a by-reference argument from something other than a bare local or a
      compile-time-known property.
    - `crates/nvs-ir/src/lower/control.rs:744`, `:978` and `:989` — a `switch` label at a representation
      other than the subject's own, a `foreach` key binding outside ADR 0007 § 5's one stored key type,
      and a `foreach` over an ADR 0053 `Iterable`/`Iterator` subject.
    - `crates/nvs-ir/src/lower/convert.rs:574` — a truthy condition over a representation the conversion
      slice does not carry.
    - `crates/nvs-ir/src/lower/exception.rs:32` — `throw` on a representation that is not an object.
    - `crates/nvs-ir/src/lower/expr.rs:1670`, `:2562`, `:2751`, `:3814` and `:3850` — a `match` label at a
      foreign representation, an instance call and a static call with no resolved target in the
      typed-expression table, `instanceof` against a subject that cannot hold an object, and `clone` on
      one.
    - `crates/nvs-ir/src/lower/mod.rs:2276`, `:2705` and `:2792` — an array-element write through a shape
      that is not a bare local, a compile-time-known property or a static property, and the two declared
      type lists that do not yet spell every atom ADR 0007 § 3 allows.
    - `crates/nvs-ir/src/lower/stmt.rs:269` and `:1465` — a local declaration shape the control-flow
      slice does not lower, and `unset` on anything but an array element with an explicit subscript.

    **What makes them acceptable standing is the ratchet, not this entry.** `CEILING` in
    `crates/nvs-ir/tests/refusals.rs:66` holds the total at seventeen and **may never rise**, so a refusal
    added beside a carried one fails the run even though attribution now claims its file. Each closes the
    way M4 required — it lowers, or a diagnostic naming its rule refuses it, never a panic however well
    worded — in the first goal that writes `nvs-ir` lowering again. A goal that is not that one has taken
    the wrong slice if it finds itself editing a file above.
