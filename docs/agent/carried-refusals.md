# Carried refusals — the holes an earlier milestone left, and who owns them now

`python tools/holes.py` attributes every refusal site to the goal item that anchors its file, and
[crates/nvs-ir/tests/refusals.rs](../../crates/nvs-ir/tests/refusals.rs) fails on a site nothing claims.
That works while the goal that opened a hole is still running. It stops working the moment the chain
advances: `tools/goal-switch.py` carries the outgoing goal's `[[check]]` blocks forward as the next goal's
floor and its unclosed *items* not at all, so a carried check arrives without the item list that made it
green — and the sites nobody had touched became unattributed twice, once per switch.

[carried-gaps.md](carried-gaps.md) is this file's sibling and the same argument one level up: it holds
the gaps a *shipped feature* still owes, which no tool can find because nothing in the tree is shaped
wrong. A refusal site is here; everything else is there.

**This file is that item list, kept where a goal switch cannot reach it.** `holes.py` reads it alongside
`loop-goal.md` and numbers what it finds from 900, so `python tools/holes.py --item 901` prints the entry
below and no goal's own numbering ever collides with it. Nothing here has to be inherited by hand, and a
goal's `.md` should not restate it.

An entry leaves this file exactly one way: the refusal is closed, in the goal that writes that crate
again. This is not an exemption list — `refusals.rs`'s `ALLOWLIST` is the only one of those, it is empty,
and adding to it is the move that gate forbids outright. Each entry ends with the `[until: ...]` trailer
[tools/playbook.py](../../tools/playbook.py)'s module doc defines, naming the state of the tree that
closes it; `python tools/playbook.py --check` reads it and `--retire` deletes the entry when it holds.

901. **M4's carried lowering refusals — every shape `nvs-ir` type-checks and then refuses.**
    M4's own item list anchored every one, and no goal of the parity program writes `nvs-ir` lowering, so
    none of them can claim one. Goal `m4-refusals` is what draws the list down, each site either lowering
    or becoming a `lower::guarded_by!` naming the diagnostic that refuses its shape where it is written.
    `python tools/holes.py --item 901` prints the live list and is the count that matters, since a line
    number here drifts with every edit above it.

    - `crates/nvs-ir/src/lower/call.rs:786` and `:1117` — an argument list through a `callable` that is
      not plain positional, and a by-reference argument from something other than a bare local or a
      compile-time-known property.
    - `crates/nvs-ir/src/lower/control.rs:744`, `:978` and `:989` — a `switch` label at a representation
      other than the subject's own, a `foreach` key binding outside `rule:types/arrays`'s one stored key type,
      and a `foreach` over a subject that is not an `array<T>`.
    - `crates/nvs-ir/src/lower/convert.rs:595` — a truthy condition over a representation the conversion
      slice does not carry.
    - `crates/nvs-ir/src/lower/exception.rs:32` — `throw` on a representation that is not an object.
    - `crates/nvs-ir/src/lower/expr.rs:1829`, `:4570` and `:4630` — a `match` label at a foreign
      representation, `instanceof` against a subject that cannot hold an object, and `clone` on one.
    - `crates/nvs-ir/src/lower/mod.rs:2435`, `:2886` and `:2985` — an array-element write through a shape
      that is not a bare local, a compile-time-known property or a static property, and the two declared
      type lists that do not yet spell every atom `rule:types/grammar` allows.
    - `crates/nvs-ir/src/lower/stmt.rs:269` and `:1484` — a local declaration shape the control-flow
      slice does not lower, and `unset` on anything but an array element with an explicit subscript.

    **Several are now invariant checks rather than shapes a program can reach** — the messages say so,
    naming the `nvs_types` diagnostic that refuses the shape where it is written (`E0490`, `E0444`,
    `E0723`, `E0700`). That does not take a site off this list: the assertion is still a refusal the
    recognizer counts, and turning one into an unreachable assertion is not the same as removing it.

    **What makes them acceptable standing is the ratchet, not this entry.** `CEILING` in
    `crates/nvs-ir/tests/refusals.rs:72` holds the total and **may never rise**, so a refusal
    added beside a carried one fails the run even though attribution now claims its file. What the ratchet
    does *not* have is a floor: nothing requires the number to fall, which is why the entry names who
    closes it. Each closes the way M4 required — it lowers, or a diagnostic naming its rule refuses it,
    never a panic however well worded — in the first goal that writes `nvs-ir` lowering again, and **no
    live entry on the chain is that goal**: this entry named goal `typed-callable`, which retired
    without closing a site, so per [carried-gaps.md](carried-gaps.md)'s contract the owner is struck and
    the entry stays. What that leaves is a scheduling question, so the module doc indexing these sites —
    `crates/nvs-ir/src/lib.rs` § *Known gaps* — tags them `unowned` and points here for the reason,
    which is what `python tools/owners.py --check --reasons` reads. A goal that finds itself editing a
    file above has taken the wrong slice unless closing one is what it came for. [until: exists crates/nvs-ir/tests/refusals.rs:const CEILING: usize = 0;]
