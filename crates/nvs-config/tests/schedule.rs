//! `rule:config/cron-is-five-fields-and-nothing-more` and `rule:config/a-missed-fire-is-skipped-and-a-dst-edge-fires-once` from the scheduler's side: the expression the boot already read, answering
//! the instant it next fires.
//!
//! The boot's own refusals are in `resolve.rs`, beside the rest of the tree's, because they are
//! questions asked of a *configuration*. What is pinned here is the other half of the same parse —
//! that a `Cron` in hand names instants, and which ones — so a change that keeps every refusal and
//! moves a fire by an hour still fails.

use jiff::Timestamp;
use nvs_config::schedule::{Cron, zone_of};

/// The instant `expression` fires at after `after`, both written as RFC 3339, in the named zone.
fn fires(expression: &str, zone: &str, after: &str) -> Option<String> {
    let zone = zone_of(Some(zone)).expect("the zone is one this build's database knows");
    let after: Timestamp = after.parse().expect("the case names a real instant");
    Cron::parse(expression)
        .expect("the case names an expression in § 2's dialect")
        .next_after(&after.to_zoned(zone))
        .map(|fire| fire.timestamp().to_string())
}

/// The same, for a case that is not about an expression that never comes round.
fn next(expression: &str, zone: &str, after: &str) -> String {
    fires(expression, zone, after).expect("the expression fires inside the horizon")
}

/// § 2: the dialect the boot accepts is the dialect that fires, asserted over a **sweep** rather
/// than one expression — a next-fire that reads the minute field and guesses at the rest answers
/// the first row plausibly and fails here. The last row is the one that sets the horizon: the
/// longest gap between two 29 Februaries is the eight years a non-leap century opens.
#[test]
fn the_next_fire_is_the_expression_the_boot_read() {
    let rows = [
        // Seconds are not part of the dialect, so the fire is the next whole minute that matches.
        ("0 3 * * *", "2026-01-15T02:59:30Z", "2026-01-15T03:00:00Z"),
        // Strictly after: the fire at the instant asked about has already happened.
        ("0 3 * * *", "2026-01-15T03:00:00Z", "2026-01-16T03:00:00Z"),
        (
            "*/15 * * * *",
            "2026-01-15T10:07:00Z",
            "2026-01-15T10:15:00Z",
        ),
        ("@daily", "2026-01-15T05:00:00Z", "2026-01-16T00:00:00Z"),
        ("@yearly", "2026-06-01T00:00:00Z", "2027-01-01T00:00:00Z"),
        (
            "59 23 31 12 *",
            "2026-01-01T00:00:00Z",
            "2026-12-31T23:59:00Z",
        ),
        // A name is part of the dialect, and Sunday is `0`.
        ("0 12 * * 0", "2026-01-15T00:00:00Z", "2026-01-18T12:00:00Z"),
        ("0 0 29 2 *", "2096-03-01T00:00:00Z", "2104-02-29T00:00:00Z"),
    ];

    let answered: Vec<String> = rows
        .iter()
        .map(|(expression, after, _)| next(expression, "UTC", after))
        .collect();

    assert_eq!(
        answered,
        rows.iter()
            .map(|(_, _, fire)| (*fire).to_string())
            .collect::<Vec<String>>(),
        "each expression fires at the instant § 2's dialect names, and at no other",
    );
}

/// § 2, and POSIX's own two-field rule: when `day-of-month` and `day-of-week` **both** narrow, a
/// date matching *either* fires. Asserted with the two single-narrowing spellings beside it, because
/// an implementation that intersected the two fields would answer the third and fourth rows
/// correctly and only differ on the first two.
#[test]
fn a_day_of_month_and_a_day_of_week_that_both_narrow_fire_on_either() {
    // 2026-08-01 is a Saturday, so the first of the month and the first Monday are different days.
    assert_eq!(
        next("0 12 1 * MON", "UTC", "2026-07-31T00:00:00Z"),
        "2026-08-01T12:00:00Z",
        "the first of the month fires although it is not a Monday",
    );
    assert_eq!(
        next("0 12 1 * MON", "UTC", "2026-08-01T12:00:00Z"),
        "2026-08-03T12:00:00Z",
        "and the Monday fires although it is not the first",
    );

    assert_eq!(
        next("0 12 1 * *", "UTC", "2026-08-01T12:00:00Z"),
        "2026-09-01T12:00:00Z",
        "with only the day of the month narrowing, the weekday says nothing",
    );
    assert_eq!(
        next("0 12 * * MON", "UTC", "2026-08-01T12:00:00Z"),
        "2026-08-03T12:00:00Z",
        "and with only the weekday narrowing, the day of the month says nothing",
    );
}

