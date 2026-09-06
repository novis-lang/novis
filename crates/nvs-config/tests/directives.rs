//! The registry's two fields are two fields — `rule:config/reloadability-is-its-own-field` against `rule:config/three-changeability-classes` — plus the lookup rule
//! the module doc states.

use nvs_config::Config;
use nvs_config::directive::{Apply, Class, DIRECTIVES, Directive, lookup};
use nvs_diagnostics::SourceMap;

/// The row governing `key`, or a failure naming the key, so a census assertion reads as the claim
/// it is making rather than as an `unwrap` chain.
fn governing(key: &str) -> &'static Directive {
    lookup(key).unwrap_or_else(|| panic!("no directive governs `{key}`"))
}

/// `rule:config/reloadability-is-its-own-field`: reloadability answers *what applying a change requires* and the changeability
/// class answers *who may set it*. The two are independent, and the failure this pins is a registry
/// that reads one off the other — which typechecks, looks right row by row, and re-creates the very
/// reading ("`System` means read once at boot") that ADR replaced.
#[test]
fn reloadability_is_a_field_of_its_own_and_not_the_changeability_class() {
    let census = |class: Class, apply: Apply| {
        DIRECTIVES
            .iter()
            .filter(|row| row.class == class && row.apply == apply)
            .count()
    };

    // `System` holds both values, so neither field is a function of the other: knowing a directive
    // is `System` says nothing about whether applying it takes a restart.
    assert!(
        census(Class::System, Apply::Reload) > 0,
        "no `System` directive reloads, so `System` is being read as \"boot-only\" \
         (`rule:config/reloadability-is-its-own-field` names `[limits.hard]` and a capability grant as the counter-examples)",
    );
    assert!(
        census(Class::System, Apply::Boot) > 0,
        "no `System` directive is `Boot`, so the registry has lost `rule:config/reloadability-is-its-own-field`'s narrow set",
    );

    // The one direction that *is* determined, and by construction rather than by policy: a
    // `Runtime` or `RuntimeTighten` directive is a value read out of the snapshot, so a new
    // snapshot is all applying it can require.
    for row in DIRECTIVES {
        if row.class.settable_by_a_request() {
            assert_eq!(
                row.apply,
                Apply::Reload,
                "`{}` is {:?} and `Boot`, which `rule:config/reloadability-is-its-own-field` says cannot happen: a directive a \
                 request can set is one the snapshot already holds",
                row.key,
                row.class,
            );
        }
    }

    // `rule:config/reloadability-is-its-own-field`'s own two lists, key by key. `Boot` first — the narrow set.
    for key in ["cache.dir", "control.socket", "server.listen"] {
        assert_eq!(
            governing(key).apply,
            Apply::Boot,
            "`{key}` is one of `rule:config/reloadability-is-its-own-field`'s `Boot` set"
        );
    }
    // Then the ones it names as reloading *despite* being `System`, which is the pairing the field
    // exists to make expressible.
    for key in [
        "limits.hard.memory",
        "extension.sha256",
        "opcache.validate",
        "opcache.revalidate_freq",
        "app.limits.memory",
        "schedule.scope",
        "deferred.max_concurrent",
        "metrics.listen",
        "trace.sample",
        // Its sibling `cache.shared` is `Boot` above; this one bounds a map in the core's own
        // memory (`rule:concurrency/cache-memory-is-charged-to-the-core`), so a new ceiling is read by the next write and re-dials nothing.
        "cache.local.max_size",
    ] {
        let row = governing(key);
        assert_eq!(
            row.class,
            Class::System,
            "`{key}` is `System` (`rule:config/three-changeability-classes`)"
        );
        assert_eq!(
            row.apply,
            Apply::Reload,
            "`{key}` reloads (`rule:config/reloadability-is-its-own-field`)"
        );
    }
}

/// The longest-prefix rule, and the pair it exists for: `[limits]` and `[limits.hard]` spell the
/// same five key names under two different classes (`rule:config/three-changeability-classes`), so a registry keyed on the last
/// segment would answer `Runtime` for a ceiling.
#[test]
fn a_more_specific_row_wins_and_a_prefix_must_end_on_a_dot() {
    assert_eq!(governing("limits.memory").class, Class::Runtime);
    assert_eq!(governing("limits.hard.memory").class, Class::System);
    assert_eq!(governing("mode.ceiling").class, Class::System);
    assert_eq!(governing("mode.default").class, Class::Runtime);
    assert_eq!(
        governing("capabilities.script.spawn").class,
        Class::RuntimeTighten
    );

    assert!(
        lookup("limitshard").is_none(),
        "a prefix that does not end on a dot governs nothing"
    );
    assert!(lookup("nosuchblock.key").is_none());
    assert!(lookup("").is_none());
}

