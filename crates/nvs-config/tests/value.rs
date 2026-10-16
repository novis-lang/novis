//! `rule:config/ini-set-is-core-config-set`'s one parser: what a value means as a quantity, and how a ceiling compares.
//!
//! The sweeps here assert by **counting** rather than by reading one row off a line: a parser that
//! answers plausibly for `512M` and drops the suffix on `512K` still passes any single case, and
//! the pair of them is the whole point of holding sizes in bytes. Where a bound is asserted it is
//! asserted on both sides — the last accepted value and the first refused one — because a ceiling
//! check that is off by one prints plausibly against either half alone.

use nvs_config::Setting;
use nvs_config::value::{Invalid, Quantity, Unit, unit_of, within_ceiling};
use nvs_diagnostics::code;

/// `text` as a quantity in `unit`, panicking with the refusal's reason when it is refused.
fn quantity(unit: Unit, text: &str) -> Quantity {
    Quantity::parse("limits.memory", unit, &Setting::Text(text.to_string()))
        .unwrap_or_else(|invalid| panic!("`{text}` was refused: {}", invalid.reason))
}

/// The refusal `value` produces in `unit`, panicking when it is accepted instead.
fn refusal(unit: Unit, value: &Setting) -> Invalid {
    Quantity::parse("limits.memory", unit, value).expect_err("was accepted, and should not be")
}

/// A size is held in bytes, so every suffix in the table reads as the same quantity written a
/// different way. Asserted by counting: one row that dropped its multiplier still equals `1G` on
/// its own line only if the multiplier was right.
// covers: tools:config/values-and-units
#[test]
fn every_size_suffix_names_the_same_binary_multiple() {
    let a_gigabyte = [
        "1G",
        "1g",
        "1GB",
        "1gb",
        "1GiB",
        "1024M",
        "1048576K",
        "1073741824",
        "1073741824B",
    ];
    let read: Vec<Quantity> = a_gigabyte
        .iter()
        .map(|text| quantity(Unit::Bytes, text))
        .collect();
    assert_eq!(
        read.iter()
            .filter(|&&size| size == Quantity::Bytes(1024 * 1024 * 1024))
            .count(),
        a_gigabyte.len(),
        "every spelling of a gigabyte must read as one: {read:?}"
    );
}

/// The same, for durations — and `m` is minutes here where it was mega above, which is the whole
/// reason the parse is directed by the unit rather than by the spelling.
// covers: tools:config/values-and-units
#[test]
fn every_duration_suffix_names_the_same_span_of_nanoseconds() {
    let a_minute = [
        "1m",
        "1min",
        "60s",
        "60",
        "60000ms",
        "60000000us",
        "60000000000ns",
    ];
    let read: Vec<Quantity> = a_minute
        .iter()
        .map(|text| quantity(Unit::Duration, text))
        .collect();
    assert_eq!(
        read.iter()
            .filter(|&&span| span == Quantity::Nanos(60 * 1_000_000_000))
            .count(),
        a_minute.len(),
        "every spelling of a minute must read as one: {read:?}"
    );
    assert_eq!(quantity(Unit::Bytes, "1m"), Quantity::Bytes(1024 * 1024));
}

/// `false` is "no ceiling" in both spellings — the boolean a file writes and the string every
/// `Core\Config::set` writes (§ 5 crosses values as strings in both directions).
// covers: tools:config/limits-and-limits-hard
#[test]
fn false_removes_a_ceiling_however_it_arrives() {
    assert_eq!(
        Quantity::parse("limits.hard.memory", Unit::Bytes, &Setting::Bool(false)),
        Ok(Quantity::Unbounded)
    );
    assert_eq!(quantity(Unit::Bytes, "false"), Quantity::Unbounded);
    assert_eq!(quantity(Unit::Count, "FALSE"), Quantity::Unbounded);
    assert_eq!(
        refusal(Unit::Bytes, &Setting::Bool(true)).unit.noun(),
        "a size",
        "`true` grants where a limit measures, so it is not a quantity in any unit"
    );
}

/// The ceiling check on both sides of its bound, and under a removed ceiling.
// covers: tools:config/limits-and-limits-hard
#[test]
fn a_value_is_within_a_ceiling_up_to_and_including_it() {
    let ceiling = Quantity::Bytes(512 * 1024 * 1024);
    assert!(quantity(Unit::Bytes, "511M").within(ceiling));
    assert!(
        quantity(Unit::Bytes, "512M").within(ceiling),
        "at the bound"
    );
    assert!(!quantity(Unit::Bytes, "513M").within(ceiling), "past it");
    assert!(
        !Quantity::Unbounded.within(ceiling),
        "removing a ceiling under one exceeds it"
    );
    assert!(quantity(Unit::Bytes, "1T").within(Quantity::Unbounded));
    assert!(Quantity::Unbounded.within(Quantity::Unbounded));
}

/// Two quantities in different units have no order, and the check refuses rather than letting the
/// pair through: a ceiling that cannot compare must refuse (module doc, security).
#[test]
fn an_incomparable_pair_exceeds() {
    assert!(!Quantity::Bytes(1).within(Quantity::Nanos(u64::MAX)));
    assert!(!Quantity::Count(0).within(Quantity::Bytes(u64::MAX)));
    assert_eq!(Quantity::Bytes(1).partial_cmp(&Quantity::Nanos(1)), None);
}