/// § 6: a local-time schedule landing in a spring-forward **gap** fires **once**, at the first valid
/// instant after the gap. `02:30` does not exist on 2026-03-08 in New York — the clocks go from
/// `01:59:59-05` to `03:00:00-04` — so the fire is `03:00`, and the day after is back to `02:30`.
#[test]
fn a_fire_in_a_spring_forward_gap_lands_on_the_first_instant_after_it() {
    assert_eq!(
        next("30 2 * * *", "America/New_York", "2026-03-07T17:00:00Z"),
        "2026-03-08T07:00:00Z",
        "the missing 02:30 fires at 03:00-04, the first instant the gap leaves",
    );
    assert_eq!(
        next("30 2 * * *", "America/New_York", "2026-03-08T07:00:00Z"),
        "2026-03-09T06:30:00Z",
        "and it fires once: the next is the following day's 02:30, not a second run",
    );
}

/// § 6: one landing in a fall-back **repeat** fires **once**, on the first occurrence. `01:30`
/// happens twice on 2026-11-01 in New York, at `05:30Z` under `-04` and again at `06:30Z` under
/// `-05`; the second is not a fire. The third case is the one an implementation gets wrong by
/// reading the civil clock alone — asked from *inside* the repeated hour, the first occurrence is
/// already behind, and answering it would run the job twice.
#[test]
fn a_fire_in_a_fall_back_repeat_happens_on_the_first_occurrence_only() {
    assert_eq!(
        next("30 1 * * *", "America/New_York", "2026-10-31T17:00:00Z"),
        "2026-11-01T05:30:00Z",
        "the repeated 01:30 fires on its first occurrence",
    );
    assert_eq!(
        next("30 1 * * *", "America/New_York", "2026-11-01T05:30:00Z"),
        "2026-11-02T06:30:00Z",
        "and the second occurrence is not a fire",
    );
    assert_eq!(
        next("30 1 * * *", "America/New_York", "2026-11-01T06:00:00Z"),
        "2026-11-02T06:30:00Z",
        "asked from inside the repeat, the answer is still ahead of the clock",
    );
}

/// A date the calendar does not hold is not a fire, and the search says so rather than running out
/// of years quietly: 30 February is a well-formed expression under § 2's ranges — the month has 31
/// days in the dialect — and it comes round never.
#[test]
fn an_expression_that_never_comes_round_answers_nothing() {
    assert_eq!(fires("30 2 30 2 *", "UTC", "2026-01-01T00:00:00Z"), None);
}

/// § 6: `timezone` defaults to `UTC` — there is no ambient zone anywhere in Novis (ADR 0063 § 4) —
/// and a name the database does not carry is a fault rather than a silent fall back to it, which is
/// what [`nvs_config::schedule::validate`] turns into the boot refusal.
#[test]
fn an_absent_timezone_is_utc_and_an_unknown_one_is_a_fault() {
    /// What zone `timezone` names, by the IANA name the database canonicalized it to.
    fn named(timezone: Option<&str>) -> Result<String, String> {
        zone_of(timezone).map(|zone| zone.iana_name().unwrap_or("unnamed").to_string())
    }

    assert_eq!(named(None), Ok("UTC".to_string()));
    assert_eq!(
        named(Some("  ")),
        Ok("UTC".to_string()),
        "an empty key has said nothing, the same reading every other key gets",
    );
    assert_eq!(
        named(Some("Europe/Vienna")),
        Ok("Europe/Vienna".to_string()),
        "and a real zone is the one named, from the database this build carries",
    );
    assert_eq!(
        zone_of(Some("Mars/Olympus")),
        Err("Mars/Olympus".to_string())
    );
}
