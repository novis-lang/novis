//! The time classes read their text and their epoch one way
//! (`rule:core-classes/a-time-class-builds-its-own-values`).
//!
//! - A zone offset takes one sign, and digits alone: `'+-5:00'` does not
//!   compile at a `Zone` parameter.
//! - A zone name keeps the database's spelling, however the source wrote it.
//! - The epoch readers floor, so an instant before 1970 gives back the second
//!   and the millisecond `Instant::fromEpoch` built it from.
//! - `Instant::fromIso` reads only the rule's forms: no leap second, no
//!   lower-case `t`, no comma before the fraction and no basic offset.
//! - `Date::fromIso('-0000-01-01')` throws, and `Date::at` names its
//!   parameters `year`, `month` and `day`.
//!
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::{Diagnostics, SourceMap};

/// The messages of every error the front end reports for `source`.
fn errors_of(source: &str) -> Vec<String> {
    let mut map = SourceMap::new();
    let id = map.add("test.nvs", source);
    let src = map.file(id);
    let mut diags = Diagnostics::new();
    let stmts = nvs_syntax::parse_file(src, &mut diags);
    let module = nvs_hir::resolve_file(&stmts, src, &mut diags);
    let mut interner = nvs_types::TypeInterner::new();
    let mut exprs = nvs_types::ExprTypeTable::new();
    let files = [nvs_types::ProgramFile { src, stmts: &stmts }];
    nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
    diags
        .iter()
        .filter(|diag| diag.is_error())
        .map(|diag| diag.message.clone())
        .collect()
}

/// A program that builds a `DateTime` in the zone `zone`, written as text.
fn in_zone(zone: &str) -> String {
    format!(
        r#"<?nvs
var $at = Core\Time\DateTime::at(
    Core\Time\Date::at(2024, 3, 1),
    Core\Time\TimeOfDay::at(12, 0),
    '{zone}',
);
echo $at->format('VV'), "\n";
"#
    )
}

#[test]
fn a_zone_offset_with_two_signs_does_not_compile() {
    for zone in ["+-5:00", "-+5:00", "+05:+3", "+ 5:00"] {
        let errors = errors_of(&in_zone(zone));
        assert!(
            errors
                .iter()
                .any(|message| message.contains("is not a zone offset")),
            "`{zone}` compiled: {errors:?}"
        );
    }
    assert_eq!(output_of(&in_zone("+05:00")), "+05:00\n");
    assert_eq!(output_of(&in_zone("-5:30")), "-05:30\n");
}

#[test]
fn a_zone_name_keeps_the_database_spelling() {
    assert_eq!(output_of(&in_zone("europe/berlin")), "Europe/Berlin\n");
    let source = r#"<?nvs
var $zone = Core\Time\Zone::of('america/NEW_YORK');
echo Core\Time\Instant::fromEpoch(0)->in($zone)->format('VV'), "\n";
"#;
    assert_eq!(output_of(source), "America/New_York\n");
}

#[test]
fn an_instant_before_1970_gives_the_counts_it_was_built_from() {
    let source = r#"<?nvs
var $half = Core\Time\Instant::fromEpoch(-1, {nanos: 500000000});
echo $half->toEpochSeconds(), " ", $half->toEpochMillis(), " ", $half->toEpochMicros(), "\n";

var $tiny = Core\Time\Instant::fromEpoch(-1, {nanos: 999999999});
echo $tiny->toEpochSeconds(), " ", $tiny->toEpochMillis(), " ", $tiny->toEpochMicros(), "\n";

var $after = Core\Time\Instant::fromEpoch(1, {nanos: 500000000});
echo $after->toEpochSeconds(), " ", $after->toEpochMillis(), " ", $after->toEpochMicros(), "\n";

var $millis = $half->toEpochMillis();
var $seconds = $half->toEpochSeconds();
var $rest = ($millis - $seconds * 1000) as uint;
var $again = Core\Time\Instant::fromEpoch($seconds, {nanos: $rest * 1000000});
echo $again->compareTo($half), "\n";
"#;
    assert_eq!(
        output_of(source),
        "-1 -500 -500000\n-1 -1 -1\n1 1500 1500000\n0\n"
    );
}

#[test]
fn an_instant_text_takes_only_the_written_forms() {
    let source = r#"<?nvs
array<string> $texts = [
    "2016-12-31T23:59:60Z",
    "2024-03-01t10:00:00Z",
    "2024-03-01T10:00:00,5Z",
    "2024-03-01T10:00:00z",
    "2024-03-01T10:00:00+0100",
    "2024-03-01T10:00:00+01:00",
    "2024-03-01 10:00:00.5Z",
    "2024-03-01T10:00Z",
];
foreach ($texts as string $text) {
    try {
        echo Core\Time\Instant::fromIso($text)->toIso(), "\n";
    } catch (ParseError $e) {
        echo "ParseError\n";
    }
}
"#;
    assert_eq!(
        output_of(source),
        "ParseError\nParseError\nParseError\nParseError\nParseError\n\
         2024-03-01T09:00:00Z\n2024-03-01T10:00:00.5Z\n2024-03-01T10:00:00Z\n"
    );
}

#[test]
fn year_minus_zero_throws_and_date_at_names_its_parameters() {
    let source = r#"<?nvs
array<string> $texts = ["-0000-01-01", "0000-01-01"];
foreach ($texts as string $text) {
    try {
        echo Core\Time\Date::fromIso($text)->format('yyyy-MM-dd'), "\n";
    } catch (ParseError $e) {
        echo "ParseError\n";
    }
}
echo Core\Time\Date::at(year: 2024, month: 12, day: 24)->format('yyyy-MM-dd'), "\n";
"#;
    assert_eq!(output_of(source), "ParseError\n0000-01-01\n2024-12-24\n");
}
