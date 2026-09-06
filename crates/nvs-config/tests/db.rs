//! `rule:security/db-pool-reset-is-a-boundary`'s pool bounds and § 11's `slow_query` threshold, as the boot reads them: one key in
//! two shapes, bounds that are finite with nothing configured (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`), the values that
//! parse and still cannot describe a pool, and a threshold that is off until an operator writes one.
//!
//! The refusals are asserted by **counting**, not by reading one off a line: a reader that grew a
//! hole in one of its checks still answers plausibly for every other.

use std::collections::BTreeMap;
use std::time::Duration;

use nvs_config::Config;
use nvs_config::db::{PoolBounds, pool_for, slow_query_for, validate};
use nvs_config::tree::Database;
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// The block every case below writes its `pool` under.
const MAIN: &str = "[db.main]\ndriver = \"pgsql\"\n";

/// The `[db.main]` block `text` writes, panicking with the refusal's message when it does not parse.
fn block(text: &str) -> Database {
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text);
    let config = parsed.unwrap_or_else(|err| panic!("{text}\n-- refused: {}", err.message));
    config
        .db
        .get("main")
        .cloned()
        .unwrap_or_else(|| panic!("{text}\n-- has no `[db.main]` block"))
}

/// The bounds `text`'s block resolves to, panicking when it is refused instead.
fn bounds(text: &str) -> PoolBounds {
    pool_for("main", &block(text), &BTreeMap::new())
        .unwrap_or_else(|err| panic!("{text}\n-- refused: {}", err.message))
}

/// § 11's threshold `text`'s block resolves to, panicking when it is refused instead.
fn threshold(text: &str) -> Option<Duration> {
    slow_query_for("main", &block(text), &BTreeMap::new())
        .unwrap_or_else(|err| panic!("{text}\n-- refused: {}", err.message))
}

/// The refusal `text`'s block produces, panicking when it is accepted instead.
fn refusal(text: &str) -> Diagnostic {
    pool_for("main", &block(text), &BTreeMap::new())
        .err()
        .unwrap_or_else(|| panic!("{text}\n-- was accepted, and should not have been"))
}

/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`, which § 13 writes its example against: a block that configures no pool at all still has
/// a finite bound on every side, and they are the ADR's own numbers.
#[test]
fn nothing_configured_is_a_finite_pool() {
    let pool = bounds(MAIN);

    assert!(pool.enabled);
    assert_eq!(pool.max, 16);
    assert_eq!(pool.idle, 2);
    assert_eq!(pool.lifetime, Duration::from_secs(30 * 60));
    assert_eq!(pool.acquire, Duration::from_secs(5));
    assert_eq!(pool, PoolBounds::DEFAULT, "and that is the default set");
}

/// § 13 writes `pool = false` and `[db.<name>.pool]` against the same key, so the tree holds one
/// field of two shapes. Both are asserted here because either alone reads as a whole feature.
#[test]
fn the_switch_and_the_table_are_one_key_in_two_shapes() {
    let off = bounds(&format!("{MAIN}pool = false\n"));
    let table = bounds(&format!(
        "{MAIN}[db.main.pool]\nmax = 4\nidle = 1\nlifetime = \"10m\"\nacquire = \"1s\"\n"
    ));

    assert!(!off.enabled, "`pool = false` restores connect-per-request");
    assert_eq!(
        off,
        PoolBounds::OFF,
        "and leaves the bounds finite, so a reader that consults one anyway is not divided by zero",
    );
    assert!(table.enabled);
    assert_eq!(table.max, 4);
    assert_eq!(table.idle, 1);
    assert_eq!(table.lifetime, Duration::from_secs(600));
    assert_eq!(table.acquire, Duration::from_secs(1));
}

/// `pool = true` is the default written down and not a meaning of its own — the case that fails if
/// the switch is ever read as "the table is absent, so nothing is configured" the other way round.
#[test]
fn the_switch_written_on_changes_nothing() {
    assert_eq!(bounds(&format!("{MAIN}pool = true\n")), PoolBounds::DEFAULT);
}

/// A partly-written block is an independent decision per bound, not one: `rule:config/later-wins-and-every-override-is-recorded` records an override
/// per key, so a `max` written alone must leave every other bound at the default.
#[test]
fn a_written_bound_replaces_only_itself() {
    let pool = bounds(&format!("{MAIN}[db.main.pool]\nmax = 64\n"));

    assert_eq!(pool.max, 64);
    assert_eq!(pool.idle, PoolBounds::DEFAULT.idle);
    assert_eq!(pool.lifetime, PoolBounds::DEFAULT.lifetime);
    assert_eq!(pool.acquire, PoolBounds::DEFAULT.acquire);
}

/// A duration is `crate::value`'s parse and not this module's, asserted as **agreement** across
/// every spelling of one length: a reader that grew its own suffix table fails here while each
/// spelling still looks right on its own line.
#[test]
fn the_spellings_of_one_duration_agree() {
    let written =
        |lifetime: &str| bounds(&format!("{MAIN}[db.main.pool]\nlifetime = {lifetime}\n"));

    assert_eq!(written("\"30m\"").lifetime, Duration::from_secs(1800));
    assert_eq!(written("\"1800s\"").lifetime, written("\"30m\"").lifetime);
    assert_eq!(
        written("1800").lifetime,
        written("\"30m\"").lifetime,
        "a bare number is seconds, which is what the unit's own table says",
    );
}

/// The values that parse and still cannot describe a pool, plus the one that is not a duration at
/// all. Counted rather than read off a line, and every one of them is `E0601`.
#[test]
fn a_bound_that_cannot_describe_a_pool_is_refused_at_boot() {
    let cases = [
        ("max = 0", "[db.main.pool]\nmax = 0\n"),
        ("idle above max", "[db.main.pool]\nmax = 2\nidle = 3\n"),
        ("lifetime = 0", "[db.main.pool]\nlifetime = 0\n"),
        ("lifetime = false", "[db.main.pool]\nlifetime = false\n"),
        ("acquire = false", "[db.main.pool]\nacquire = false\n"),
        (
            "a duration nothing spells",
            "[db.main.pool]\nlifetime = \"30 minutes\"\n",
        ),
    ];

    let accepted: Vec<&str> = cases
        .iter()
        .filter(|(_, pool)| {
            pool_for("main", &block(&format!("{MAIN}{pool}")), &BTreeMap::new()).is_ok()
        })
        .map(|(label, _)| *label)
        .collect();

    assert!(
        accepted.is_empty(),
        "§ 13's bounds are finite and usable, and these were accepted: {accepted:?}",
    );
    for (label, pool) in cases {
        assert_eq!(
            refusal(&format!("{MAIN}{pool}")).code,
            Some(code::E_BAD_DIRECTIVE),
            "a bad directive is a bad directive however it arrived: {label}",
        );
    }
}

/// The refusal names the key and offers the one spelling that is meant, which is the difference
/// between a boot an operator can fix and one they can only revert.
#[test]
fn a_zero_max_is_refused_by_the_key_and_points_at_the_switch() {
    let diagnostic = refusal(&format!("{MAIN}[db.main.pool]\nmax = 0\n"));

    assert!(
        diagnostic.message.contains("db.main.pool.max"),
        "the message should name the key: {:?}",
        diagnostic.message,
    );
    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.starts_with("help: ") && note.contains("pool = false")),
        "and the help should name § 13's one spelling of off: {:?}",
        diagnostic.notes,
    );
}

/// What the hand-written `Deserialize` on `tree::Pool` is for: a typo inside the table is still
/// `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s unknown-key refusal, naming the key. `#[serde(untagged)]` would report *data did
/// not match any variant* here and name nothing.
#[test]
fn an_unknown_key_in_the_pool_table_is_refused_by_name() {
    let mut sources = SourceMap::new();
    let text = format!("{MAIN}[db.main.pool]\nmaximum = 4\n");
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", &text);

    let diagnostic = parsed.expect_err("a key no pool has is not a key an operator may write");
    assert_eq!(diagnostic.code, Some(code::E_BAD_DIRECTIVE));
    assert!(
        diagnostic.message.contains("maximum"),
        "the refusal should name the key that does not exist: {:?}",
        diagnostic.message,
    );
}