/// Two rows for one key would make [`lookup`] answer by declaration order, which the module doc
/// says it does not do.
#[test]
fn every_row_names_a_distinct_key() {
    for (i, row) in DIRECTIVES.iter().enumerate() {
        assert!(
            !DIRECTIVES[..i].iter().any(|earlier| earlier.key == row.key),
            "`{}` has two rows",
            row.key,
        );
        assert!(
            !row.key.is_empty(),
            "a row with an empty key would govern every key"
        );
    }
}

/// `rule:errors/on-limit`: the tier-1 handler's reserved slice is `System`, "not `Runtime`" — and it is
/// written inside a block whose own row is `Runtime`, so the longest-prefix rule is the only thing
/// holding it there. Losing the row would not fail to compile, would not fail any census above, and
/// would quietly let a script set the size of the safety net it is about to need.
#[test]
fn the_fatal_reserve_is_system_class_inside_a_runtime_block() {
    assert_eq!(governing("limits.memory").class, Class::Runtime);
    assert_eq!(
        governing("limits.fatal_reserve_memory").class,
        Class::System,
        "a request may not set its own reserved slice (`rule:errors/on-limit`)",
    );
    assert!(
        !governing("limits.fatal_reserve_memory")
            .class
            .settable_by_a_request()
    );
}

/// `block()` is what a diagnostic names when it refuses a key (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`), so a row that *is* a
/// block reports the root and a key inside one reports the table it is written in.
#[test]
fn a_key_reports_the_block_it_is_written_in() {
    assert_eq!(governing("limits.hard.memory").block(), "limits");
    assert_eq!(governing("deferred.max_concurrent").block(), "deferred");
    assert_eq!(governing("server.listen").block(), "");
}

/// Every key the header `[block]` accepts, read back out of the refusal `deny_unknown_fields`
/// writes for one it does not (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`). The list is `nvs_config::tree`'s own field set rather
/// than a copy of it, which is the whole point of the case below: a key added to one of those
/// blocks joins this list in the commit that adds it, with no edit here to remember.
fn keys_in(block: &str) -> Vec<String> {
    let text = format!("[{block}]\nnvs_no_such_key = true\n");
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", &text);
    let message = parsed
        .err()
        .unwrap_or_else(|| panic!("`[{block}]` accepted an unknown key"))
        .message;
    let listed = message
        .split_once("expected one of ")
        .unwrap_or_else(|| panic!("`[{block}]` refused without listing its keys: {message:?}"))
        .1;
    let keys: Vec<String> = listed
        .split(", ")
        .map(|name| name.trim_matches('`').to_string())
        .collect();
    assert!(
        !keys.is_empty(),
        "`[{block}]` reported no keys at all: {message:?}",
    );
    keys
}

/// `rule:config/three-changeability-classes` names a response header as its counter-example to `System`, and ADR 0074's policy
/// blocks are what that names: a request may set any of them for itself, because it could already
/// write the header directly. The registry states that as the one `http` row covering the whole
/// block, so the claim holds only through the longest-prefix rule — a later, more specific row
/// under `[http]` would take a key back out of `Runtime` without failing anything else here.
///
/// The keys come from `keys_in`, so this asserts the class **every** key of those blocks resolves
/// to rather than the class of the ones somebody listed.
#[test]
fn every_http_response_directive_is_runtime_class() {
    // ADR 0074 §§ 1-3, the three blocks a *response* reads. `[http.client]` (§ 5) is the outbound
    // half and `[http.errors]` is `rule:errors/compile-failure`'s, so neither is this case's question.
    for block in ["http.headers", "http.cors", "http.cookies"] {
        for key in keys_in(block) {
            let dotted = format!("{block}.{key}");
            let row = governing(&dotted);
            assert_eq!(
                row.class,
                Class::Runtime,
                "`{dotted}` resolves through `{}` to {:?}, and `rule:config/three-changeability-classes` makes a response policy \
                 directive `Runtime`: refusing it would refuse nothing, because the request can \
                 write the header itself",
                row.key,
                row.class,
            );
            assert_eq!(
                row.apply,
                Apply::Reload,
                "`{dotted}` is a value read out of the snapshot, so a new snapshot applies it \
                 (`rule:config/reloadability-is-its-own-field`)",
            );
        }
    }
}
