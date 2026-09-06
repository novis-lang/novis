An operator wanting deterministic control instead of a probability gets two explicit commands.
`nvs cache gc` is the same walk `rule:packaging/eviction-rides-the-cold-miss-at-a-probability` performs
on a miss, without the roll: it deletes oldest-first down to the floor if the store is over its cap, and
otherwise does nothing. `nvs cache clear` empties the store.

Neither is ever needed for correctness. A content-addressed entry cannot be stale — a changed source, a
changed toolchain or a changed extension set is a different key — so the only thing either command
changes is disk usage, and the next run of anything they removed pays one cold compile.