/// `rule:observability/a-slow-query-is-logged-past-a-threshold`'s threshold, on both sides of the boundary that matters: unwritten is off, which is
/// the ADR's own default and the reason a deployment gets no slow-query log it did not ask for, and
/// a written `0` is a threshold every statement passes rather than a second spelling of off.
///
/// The spellings are asserted as **agreement** for [`the_spellings_of_one_duration_agree`]'s reason:
/// this reader is `crate::value`'s parse and not a suffix table of its own.
#[test]
fn a_slow_query_threshold_is_off_until_a_block_writes_one() {
    let written = |slow: &str| threshold(&format!("{MAIN}slow_query = {slow}\n"));

    assert_eq!(threshold(MAIN), None, "unwritten is off, and off is silent");
    assert_eq!(written("\"2s\""), Some(Duration::from_secs(2)));
    assert_eq!(written("\"2000ms\""), written("\"2s\""));
    assert_eq!(
        written("2"),
        written("\"2s\""),
        "a bare number is seconds here too",
    );
    assert_eq!(
        written("0"),
        Some(Duration::ZERO),
        "a written zero logs every statement, which is what `slower than nothing` means",
    );
}

/// A `slow_query` nothing can read is a refusal at boot rather than a statement path that quietly
/// times nothing — and it arrives through `validate`, which is the call the boot actually makes, so
/// this fails if the reader is written and never wired up.
///
/// `false` is refused for the same reason `lifetime = false` is: it removes a ceiling everywhere
/// else in this tree, and a threshold is not a ceiling. The help says so in the threshold's own
/// words, not the pool's.
#[test]
fn a_slow_query_that_is_not_a_duration_is_refused_at_boot() {
    for slow in ["false", "\"200 milliseconds\""] {
        let text = format!("{MAIN}slow_query = {slow}\n");
        let mut sources = SourceMap::new();
        let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", &text);
        let config = parsed.unwrap_or_else(|err| panic!("{text}\n-- refused: {}", err.message));

        let diagnostic = validate(&config, &BTreeMap::new())
            .expect_err("a threshold nothing can read is not a boot that may continue");
        assert_eq!(diagnostic.code, Some(code::E_BAD_DIRECTIVE), "{slow}");
        assert!(
            diagnostic.message.contains("db.main.slow_query"),
            "the message should name the key: {:?}",
            diagnostic.message,
        );
    }

    let diagnostic = slow_query_for(
        "main",
        &block(&format!("{MAIN}slow_query = false\n")),
        &BTreeMap::new(),
    )
    .expect_err("and the reader refuses it on its own too");
    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.starts_with("help: ") && note.contains("leave the key out")),
        "the help names the threshold's own way out and never `pool = false`: {:?}",
        diagnostic.notes,
    );
}
