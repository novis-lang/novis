# The ladder tree — how a program's cost grows with its input

A bench under [`benches/members/`](../members/README.md) runs one input many times, so it shows what
one operation costs and whether each operation leaves something behind for the next. It says nothing
about **input size**, and input size is where quadratic work hides. A ladder runs the same program at
growing sizes and fits how its cost grows. `bun nv scaling` runs every ladder here; the ramp, the
agreement test, the bounds and what each `expect` allows live in its module doc,
[`tools/nv/cmd/scaling.ts`](../../tools/nv/cmd/scaling.ts), and nowhere else.

## What a ladder is

One `.nvs` file under `benches/scaling/<area>/`, where `<area>` is one of the names in `AREAS` in
that module. Its shape is a bench's: a `Bench::run` that does the work, and a closing line,
`echo Bench::run(N)`, that passes the size as one integer literal. The literal is the ladder's
`start`; the tool rewrites it in a copy for every size it runs.

```nvs
<?nvs
// Sorts a list of n prices, as a shop does before it shows the cheapest products first.
// scaling: start 1000
// scaling: max 64000
// scaling: expect nlogn
...
echo Bench::run(1000), "\n";
```

| Line | Says |
|---|---|
| `// scaling: start N` | the first size, and the literal in the closing line |
| `// scaling: max N` | the largest size the ramp may reach; at least three doublings above `start` |
| `// scaling: expect <class>` | the growth the work may have: `constant`, `linear`, `nlogn` or `karatsuba`. `quadratic` is never accepted |
| `// scaling: kind <kind>` | what is measured: `run` (the default), `compile`, `lsp`, `fmt` or `serve` |
| `// scaling: proposal` | the ladder grows past its bound and the fix waits for a decision; it is reported, and does not fail |

Size the work so that **the size is the only thing that grows**: build the input inside `run` from
the size, do the operation once over it, and return something that depends on the result so nothing
is skipped. A ladder that needs Unix sockets declares `// requires: unix`, as a bench does. A folder's
`nvs.toml` applies to every ladder in it, as in the bench tree.

## Running them

```
bun nv scaling                  every ladder, judged against what it declares
bun nv scaling arrays/sort      only the ladders whose path contains this
bun nv scaling --check          the same, with one closing line when every ladder passes
bun nv scaling --areas          fails while an area has no ladder
```