/// One limit is written in every block that can bound it and must parse the same in all of them,
/// so `unit_of` is keyed on the last segment — including the bare name `Core\Config::set` uses.
#[test]
fn one_limit_has_one_unit_in_every_block_that_spells_it() {
    let spellings = [
        "memory",
        "limits.memory",
        "limits.hard.memory",
        "app.0.limits.memory",
        "app.2.limits.hard.memory",
        "schedule.1.limits.memory",
    ];
    assert_eq!(
        spellings
            .iter()
            .filter(|key| unit_of(key) == Some(Unit::Bytes))
            .count(),
        spellings.len(),
        "a ceiling that parsed differently from the value it bounds compares two quantities"
    );
    assert_eq!(unit_of("limits.cpu_time"), Some(Unit::Duration));
    assert_eq!(unit_of("limits.wall_time"), Some(Unit::Duration));
    assert_eq!(unit_of("limits.max_tasks"), Some(Unit::Count));
    assert_eq!(unit_of("limits.max_output"), Some(Unit::Bytes));
}

/// A key outside a limits block is not a quantity however it is spelled, so a future block naming
/// something `memory` does not inherit a unit by accident — nor does one called `oldlimits`.
#[test]
fn a_key_outside_a_limits_block_has_no_unit() {
    for key in [
        "cache.memory",
        "oldlimits.memory",
        "limits.hard.unknown",
        "mode.ceiling",
        "server.listen",
    ] {
        assert_eq!(unit_of(key), None, "`{key}` is not a limit");
    }
}

/// `within_ceiling` is the check both paths call, and a key with no unit answers `None` rather than
/// a comparison it has no business making.
#[test]
fn the_ceiling_check_answers_only_for_a_quantity() {
    let value = Setting::Text("256M".to_string());
    let ceiling = Setting::Text("512M".to_string());
    assert_eq!(
        within_ceiling("limits.memory", &value, &ceiling),
        Ok(Some(true))
    );
    assert_eq!(
        within_ceiling("limits.memory", &ceiling, &value),
        Ok(Some(false))
    );
    assert_eq!(within_ceiling("mode.ceiling", &value, &ceiling), Ok(None));
    let refused = within_ceiling(
        "limits.memory",
        &Setting::Text("12 bananas".to_string()),
        &ceiling,
    )
    .expect_err("`12 bananas` is not a size");
    assert_eq!(refused.written, "12 bananas");
}

/// Every shape a value can arrive in that is not a measurement is refused, and the refusal names
/// the unit it wanted rather than serde's "invalid type" — which is why `tree.rs` leaves value
/// checking to this module at all.
#[test]
fn a_value_that_is_not_a_quantity_is_refused_naming_the_unit() {
    let not_sizes = [
        Setting::Text("12 bananas".to_string()),
        Setting::Text(String::new()),
        Setting::Text("M".to_string()),
        Setting::Text("-1".to_string()),
        Setting::Text("1.5G".to_string()),
        Setting::Integer(-1),
        Setting::Float(0.5),
        Setting::Bool(true),
        Setting::List(vec!["512M".to_string()]),
        Setting::Text("18446744073709551616".to_string()),
        Setting::Text("16E".to_string()),
        Setting::Text("18446744073709551615K".to_string()),
    ];
    assert_eq!(
        not_sizes
            .iter()
            .filter(|value| Quantity::parse("limits.memory", Unit::Bytes, value).is_err())
            .count(),
        not_sizes.len(),
        "each of these is a size only if the parser stopped reading early"
    );
    let diagnostic = refusal(Unit::Bytes, &not_sizes[0]).diagnostic(None);
    assert_eq!(diagnostic.code, Some(code::E_BAD_DIRECTIVE));
    assert!(
        diagnostic.message.contains("which is not a size"),
        "{}",
        diagnostic.message
    );
    assert_eq!(
        refusal(Unit::Duration, &Setting::Text("600x".to_string()))
            .unit
            .example(),
        "600s",
        "the help offers a value in the unit the key takes"
    );
}

/// A count takes no suffix and a ratio is a fraction of one, both bounded on the refused side.
// covers: tools:config/values-and-units
#[test]
fn a_count_and_a_ratio_accept_only_their_own_shapes() {
    assert_eq!(quantity(Unit::Count, "64"), Quantity::Count(64));
    assert_eq!(
        Quantity::parse("limits.max_tasks", Unit::Count, &Setting::Integer(64)),
        Ok(Quantity::Count(64))
    );
    assert!(
        Quantity::parse(
            "limits.max_tasks",
            Unit::Count,
            &Setting::Text("64M".to_string())
        )
        .is_err()
    );
    assert_eq!(quantity(Unit::Ratio, "0.01"), Quantity::Ratio(0.01));
    assert_eq!(
        Quantity::parse("trace.sample", Unit::Ratio, &Setting::Float(1.0)),
        Ok(Quantity::Ratio(1.0))
    );
    assert!(Quantity::parse("trace.sample", Unit::Ratio, &Setting::Float(1.5)).is_err());
    assert!(Quantity::parse("trace.sample", Unit::Ratio, &Setting::Float(f64::NAN)).is_err());
}
