//! An in-place result is the result the build path gives, and a cut frees
//! what it no longer needs (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! Three programs, each run with in-place changes on and off:
//!
//! - a cut after `mapKeys` or `flip` appends at the key the built array
//!   appends at;
//! - many short slices of long strings stay under a memory limit, because a
//!   short cut is copied out and its long subject freed;
//! - a queue loop of `first`, `withoutFirst` and `append` reads the same
//!   number of keys per round at two sizes, so it grows linearly.
//!
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;

/// Runs `source` with in-place changes on and off, checks that both print
/// the same text, and gives it.
fn both_ways(source: &str) -> String {
    let in_place = output_of_program(&common::lower(source));
    let copying = output_of_program(&nvs_ir::lower::without_in_place(|| common::lower(source)));
    assert_eq!(
        in_place, copying,
        "the in-place run and the built run differ:\n{source}"
    );
    in_place
}

/// `mapKeys` and `flip` give an array whose keys are `0`, `1`, `2`. A build
/// of those keys is a list, so after `withoutLast` the next append writes
/// `2`. The in-place result must append there too.
#[test]
fn a_cut_after_new_keys_appends_where_the_built_array_does() {
    let source = r#"<?nvs
final class Show {
    public static function keys(array<int> $a): string {
        return Core\Str::join(Core\Arr::keys($a), ",");
    }

    public static function names(array<string> $a): string {
        return Core\Str::join(Core\Arr::keys($a), ",");
    }
}

array<int> $a = [];
$a[] = 10;
$a[] = 20;
$a[] = 30;
$a = Core\Arr::mapKeys($a, fn(int $v, string $k): string => $k);
$a = Core\Arr::withoutLast($a);
$a[] = 40;
echo Show::keys($a), "\n";

array<int> $b = [];
$b[] = 10;
$b[] = 20;
$b[] = 30;
$b = Core\Arr::mapKeys($b, fn(int $v, string $k): string => $k);
$b = Core\Arr::withoutLast($b);
$b = Core\Arr::append($b, 40);
echo Show::keys($b), "\n";

array<int> $n = [];
$n[] = 0;
$n[] = 1;
$n[] = 2;
array<string> $f = Core\Arr::flip($n);
$f = Core\Arr::withoutLast($f);
$f[] = "z";
echo Show::names($f), "\n";
"#;
    assert_eq!(both_ways(source), "0,1,2\n0,1,2\n0,1,2\n");
}

/// 400 slices of 8 bytes, each taken from its own string of 1 MiB, are
/// kept in a list. Each kept slice must free its subject, so the program
/// ends far under a ceiling that 400 subjects would pass many times over.
#[test]
fn a_short_slice_frees_its_long_subject() {
    /// The memory limit: 32 MiB, under what 400 subjects of 1 MiB hold.
    const CEILING: usize = 32 << 20;
    let source = r#"<?nvs
array<string> $kept = [];
for (int $i = 0; $i < 400; $i++) {
    string $big = Core\Str::repeat("x", 1048576);
    $big = Core\Str::slice($big, 0, 8);
    $kept[] = $big;
}
echo Core\Arr::count($kept), " ", Core\Str::length($kept[399]), "\n";
"#;
    for program in [
        common::lower(source),
        nvs_ir::lower::without_in_place(|| common::lower(source)),
    ] {
        let unit = nvs_codegen::compile(&program).expect("the fixture compiles");
        let mut ctx = Ctx::buffered();
        unit.install_in(&mut ctx);
        ctx.set_memory_limit(CEILING);
        let entry = unit.script().expect("the script frame was compiled");
        let status = entry.call(&mut ctx);
        let pending = ctx.take_pending();
        assert!(
            status.is_ok(),
            "the slices did not fit under the limit: {pending:?}"
        );
        let output = ctx.take_buffered_output().expect("a buffered context");
        assert_eq!(String::from_utf8_lossy(&output), "400 8\n");
    }
}

/// How many keys one round of the queue loop reads, at a queue of `size`
/// entries turned `size` times.
#[cfg(debug_assertions)]
fn parses_per_round(size: usize) -> usize {
    let source = format!(
        r#"<?nvs
array<int> $q = [];
for (int $i = 0; $i < {size}; $i++) {{
    $q[] = $i;
}}
for (int $round = 0; $round < {size}; $round++) {{
    int $x = Core\Arr::first($q) ?? 0;
    $q = Core\Arr::withoutFirst($q);
    $q = Core\Arr::append($q, $x);
}}
echo Core\Arr::count($q), "\n";
"#
    );
    let program = common::lower(&source);
    let before = nvs_runtime::array::key_parses();
    let output = output_of_program(&program);
    let parses = nvs_runtime::array::key_parses() - before;
    assert_eq!(output, format!("{size}\n"));
    parses / size
}

/// The queue loop reads a few keys per round, the same at both sizes. A
/// loop that walked the queue's keys each round would read about as many
/// keys per round as the queue has entries.
#[cfg(debug_assertions)]
#[test]
fn a_queue_loop_reads_the_same_keys_per_round_at_every_size() {
    let small = parses_per_round(500);
    let large = parses_per_round(2000);
    assert!(
        large <= small + 2 && large < 50,
        "a round reads {small} keys at 500 entries and {large} at 2000"
    );
}
