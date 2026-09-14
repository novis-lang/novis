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
`loop-goal.md` and numbers what it finds from 900, so the first entry here is `python tools/holes.py
--item 901` and no goal's own numbering ever collides with it. Nothing here has to be inherited by hand,
and a goal's `.md` should not restate it.

**It holds no entry today**, which is the state `crates/nvs-ir/tests/refusals.rs`'s `CEILING` keeps: at
zero, a refusal site added anywhere in `nvs-ir` or `nvs-codegen` is a red test in the slice that writes
it, so no site can reach a goal switch to be carried in the first place. An entry is owed again only if
that ratchet is ever raised.

An entry leaves this file exactly one way: the refusal is closed, in the goal that writes that crate
again. This is not an exemption list — `refusals.rs`'s `ALLOWLIST` is the only one of those, it is empty,
and adding to it is the move that gate forbids outright. Each entry ends with the `[until: ...]` trailer
[tools/playbook.py](../../tools/playbook.py)'s module doc defines, naming the state of the tree that
closes it; `python tools/playbook.py --check` reads it and `--retire` deletes the entry when it holds.
