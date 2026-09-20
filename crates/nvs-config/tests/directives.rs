//! The registry's two fields are two fields — `rule:config/reloadability-is-its-own-field` against `rule:config/three-changeability-classes` — plus the lookup rule
//! the module doc states.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use nvs_config::Config;
use nvs_config::directive::{Apply, Class, DIRECTIVES, Directive, lookup};
use nvs_config::value::{Unit, unit_of};
use nvs_diagnostics::{Code, SourceMap, code};

/// The row governing `key`, or a failure naming the key, so a census assertion reads as the claim
/// it is making rather than as an `unwrap` chain.
fn governing(key: &str) -> &'static Directive {
    lookup(key).unwrap_or_else(|| panic!("no directive governs `{key}`"))
}

/// `rule:config/reloadability-is-its-own-field`: reloadability answers *what applying a change requires* and the changeability
/// class answers *who may set it*. The two are independent, and the failure this pins is a registry
/// that reads one off the other — which typechecks, looks right row by row, and re-creates the very
/// reading ("`System` means read once at boot") that ADR refuses.
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

    // `rule:config/reloadability-is-its-own-field`'s own lists, key by key. `Boot` first — the narrow set.
    for key in [
        "opcache.file_cache_dir",
        "control.socket",
        "server.listen",
        "queue.connection",
        "queue.workers",
    ] {
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
        // `[queue]`'s other half, whose `connection` and `workers` are in the `Boot` list above:
        // both of these are read per job out of the snapshot, so applying one re-creates nothing.
        "queue.max_attempts",
        "queue.visibility",
        "metrics.listen",
        "trace.sample",
        // Its sibling `cache.shared` is `Boot` above; this one bounds a map in the core's own
        // memory (`rule:concurrency/cache-memory-is-charged-to-the-core`), so a new ceiling is read by the next write and re-dials nothing.
        "cache.local.max_size",
        // The third tier's two, which are this class for the same reason doubled: what they bound
        // is one map for the whole process rather than one per core
        // (`rule:concurrency/the-process-tier-is-one-store-per-process`).
        "cache.process.max_size",
        "cache.process.fill_wait",
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
/// same key names under two different classes (`rule:config/three-changeability-classes`), so a registry keyed on the last
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
/// writes for one it does not (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`), in whichever of serde's three shapes that
/// block's width gives it. The list is `nvs_config::tree`'s own field set rather
/// than a copy of it, which is the whole point of the case below: a key added to one of those
/// blocks joins this list in the commit that adds it, with no edit here to remember.
fn keys_in(block: &str) -> Vec<String> {
    keys_listed(&format!("[{block}]\nnvs_no_such_key = true\n"))
}

/// The keys `text`'s refusal lists, for a block whose header is not `[name]` — an array of tables
/// is written `[[app]]`, and a `[app]` in its place is refused for being the wrong *shape* rather
/// than for the unknown key, which lists nothing.
fn keys_listed(text: &str) -> Vec<String> {
    let mut sources = SourceMap::new();
    let header = text.lines().next().unwrap_or_default();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text);
    let message = parsed
        .err()
        .unwrap_or_else(|| panic!("`{header}` accepted an unknown key"))
        .message;
    let listed = message
        .split_once("expected ")
        .unwrap_or_else(|| panic!("`{header}` refused without listing its keys: {message:?}"))
        .1;
    // Every backtick-quoted name in the list, rather than a split on one separator: serde writes
    // ``a` or `b`` for a two-key block and reaches "one of `a`, `b`, `c`" only at three, so a
    // separator scan answers about whichever form the block happens to have today.
    let keys: Vec<String> = listed
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    assert!(
        !keys.is_empty(),
        "`{header}` reported no keys at all: {message:?}",
    );
    keys
}

/// `[[app]]` is the roster of applications, so every key of a block is `System` and applying a
/// change to one is a `Reload`.
///
/// Both halves are the security content rather than bookkeeping. A block states what one
/// application may do, so a request able to set any key of one would be choosing its own grants —
/// and, since the roster is an array, another application's by index. The apply class is `Reload`
/// because a block is folded onto the global tree by `Snapshot::build` while the snapshot is being
/// built, so a changed block reaches the next request by that fold running again and re-creates
/// nothing.
///
/// Asked of every key `[[app]]` accepts and of the nested spellings beneath them, because the
/// claim holds only through the module doc's longest-prefix rule: a row added under `app`, or a
/// `limits` row somehow reached from `app.0.limits.memory`, would take a key back out of `System`
/// with every other assertion in this file still passing.
// covers: directive:app
#[test]
fn every_key_of_an_app_block_is_system_class_applied_at_reload() {
    let keys = keys_listed("[[app]]\nnvs_no_such_key = true\n");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        [
            "root",
            "entry",
            "mode",
            "origin",
            "limits",
            "capabilities",
            "log"
        ],
        "a key added to `[[app]]` joins this census in the commit that adds it, because the one \
         `app` row is what governs it and nothing else names it",
    );

    let mut asked = vec!["app".to_string(), "app.0".to_string()];
    for key in &keys {
        asked.push(format!("app.{key}"));
        asked.push(format!("app.0.{key}"));
    }
    asked.push("app.0.limits.hard.memory".to_string());
    asked.push("app.0.capabilities.fs.read".to_string());
    asked.push("app.0.capabilities.script.spawn".to_string());

    for key in asked {
        let row = governing(&key);
        assert_eq!(
            row.key, "app",
            "`{key}` resolves through `{}` rather than through the one `app` row, so what a \
             block may say is no longer one class",
            row.key,
        );
        assert_eq!(
            row.class,
            Class::System,
            "`{key}` is part of what an operator granted one application, and a request that \
             could set it would be choosing its own capabilities (`rule:config/three-changeability-classes`)",
        );
        assert!(
            !row.class.settable_by_a_request(),
            "`Core\\Config::set(\"{key}\", …)` has to refuse: the class says who may set a \
             directive, and this is the one class that answers nobody",
        );
        assert_eq!(
            row.apply,
            Apply::Reload,
            "`{key}` is folded onto the tree while a snapshot is built, so a new snapshot applies \
             it and nothing is re-created (`rule:config/reloadability-is-its-own-field`)",
        );
    }
}

/// `rule:config/three-changeability-classes`: the grants are the one row a request may write at
/// all, and only ever downwards.
///
/// `RuntimeTighten` because `rule:security/isolate-shares-nothing` gives the block a direction
/// rather than a ceiling — a script may drop a right it holds and can never add one it does not —
/// so the class is neither `System`, which answers nobody, nor `Runtime`, which answers "freely, up
/// to what the operator kept" for a value that has no quantity to be under. `Reload` because a
/// changed grant is folded onto the tree while the next snapshot is built and re-creates nothing.
///
/// Asked of every grant the block accepts rather than of the ones somebody listed, and of the
/// families as well as their keys, because one row governs all of them through the longest-prefix
/// rule: a row added under `capabilities` would take a whole family back out of the class with
/// every other assertion in this file still passing.
///
/// The last assertion is the boundary this block shares with `[[app]]`. The same grant written
/// inside an application's block is the `app` row's and therefore `System`, because a request able
/// to write `app.0.capabilities.fs.read` would be choosing an application's grants rather than
/// narrowing its own — two questions that look like one key.
// covers: directive:capabilities
#[test]
fn every_grant_is_the_one_tightening_row_and_the_same_grant_under_an_app_block_is_not() {
    let families = keys_in("capabilities");
    assert_eq!(
        families.iter().map(String::as_str).collect::<Vec<_>>(),
        [
            "script", "fs", "net", "tls", "process", "debug", "db", "mail", "cache", "queue"
        ],
        "`[capabilities]` accepts {families:?}: a family added to the block joins this census in \
         the commit that adds it, because which grants exist is not the registry's question and \
         this is the only place that notices",
    );

    let mut asked = vec!["capabilities".to_string()];
    for family in &families {
        asked.push(format!("capabilities.{family}"));
        for grant in keys_in(&format!("capabilities.{family}")) {
            asked.push(format!("capabilities.{family}.{grant}"));
        }
    }

    for key in &asked {
        let row = governing(key);
        assert_eq!(
            row.key, "capabilities",
            "`{key}` resolves through `{}` rather than through the one grants row, so what a \
             script may give up is no longer one class",
            row.key,
        );
        assert_eq!(
            row.class,
            Class::RuntimeTighten,
            "`{key}` is a right a script may drop and never add \
             (`rule:security/isolate-shares-nothing`), which is the one class that says so",
        );
        assert!(
            row.class.settable_by_a_request(),
            "`{key}` is writable by a request in principle — the direction is what bounds it, and \
             a class answering nobody would be `System` and a different decision",
        );
        assert_eq!(
            row.apply,
            Apply::Reload,
            "`{key}` is folded onto the tree while a snapshot is built, so a new snapshot applies \
             it and nothing is re-created (`rule:config/reloadability-is-its-own-field`)",
        );
    }

    assert!(
        !DIRECTIVES
            .iter()
            .any(|row| row.key.starts_with("capabilities.")),
        "a row under `capabilities` would govern one family by longest prefix, and the sweep above \
         would go on passing for every other one",
    );

    for key in [
        "app.0.capabilities.fs.read",
        "app.capabilities.script.spawn",
    ] {
        let row = governing(key);
        assert_eq!(
            row.key, "app",
            "`{key}` is part of what an operator wrote for one application, so it resolves \
             through `{}` rather than through the grants row",
            row.key,
        );
        assert_eq!(
            row.class,
            Class::System,
            "`{key}` answers who may grant rather than who may give one up, and a request able to \
             write it would be choosing an application's capabilities by index",
        );
    }
}

/// `rule:concurrency/cache-memory-is-charged-to-the-core`: the per-core tier's ceiling is the
/// operator's, and a new one costs nothing to apply.
///
/// `System` because the memory the key bounds belongs to the core and not to any one request, so a
/// request that raised it would be spending what every other request on that core then goes
/// without. `Reload` because the ceiling is read by the next write and enforced by forgetting
/// entries, which re-dials nothing and re-creates nothing.
///
/// Asserted beside `cache.shared`, which is `Boot`, because that is what the row costs: three
/// tiers written as three rows rather than as one `cache` row is the only way the block holds two
/// apply classes at once, and a blanket row added above them would pass every other assertion here
/// while making an operator restart for a ceiling change.
// covers: directive:cache.local
#[test]
fn the_local_tiers_ceiling_is_one_system_key_applied_at_reload_beside_a_boot_sibling() {
    let keys = keys_in("cache.local");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["max_size"],
        "`[cache.local]` accepts {keys:?}, and the row covers whatever the block holds: a second \
         key joins this census in the commit that adds it",
    );

    let row = governing("cache.local.max_size");
    assert_eq!(
        row.key, "cache.local",
        "`cache.local.max_size` resolves through `{}` rather than through the tier's own row",
        row.key,
    );
    assert_eq!(
        row.class,
        Class::System,
        "the bytes this key bounds are the core's, so a request raising it spends what every \
         other request on that core then goes without (`rule:concurrency/cache-memory-is-charged-to-the-core`)",
    );
    assert!(
        !row.class.settable_by_a_request(),
        "`Core\\Config::set(\"cache.local.max_size\", …)` has to refuse, which is what `System` \
         means and the whole of what it means",
    );
    assert_eq!(
        row.apply,
        Apply::Reload,
        "a new ceiling is read by the next write and enforced by forgetting entries \
         (`rule:config/reloadability-is-its-own-field`)",
    );
    assert_eq!(
        governing("cache.shared.url").apply,
        Apply::Boot,
        "the coherent tier is dialled once per core, so moving it re-dials every connection — and \
         `[cache]` holding both apply classes is why these are per-tier rows",
    );
}

/// `rule:concurrency/the-process-tier-is-one-store-per-process`: both keys of the middle tier are
/// the operator's, and both are read by the next caller rather than at boot.
///
/// `max_size` is the row above's argument doubled — the map it bounds is held once for the whole
/// process, so a request raising it spends what every request on the box then goes without — and
/// `fill_wait` is how long a request on one core may be held waiting on a fetch another core
/// started, which is not a bound its beneficiary may choose either.
///
/// The last assertion is the one that would fail first if the block were ever stated as a single
/// `cache` row: there is none, so a tier nobody wrote a row for is ungoverned rather than
/// silently inheriting a class. The registry states rules, not the list of legal keys —
/// `deny_unknown_fields` is what refuses a fourth tier.
// covers: directive:cache.process
#[test]
fn both_process_tier_keys_are_system_at_reload_and_no_blanket_cache_row_answers_for_them() {
    let keys = keys_in("cache.process");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["max_size", "fill_wait"],
        "`[cache.process]` accepts {keys:?}: a key added to the block joins this census in the \
         commit that adds it, because the one row is what governs all of them",
    );

    for key in ["cache.process.max_size", "cache.process.fill_wait"] {
        let row = governing(key);
        assert_eq!(
            row.key, "cache.process",
            "`{key}` resolves through `{}` rather than through the tier's own row",
            row.key,
        );
        assert_eq!(
            row.class,
            Class::System,
            "`{key}` bounds what one process spends on behalf of every request on it, which is \
             not a request's call (`rule:concurrency/the-process-tier-is-one-store-per-process`)",
        );
        assert!(
            !row.class.settable_by_a_request(),
            "`Core\\Config::set(\"{key}\", …)` has to refuse: a request that shortened the wait \
             would be deciding for the callers queued behind the one fill",
        );
        assert_eq!(
            row.apply,
            Apply::Reload,
            "a new ceiling is read by the next write and a new wait by the next fill, and neither \
             re-creates the map — it exists before the workers do \
             (`rule:config/reloadability-is-its-own-field`)",
        );
    }

    assert_eq!(
        governing("cache.local.max_size").key,
        "cache.local",
        "the two tiers are two rows: one answering for both would be a single figure for a map \
         held once per process and a map held once per core",
    );
    assert!(
        lookup("cache").is_none() && lookup("cache.nvs_no_such_tier.max_size").is_none(),
        "a blanket `cache` row would hand every future tier whichever class it happened to carry, \
         and `cache.shared` being `Boot` beside these two is what that row could not say",
    );
}

/// `rule:core-api/two-cache-tiers`: the coherent tier is a *store* rather than a map, and every key
/// naming or reaching it is the operator's and applies at boot.
///
/// `System` because where a fleet's coherent state lives is not a decision one request may make for
/// the rest — and because four of the five keys are how that store is *reached*, so a request able
/// to write them would be re-pointing a credential as well as an address. `Boot` because each core
/// holds one connection to the store: moving it re-dials every one of them, which is the same
/// "re-creates the runtime's mapping" the artifact directory is `Boot` for.
///
/// The last assertion is the one the three-rows-per-tier shape exists for. `[cache]` holds two apply
/// classes at once — this tier's `Boot` beside the two in-memory tiers' `Reload` — so a blanket row
/// above them could only be right about one, and which of the two it got wrong is either an
/// operator restarting for a ceiling change or a fleet believing it moved a store it did not.
// covers: directive:cache.shared
#[test]
fn every_shared_tier_key_is_system_class_applied_at_boot_beside_two_reload_siblings() {
    let keys = keys_in("cache.shared");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["url", "password", "password_file", "database", "timeout"],
        "`[cache.shared]` accepts {keys:?}, and the one row governs whatever the block holds: a \
         key added to it joins this census in the commit that adds it",
    );

    for key in keys.iter().map(|key| format!("cache.shared.{key}")) {
        let row = governing(&key);
        assert_eq!(
            row.key, "cache.shared",
            "`{key}` resolves through `{}` rather than through the tier's own row",
            row.key,
        );
        assert_eq!(
            row.class,
            Class::System,
            "`{key}` is part of naming and reaching the store a deployment shares, which is the \
             operator's decision and not a request's (`rule:core-api/two-cache-tiers`)",
        );
        assert!(
            !row.class.settable_by_a_request(),
            "`Core\\Config::set(\"{key}\", …)` has to refuse: a request that could write any of \
             these would be pointing every other core at a store it chose",
        );
        assert_eq!(
            row.apply,
            Apply::Boot,
            "every core holds an open connection to this store, so a change here re-dials all of \
             them rather than being read by the next caller \
             (`rule:config/reloadability-is-its-own-field`)",
        );
    }

    for sibling in ["cache.local.max_size", "cache.process.fill_wait"] {
        assert_eq!(
            governing(sibling).apply,
            Apply::Reload,
            "`{sibling}` bounds a map this process already holds, so it applies without re-dialling \
             anything — and `[cache]` carrying both apply classes is why the tiers are three rows",
        );
    }
}

/// `rule:config/one-local-control-socket`: the one local door to a running server is the operator's,
/// and it is created once.
///
/// `System` because the socket's owner and mode *are* the authentication, so a request able to write
/// the key would be choosing where that door is and which account answers it — the registry's class
/// is the only thing standing between a served request and that choice. `Boot` because the endpoint
/// is a kernel object created as the server starts, and a reload that renamed it would leave
/// `nvs ctl` addressing the old one.
///
/// The last assertion is the one that distinguishes this row's *shape* from `[capabilities]`', and
/// the registry states both. This row is keyed at the dotted key, so `[control]` has no blanket row
/// and a second key added to the block is governed by nothing — which `Core\Config::set` reads as
/// unwritable rather than as this row's class. A blanket `control` row would hand that future key
/// `System`/`Boot` by accident, which is the right answer arrived at by not asking.
// covers: directive:control.socket
#[test]
fn the_control_socket_is_one_system_key_applied_at_boot_and_its_block_has_no_row() {
    let keys = keys_in("control");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["socket"],
        "`[control]` accepts {keys:?}, and there is one local endpoint by design: a second key \
         joins this census in the commit that adds it, and needs a row of its own to be governed",
    );

    let row = governing("control.socket");
    assert_eq!(
        row.key, "control.socket",
        "`control.socket` resolves through `{}`, so the block rather than the key is what the \
         registry is stating something about",
        row.key,
    );
    assert_eq!(
        row.class,
        Class::System,
        "who owns the socket is the whole of its authentication, so where it lives is not a \
         decision one served request may make (`rule:config/one-local-control-socket`)",
    );
    assert!(
        !row.class.settable_by_a_request(),
        "`Core\\Config::set(\"control.socket\", …)` has to refuse: a request that moved the door \
         would be choosing which account answers the next administrative operation",
    );
    assert_eq!(
        row.apply,
        Apply::Boot,
        "the endpoint is created as the server starts, so a new name is a new kernel object rather \
         than a value the next caller reads (`rule:config/reloadability-is-its-own-field`)",
    );

    assert!(
        lookup("control").is_none() && lookup("control.nvs_no_such_key").is_none(),
        "a blanket `control` row would govern a key nobody has written a row for, and handing a \
         future key `System`/`Boot` by accident is the right answer reached without asking",
    );
}

/// `rule:core-classes/temporary-dir-sweep`: the one switch that stops the end-of-script sweep belongs to the
/// operator, and it is read again by the next script that ends.
///
/// `System` because the rule gives the key to the operator alone: a request able to write it would
/// be exempting its own files from the sweep, and a program that can do that can be talked into
/// hoarding until the disk is full — which is why the rule states that there is no in-language
/// setter at all, leaving this row as the only thing that has to hold. `Reload` because what reads
/// it is the next script to end rather than anything created at boot, so a flip re-creates nothing;
/// that is what makes "on around one problematic request and off again" a thing an operator can do.
///
/// `io.temp_root` beside it is what makes the pairing a claim rather than a spelling. Both keys are
/// the operator's and neither is a request's, and they still separate: the root is a directory the
/// boot sweep walks once with the path it started with, so moving it under a running server would
/// strand everything in the old one, while keeping what the sweep would have deleted is a question
/// the sweep asks each time it runs. Same class, different apply, and `rule:config/reloadability-is-its-own-field` is the
/// pairing that makes both sayable.
// covers: directive:debug.keep_temporary
#[test]
fn keeping_a_temporary_directory_is_the_operators_at_reload_while_its_root_is_fixed_at_boot() {
    let keys = keys_in("debug");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["mode", "inline", "keep_temporary"],
        "`[debug]` accepts {keys:?}, and a key added to the block joins this census in the commit \
         that adds it — the two the registry governs are named below and the third is stated as \
         ungoverned rather than left to be discovered",
    );

    let row = governing("debug.keep_temporary");
    assert_eq!(
        row.key, "debug.keep_temporary",
        "`debug.keep_temporary` resolves through `{}`, so what the registry is stating something \
         about is the block rather than this key",
        row.key,
    );
    assert_eq!(
        row.class,
        Class::System,
        "a request that could keep its own temporary directories would be exempting its own files \
         from the sweep, which is the hoarding `rule:core-classes/temporary-dir-sweep` refuses",
    );
    assert!(
        !row.class.settable_by_a_request(),
        "`Core\\Config::set(\"debug.keep_temporary\", true)` has to refuse: the rule gives this key \
         no in-language setter, and this row is the whole of what enforces that",
    );
    assert_eq!(
        row.apply,
        Apply::Reload,
        "the next script to end is what reads it, so a flip applies to the sweep after it and \
         re-creates nothing (`rule:config/reloadability-is-its-own-field`)",
    );

    let root = governing("io.temp_root");
    assert_eq!(
        (root.class, root.apply),
        (Class::System, Apply::Boot),
        "the root the sweep walks is created and swept once as the server starts, so moving it \
         under a running server would leave the old one holding entries nothing sweeps — the same \
         class as the key above and the other apply, which is the contrast this case exists for",
    );
    assert_eq!(
        row.class, root.class,
        "both keys are the operator's, and a difference in class here would mean one of them had \
         become a decision a served request may make",
    );

    assert!(
        lookup("debug").is_none(),
        "a blanket `debug` row would hand `debug.mode` — the one key of this block no row governs — \
         `System`/`Reload` without anybody deciding that is what it is",
    );
}

/// `rule:errors/debug-dump`: the key deciding whether a dump reaches the page is the one key of
/// `[debug]` a request may name at all, and it may only ever be narrowed.
///
/// `RuntimeTighten` rather than `Runtime` is the rule's security half stated as a class: a request
/// that could raise this key would be putting its own dump in the response it is writing, so the
/// only direction the registry leaves open is the one that discloses less, and the run mode's
/// default is the only thing that turns it on. What reaches a request is narrower still — a
/// `RuntimeTighten` row that is not a quantity cannot be set in either direction, which
/// `nvs_config::request`'s module doc owns — but that is the overlay's answer *over* this row, and
/// a `Runtime` row here would leave the overlay as the only thing in the way.
///
/// `Reload` because every dump a request makes reads it in force, so a changed value reaches the
/// next one with nothing re-created.
///
/// The census claim is the pairing inside one block: `[debug]`'s other governed key is `System`, so
/// a case reading this row alone would pass against a registry that had quietly made both of them
/// the same class.
// covers: directive:debug.inline
#[test]
fn an_inline_dump_is_the_one_debug_key_a_request_may_name_and_only_downwards() {
    let row = governing("debug.inline");
    assert_eq!(
        row.key, "debug.inline",
        "`debug.inline` resolves through `{}`, so the block rather than the key is what the \
         registry is stating something about",
        row.key,
    );
    assert_eq!(
        row.class,
        Class::RuntimeTighten,
        "`Runtime` here would let a request raise the key and write its own dump into the response, \
         which is the disclosure `rule:errors/debug-dump` exists to close",
    );
    assert!(
        row.class.settable_by_a_request(),
        "the tightening direction is open at the registry, and what refuses a value nothing can \
         compare is the overlay in `nvs_config::request` — a `System` row here would be a second \
         answer to a question that already has one",
    );
    assert_eq!(
        row.apply,
        Apply::Reload,
        "a dump reads the key in force, so a changed value reaches the next request by the \
         snapshot being rebuilt (`rule:config/reloadability-is-its-own-field`)",
    );

    let sibling = governing("debug.keep_temporary");
    assert_eq!(
        sibling.class,
        Class::System,
        "`[debug]` is not one class with a spelling per key: the sweep's switch is the operator's \
         and this one is not, and a census reading either row alone cannot see that",
    );

    let derived = nvs_config::mode::DERIVED
        .iter()
        .find(|derived| derived.key == "debug.inline")
        .expect("`debug.inline` is the one directive whose default only a mode can turn on");
    assert_eq!(
        derived.production, "false",
        "the mode's default is the only thing that enables this key, and a host that wrote no \
         configuration starts in `production` — so the one value that must be off is this one",
    );
}

/// `rule:concurrency/deferred-is-bounded-by-two-directives`: the two keys of `[deferred]` are two
/// classes, and the rule is explicit that they are.
///
/// The cap counts request trees a core keeps alive after their responses are on the wire, so it is
/// what the rule prices a host against — `max_concurrent × [limits.hard]` memory on top of the
/// in-flight requests — and a request that could raise it would be sizing the machine every *other*
/// request runs on. The deadline is an ordinary per-request default a call may name its own value
/// for, which makes it the first row in this block's neighbourhood a request may write at all.
///
/// Both are `Reload`: neither is anything a boot created, and a changed value reaches the next
/// deferred tree by the snapshot being rebuilt. So the block is a pairing that only
/// `rule:config/reloadability-is-its-own-field`'s two fields can state — one apply class, two
/// changeability classes — and a case reading either row alone would pass against a registry that
/// had made both of them the cap's.
// covers: directive:deferred.deadline
#[test]
fn a_deferred_deadline_is_a_requests_to_set_while_the_cap_beside_it_is_the_hosts() {
    let keys = keys_in("deferred");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["max_concurrent", "deadline"],
        "`[deferred]` accepts {keys:?}, and both halves are named below: a third key joins this \
         census in the commit that adds it, and needs a row of its own to be governed at all",
    );

    let deadline = governing("deferred.deadline");
    assert_eq!(
        deadline.key, "deferred.deadline",
        "`deferred.deadline` resolves through `{}`, so the block rather than the key is what the \
         registry is stating something about — and the block is the thing that must not be one row",
        deadline.key,
    );
    assert_eq!(
        deadline.class,
        Class::Runtime,
        "a call may name its own deadline, so the default it inherits is an ordinary per-request \
         one rather than a decision the operator keeps",
    );
    assert!(
        deadline.class.settable_by_a_request(),
        "`Core\\Config::set(\"deferred.deadline\", \"5s\")` is the call the rule describes, and a \
         class that refused it would leave the per-call value as the only way to shorten anything",
    );

    let cap = governing("deferred.max_concurrent");
    assert_eq!(
        cap.class,
        Class::System,
        "the cap is what the rule prices a host's memory against, so a request that could raise it \
         would be sizing the machine every other request is running on",
    );
    assert!(
        !cap.class.settable_by_a_request(),
        "`Core\\Config::set(\"deferred.max_concurrent\", …)` has to refuse: past the cap \
         `afterResponse` throws, and a request that could move it would be choosing how much of \
         the core's memory the deferred trees may hold",
    );
    assert_ne!(
        deadline.class, cap.class,
        "`[deferred]` is the block `rule:concurrency/deferred-is-bounded-by-two-directives` writes \
         as two classes, and a registry that had quietly made it one would still answer plausibly \
         for either key on its own",
    );

    assert_eq!(
        (deadline.apply, cap.apply),
        (Apply::Reload, Apply::Reload),
        "neither key is anything a boot created, so a changed value reaches the next deferred tree \
         by the snapshot being rebuilt — one apply class across a block whose changeability splits",
    );

    assert!(
        lookup("deferred").is_none(),
        "a blanket `deferred` row would govern both halves at once, which is exactly the one class \
         this case exists to say the block does not have",
    );
}

/// `rule:concurrency/deferred-is-bounded-by-two-directives` prices a host against a **product** —
/// `max_concurrent × [limits.hard]` memory, on top of the requests still being answered — and a
/// product is bounded only while both of its factors are. So this reads the cap against the other
/// factor rather than against the sibling key beside it, which the case above already pins.
///
/// `[limits]` is `Runtime`, because a request lowering its own ceiling is its own business, and
/// `[limits.hard]` is the `System` row above it that no request can move. A registry that let go of
/// either factor would leave an operator's arithmetic true of nothing: the cap would go on refusing
/// to be raised while every tree it counts grew underneath it.
///
/// The two factors are also two *units*, which is what stops the product being nonsense — the cap
/// counts trees and the ceiling is bytes — so a cap written in a size's spelling is refused rather
/// than read as two hundred and sixty-eight million trees.
///
/// Past the cap `Core\Task::afterResponse` throws at the call site rather than queueing
/// (`nvs_runtime::deferred`), and the registry's half of that is that there is nothing else to
/// write. `[deferred]` governs two numbers and no behaviour, so an operator who wanted a queue or a
/// wait in front of a non-durable executor has no key for one and no row would govern it.
// covers: directive:deferred.max_concurrent
#[test]
fn the_deferred_cap_is_priced_against_a_ceiling_no_request_can_move() {
    let cap = governing("deferred.max_concurrent");
    let ceiling = governing("limits.hard.memory");
    let per_request = governing("limits.memory");

    assert_eq!(
        (cap.key, ceiling.key),
        ("deferred.max_concurrent", "limits.hard"),
        "the rule's product is these two rows, and `limits.hard.memory` resolving through any \
         other one would mean the ceiling being multiplied is not the ceiling an operator wrote",
    );
    assert_eq!(
        (cap.class, ceiling.class),
        (Class::System, Class::System),
        "both factors of `max_concurrent × [limits.hard]` are the operator's, and a registry that \
         let go of either would leave the cap refusing to be raised while every tree it counts \
         grew underneath it",
    );
    assert_eq!(
        per_request.class,
        Class::Runtime,
        "the ceiling a request may move is the soft one, which is the whole reason the product is \
         written against the hard row and not against `[limits]`",
    );
    assert!(
        per_request.class.settable_by_a_request() && !ceiling.class.settable_by_a_request(),
        "a request may lower what it is allowed to hold and may not raise what anything else is \
         allowed to hold — the two halves of `[limits]` that make the cap's arithmetic an operator's",
    );

    // The units, which is what the multiplication means at all: a count of trees times a size.
    let quantity = |key: &str, unit, text: &str| {
        nvs_config::Quantity::parse(key, unit, &nvs_config::Setting::Text(text.to_string()))
    };
    assert!(
        matches!(
            quantity(cap.key, nvs_config::Unit::Count, "256"),
            Ok(nvs_config::Quantity::Count(256))
        ),
        "the cap is a plain count of request trees, which is the factor an operator multiplies",
    );
    assert!(
        quantity(cap.key, nvs_config::Unit::Count, "256M").is_err(),
        "a size's spelling in the cap is refused rather than read as two hundred and sixty-eight \
         million trees — a cap that admitted the other factor's unit would price a host at a \
         figure nobody wrote",
    );
    assert!(
        matches!(
            quantity(ceiling.key, nvs_config::Unit::Bytes, "512M"),
            Ok(nvs_config::Quantity::Bytes(536_870_912))
        ),
        "and the other factor is bytes, so the product is bytes per core and not a number with no \
         unit at all",
    );

    // Being full is a refusal and not a setting. `nvs_runtime::deferred` throws at the call site
    // while the request can still do the work inline, and no key here can turn that into a queue.
    for invented in [
        "deferred.overflow",
        "deferred.on_capacity",
        "deferred.max_queued",
        "deferred.queue_depth",
        "deferred.wait",
    ] {
        assert!(
            lookup(invented).is_none(),
            "`{invented}` resolves to a row, so `[deferred]` has a second thing to say about being \
             full — and the throw past the cap is a default somebody can configure away rather \
             than the whole of the behaviour",
        );
    }

    assert_eq!(
        governing("deferred.max_concurrent.default").key,
        "deferred.max_concurrent",
        "a key invented underneath the cap is still governed by the cap's row, so a spelling \
         nobody audited cannot fall through to being governed by nothing and settable by anyone",
    );
}

/// `rule:packaging/extension-loading-is-root-controlled`: an extension is loaded from an
/// `[[extension]]` entry in the root-owned file and from nowhere else, so the row is `System` — the
/// class that answers nobody — down to every key of every entry.
///
/// **`Reload` is the half the rule states outright**, and it is not bookkeeping: `Boot` here would
/// mean an operator could not add, replace or drop an extension without dropping every request in
/// flight. The swap re-verifies each pin against the file on disk and refuses the whole set if one
/// does not match, and the set folds into every compiled unit's key
/// (`rule:config/the-extension-set-is-in-every-unit-key`), so nothing compiled against the old set
/// is reused against the new one.
///
/// The entry's keys are read back out of the block rather than listed here, and that the pin is one
/// of them is the rule's own reasoning: a checksum spelled as a naming convention over a repeated
/// key — an `image_sha256` beside an `image` — would be a pin an entry could simply omit without
/// looking incomplete, which is why the format has an array-of-tables shape at all.
// covers: directive:extension
#[test]
fn an_extension_entry_carries_its_pin_and_no_part_of_it_is_a_programs() {
    let keys = keys_listed("[[extension]]\nnvs_no_such_key = true\n");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["path", "sha256"],
        "a key added to `[[extension]]` joins this census in the commit that adds it, and the pin \
         sitting beside the path as a field of the same entry is what the shape buys",
    );

    let mut asked = vec!["extension".to_string(), "extension.0".to_string()];
    for key in &keys {
        asked.push(format!("extension.{key}"));
        asked.push(format!("extension.0.{key}"));
    }

    for key in asked {
        let row = governing(&key);
        assert_eq!(
            row.key, "extension",
            "`{key}` resolves through `{}` rather than through the one `extension` row, so which \
             binaries this host loads is no longer one class",
            row.key,
        );
        assert_eq!(
            row.class,
            Class::System,
            "`{key}` decides what native code runs inside every request on this host, so it is \
             the class that answers nobody",
        );
        assert!(
            !row.class.settable_by_a_request(),
            "`Core\\Config::set(\"{key}\", …)` has to refuse: a project that could cause code to \
             be loaded is the one thing this rule exists to refuse, and a pin it could write is a \
             pin it could write to match whatever it put on disk",
        );
        assert_eq!(
            row.apply,
            Apply::Reload,
            "`{key}` is reloadable rather than boot-only, which the rule states outright: a `Boot` \
             here would price adding or dropping an extension at every request in flight",
        );
    }
}

/// The one `http` row, read as the *default* answer for the whole `[http.*]` tree rather than as
/// one more row: `rule:config/three-changeability-classes` names a response header as its
/// counter-example to `System`, so a request may shape its own response because it could already
/// have written the header by hand.
///
/// What that leaves worth asserting is the **exceptions**, and that they have exactly two grounds.
/// A key is taken back out of the blanket when it is a secret every request's door reads — the CSRF
/// key, which a request able to choose would be choosing which forgeries its co-residents accept —
/// or when it settles a resource more than one request shares: the outbound pool's caps and the
/// process-wide TLS configuration and proxy. Neither ground is about `[http]` being outbound or
/// inbound, which is why `[http.client]`'s per-call bounds are still the blanket's.
///
/// So the census runs the other way round from a list of exceptions. Every row under `http` is read
/// out of the registry and each has to be one of those, and `[http.client]`'s own keys are read out
/// of the block: one added to it joins this case in the commit that adds it, and lands either on an
/// exception or on a bound over the single call the request is making.
///
/// `http.client.socket` is the row that makes this worth asserting at all. It carries a row and
/// restates the blanket's own answer, so a census reading "a row under `http` means `System`" would
/// pass against a registry that had quietly made it one.
// covers: directive:http
#[test]
fn the_one_http_row_answers_for_a_response_and_its_exceptions_are_shared_or_secret() {
    let blanket = governing("http.errors.detail");
    assert_eq!(
        blanket.key, "http",
        "`http.errors.detail` resolves through `{}`, so the block a response is shaped by is no \
         longer one row with exceptions written against it",
        blanket.key,
    );
    assert_eq!(
        (blanket.class, blanket.apply),
        (Class::Runtime, Apply::Reload),
        "a handler that wanted another policy could write the header itself, so shaping its own \
         response buys it nothing — and a changed block reaches the next request by the snapshot \
         being rebuilt",
    );

    let mut carved: Vec<&str> = DIRECTIVES
        .iter()
        .filter(|row| row.key.starts_with("http.") && row.class != blanket.class)
        .map(|row| row.key)
        .collect();
    carved.sort_unstable();
    assert_eq!(
        carved,
        [
            "http.client.pool_idle",
            "http.client.pool_idle_timeout",
            "http.client.proxy",
            "http.client.tls",
            "http.csrf_key",
            "http.csrf_key_file",
        ],
        "a key leaves the blanket row for one of two reasons and no third — it is a secret the \
         door reads for every request, or it settles something more than one request shares — so a \
         seventh exception is a decision to write down rather than a row to add",
    );
    for key in &carved {
        assert_eq!(
            governing(key).class,
            Class::System,
            "`{key}` is not this request's to choose, and the class that answers nobody is the \
             only one that says so",
        );
    }

    for key in keys_in("http.client") {
        let dotted = format!("http.client.{key}");
        let row = governing(&dotted);
        if row.key != "http" {
            assert!(
                carved.contains(&row.key) || row.key == "http.client.socket",
                "`{dotted}` resolves through `{}`, which is neither the blanket row nor one of the \
                 exceptions above — so the block now answers a third way nothing has stated",
                row.key,
            );
            continue;
        }
        assert!(
            [
                "connect_timeout",
                "deadline",
                "idle",
                "max_duration",
                "max_redirects"
            ]
            .contains(&key.as_str()),
            "`{dotted}` falls through to the blanket row, so a request may set it — which is only \
             right for a bound over the one call that request is making",
        );
    }

    let socket = governing("http.client.socket");
    assert_eq!(
        (socket.key, socket.class),
        ("http.client.socket", Class::Runtime),
        "the one row under `[http.client]` that is not an exception restates the blanket's answer, \
         and a census reading a row there as `System` would pass against a registry that had made \
         it one",
    );
}

/// `rule:security/csrf-is-on-by-default`'s key, read against the block it is carved out of: the door
/// verifies *every* request's token against this one value, so a request that could write it would be
/// choosing which forgeries its co-residents accept, and one that could clear it would be turning the
/// token half of the check off for all of them.
///
/// **The contrast is one segment wide**, which is why the case asserts both sides of it. `[http]` is
/// the block a request may write — a handler that wanted another policy could write the header itself
/// — so a lookup that failed to reach this row would not fail loudly. It would land on the blanket
/// and quietly answer `Runtime` for the key the whole check rests on.
///
/// **`Reload` rather than `Boot`** because the door reads the standing tree per request, so a rotated
/// key governs the next request. The pair is asserted together with the class for
/// `rule:config/reloadability-is-its-own-field`'s reason: `Boot` here would read as the cautious
/// answer for a secret and would mean a deployment that rotated a leaked key went on accepting
/// tokens minted under it until somebody restarted the server.
// covers: directive:http.csrf_key
#[test]
fn the_door_key_is_carved_out_of_a_block_a_request_may_otherwise_write() {
    let row = governing("http.csrf_key");
    assert_eq!(
        (row.key, row.class, row.apply),
        ("http.csrf_key", Class::System, Apply::Reload),
        "`http.csrf_key` resolves through `{}` to {:?}/{:?}, and each half of that would be wrong its \
         own way: `Runtime` hands a request the key every other request is checked against, and \
         `Boot` leaves a rotated key out of force until a restart",
        row.key,
        row.class,
        row.apply,
    );
    assert!(
        !row.class.settable_by_a_request(),
        "`Core\\Config::set(\"http.csrf_key\", …)` has to refuse, in both directions: naming a key \
         mints tokens the door believes and clearing one disarms the check for every co-resident \
         request",
    );

    // The blanket the row is carved out of, asserted beside it because the two answers are one
    // segment apart and the near miss lands on this one rather than on nothing.
    let blanket = governing("http.cookies.samesite");
    assert_eq!(
        (blanket.key, blanket.class),
        ("http", Class::Runtime),
        "the block the key sits in is the request's own, so the carve-out is the whole of what keeps \
         it out of reach — a lookup that missed it would answer {:?} instead of failing",
        blanket.class,
    );
    assert!(
        blanket.class.settable_by_a_request(),
        "`[http]` is settable by design (`rule:config/three-changeability-classes`), which is what \
         makes the row above load-bearing rather than a restatement",
    );

    // A key written *beneath* the carve-out stays inside it, and a key that merely begins with its
    // text does not: a prefix here is segments, so `http.csrf_keyx` is the blanket's and no spelling
    // of the door's key is ever answered twice.
    for beneath in ["http.csrf_key.value", "http.csrf_key.0"] {
        assert_eq!(
            governing(beneath).key,
            "http.csrf_key",
            "`{beneath}` has to resolve through the carve-out, or a request would write the key by \
             writing something underneath it",
        );
    }
    assert_eq!(
        governing("http.csrf_keyx").key,
        "http",
        "a longer text is not a longer prefix, so the carve-out covers exactly the key it names — a \
         registry matching on text would take keys out of `[http]` that nobody decided about",
    );
}

/// The `_file` spelling of the key above, and the one thing this case exists to pin: two spellings of
/// one value answer the same way. `rule:config/a-secret-is-a-file-whose-content-is-the-value` makes
/// the file half the spelling an audited deployment writes, and exactly one of the pair may be set —
/// so a registry that answered `Runtime` for this one would hand a request the door's key through the
/// half an operator was told to prefer.
///
/// It carries a **row of its own** rather than inheriting the inline one's, because a prefix is
/// segments and not text: `http.csrf_key_file` is not beneath `http.csrf_key`, and without the row it
/// would fall through to the `[http]` blanket. The sweep at the end is over every `_file` row the
/// registry holds rather than this one alone, so a credential that gains a file half later answers as
/// its inline half in the commit that adds it. A secret in an operator-named block — `db.main.password`
/// — is governed by that block's row and audited in `tests/secret.rs`, which is why this sweep is
/// over the registry and not over the secret table.
// covers: directive:http.csrf_key_file
#[test]
fn the_file_half_of_the_door_key_answers_exactly_as_the_inline_half_does() {
    let inline = governing("http.csrf_key");
    let file = governing("http.csrf_key_file");
    assert_eq!(
        (file.key, file.class, file.apply),
        ("http.csrf_key_file", Class::System, Apply::Reload),
        "`http.csrf_key_file` resolves through `{}`, and a row of its own is what it needs: the \
         inline key is not a prefix of it, so nothing else would keep it out of `[http]`",
        file.key,
    );

    let mut siblings: Vec<(&str, Class, Apply)> = DIRECTIVES
        .iter()
        .filter(|row| row.key.ends_with("_file"))
        .map(|row| (row.key, row.class, row.apply))
        .collect();
    siblings.sort_unstable_by_key(|(key, _, _)| *key);
    assert_eq!(
        siblings,
        [("http.csrf_key_file", Class::System, Apply::Reload)],
        "a `_file` row added to the registry joins this census in the commit that adds it, and the \
         one it has to match is its own inline half",
    );
    for (key, class, apply) in siblings {
        let stem = governing(key.trim_end_matches("_file"));
        assert_eq!(
            (class, apply),
            (stem.class, stem.apply),
            "`{key}` and `{}` are two spellings of one value, so a difference here is a second \
             answer to who may write it — and the file half is the one an audited deployment was \
             told to use",
            stem.key,
        );
    }
    assert!(
        !file.class.settable_by_a_request() && !inline.class.settable_by_a_request(),
        "neither spelling is a request's, and asserting the pair together is the point: one of them \
         settable is the whole refusal gone",
    );
}

/// `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`'s first cap, read
/// for its **ground** rather than for its class: the pool is per core, so the count bounds a core's
/// memory and not a request's, and that is the only reason it is not the blanket row's.
///
/// The case is written as a contrast inside one block, because the wrong ground is the one a reader
/// reaches for. "`[http.client]` is the operator's" would be a rule with the same answer here and a
/// different answer five keys away: `connect_timeout`, `deadline`, `idle`, `max_duration` and
/// `max_redirects` sit in the same block, fall through to `http`, and a request may set every one of
/// them — each bounds the single call that request is making and spends nothing anyone else then
/// goes without. So the census asserts both sides of the line at once.
///
/// **A near miss under the cap is the half that would go unnoticed.** The blanket row is `Runtime`,
/// so a lookup that did not reach `http.client.pool_idle` from a key written beneath it would not
/// fall through to nothing and fail loudly — it would land on `http` and quietly become a spelling
/// a request may set, next door to a cap on memory it shares with every co-resident request.
// covers: directive:http.client.pool_idle
#[test]
fn the_outbound_pools_count_is_the_operators_because_a_core_holds_it_not_a_request() {
    let cap = governing("http.client.pool_idle");
    assert_eq!(
        (cap.key, cap.class, cap.apply),
        ("http.client.pool_idle", Class::System, Apply::Reload),
        "the count of idle connections one core keeps is carved out of the `http` blanket by a row \
         of its own, and `Reload` because the caps are read when a call asks the pool for a \
         connection, so a changed value bounds the next one",
    );
    assert!(
        !cap.class.settable_by_a_request(),
        "`Core\\Config::set(\"http.client.pool_idle\", …)` has to refuse: the connections are the \
         core's and outlive the request that opened them, so a request raising the count would be \
         spending memory every co-resident request then goes without",
    );

    // The other side of the line, in the same block: a bound over the one call the request is
    // making is that request's own business, so "`[http.client]` is the operator's" is not the
    // ground and a case asserting it would pass here while being wrong five keys away.
    for per_call in [
        "connect_timeout",
        "deadline",
        "idle",
        "max_duration",
        "max_redirects",
    ] {
        let dotted = format!("http.client.{per_call}");
        let row = governing(&dotted);
        assert_eq!(
            (row.key, row.class),
            ("http", Class::Runtime),
            "`{dotted}` resolves through `{}` to {:?}, and it bounds the single call this request \
             is making — the ground for the pool's caps is whose memory is held, not which block \
             the key is written in",
            row.key,
            row.class,
        );
        assert!(
            row.class.settable_by_a_request(),
            "a program that knows its own upstream names `{dotted}` at the call site, so a \
             registry refusing it would refuse nothing and cost the caller the one spelling that \
             is genuinely theirs",
        );
    }

    // A key invented beneath the cap. `deferred.max_concurrent`'s case asks this against a block
    // with no blanket row at all, where a miss reaches nothing; here a miss reaches `Runtime`.
    for beneath in [
        "http.client.pool_idle.max",
        "http.client.pool_idle.default",
        "http.client.pool_idle.per_core",
    ] {
        let row = governing(beneath);
        assert_eq!(
            (row.key, row.class),
            ("http.client.pool_idle", Class::System),
            "`{beneath}` resolves through `{}` to {:?} — a spelling nobody audited fell through to \
             the blanket row, so a cap on a core's memory has a request-settable name sitting \
             underneath it",
            row.key,
            row.class,
        );
    }
}

/// The pool's other cap, and the case is that it is a **second row** rather than the first one's
/// suffix. `http.client.pool_idle` is a plain string prefix of `http.client.pool_idle_timeout`, so
/// a registry that resolved by text rather than on dot boundaries would answer `System` for the
/// timeout with no row for it at all — the right answer for the wrong reason, which no other
/// assertion in this file would notice.
///
/// What the second row buys is what deleting it would cost:
/// `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned` bounds the pool
/// with a count *and* a duration because neither is a bound on its own — under the count alone a
/// connection the far end retired hours ago is still one of the sixteen a core holds, and a request
/// able to move the duration would hold every one of them open for as long as it liked. The
/// registry's half of that is one class for both halves, and the dot-boundary lookup is what keeps
/// the second half from inheriting its answer.
// covers: directive:http.client.pool_idle_timeout
#[test]
fn the_pools_timeout_is_its_own_row_and_not_the_counts_suffix() {
    let cap = governing("http.client.pool_idle");
    let timeout = governing("http.client.pool_idle_timeout");
    assert_eq!(
        (timeout.key, timeout.class, timeout.apply),
        (
            "http.client.pool_idle_timeout",
            Class::System,
            Apply::Reload
        ),
        "the timeout resolves through `{}`, so how long a core may hold a dead connection open is \
         answered by a row that is not about it",
        timeout.key,
    );
    assert!(
        !timeout.class.settable_by_a_request(),
        "a request that could raise the timeout would keep every idle connection this core holds \
         open for as long as it liked, which is the count's memory spent over a window the count \
         says nothing about",
    );

    // The boundary itself, asked of a key the count's text covers and its block does not. A
    // text-prefix lookup answers `http.client.pool_idle` here and would answer it for the timeout
    // too; the dot-boundary one drops through to the blanket, which is what makes the second row
    // load-bearing rather than a longer spelling of the first.
    let near_miss = governing("http.client.pool_idlex");
    assert_eq!(
        (near_miss.key, near_miss.class),
        ("http", Class::Runtime),
        "`http.client.pool_idlex` resolves through `{}`, so the registry matches a row by text \
         rather than on dot boundaries — under which the count would govern the timeout, the \
         second row could be deleted with nothing failing, and a key nobody wrote would inherit a \
         cap's class",
        near_miss.key,
    );

    // Two caps, two units, which is why one does not imply the other: a count of connections and a
    // wait, so a timeout written in the count's spelling is refused rather than read as thirty.
    let quantity = |key: &str, unit, text: &str| {
        nvs_config::Quantity::parse(key, unit, &nvs_config::Setting::Text(text.to_string()))
    };
    assert!(
        matches!(
            quantity(timeout.key, nvs_config::Unit::Duration, "30s"),
            Ok(nvs_config::Quantity::Nanos(30_000_000_000))
        ),
        "the timeout is a wait, which is the half of the pool's bound the count cannot state",
    );
    assert!(
        matches!(
            quantity(timeout.key, nvs_config::Unit::Duration, "30"),
            Ok(nvs_config::Quantity::Nanos(30_000_000_000))
        ),
        "an unsuffixed timeout is seconds rather than a refusal or some smaller unit, which is the \
         whole of `Unit::Duration::accepts` and the reading an operator who wrote the number beside \
         a count is owed",
    );
    assert!(
        quantity(cap.key, nvs_config::Unit::Count, "30s").is_err(),
        "the count takes no suffix at all, so neither cap can be written in the other's spelling — \
         a pair that shared a spelling would let an operator set a window where they meant a \
         number of connections and read back a bound nobody wrote",
    );
}

/// `rule:security/one-tls-client`: there is one TLS client in the process and `[http.client.tls]` is
/// what settles it, so every key of the block is `System` — a request writing one would be choosing
/// whose certificates every co-resident request believes.
///
/// **`Boot` is the half no other row under `[http.client]` answers**, and it is what this case is
/// worth a place for. The configuration object those three keys describe is built once on first use
/// and handed out by `Arc` after it (`nvs_host::tls`), so a snapshot arriving with a changed anchor
/// set has nothing to apply it to: an operator who edits the block and reloads would be told the
/// change landed while every call went on believing the old list. The census over the whole of
/// `[http.client]` is here rather than a second assertion on the block already named, because a row
/// added there inherits `Reload` from the blanket without anybody deciding it should.
///
/// The keys are read back out of the block rather than listed, so one added to `[http.client.tls]`
/// joins this census in the commit that adds it — and `keylog` is in it, which matters: the file it
/// names decrypts everything this deployment sends, and `Runtime` for that one key would put the
/// whole of it inside a request's reach.
// covers: directive:http.client.tls
#[test]
fn every_key_of_the_tls_block_is_the_deployments_and_waits_for_a_restart() {
    let keys = keys_in("http.client.tls");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["roots", "min_version", "keylog"],
        "a key added to `[http.client.tls]` joins this census in the commit that adds it, and all \
         three of these describe the one client the process shares rather than one call",
    );

    let mut asked = vec!["http.client.tls".to_string()];
    asked.extend(keys.iter().map(|key| format!("http.client.tls.{key}")));
    for key in asked {
        let row = governing(&key);
        assert_eq!(
            (row.key, row.class, row.apply),
            ("http.client.tls", Class::System, Apply::Boot),
            "`{key}` resolves through `{}` to {:?}/{:?}, and one of those two being wrong is a \
             different failure: `Runtime` would hand a request the trust list, and `Reload` would \
             report a reloaded anchor set that no call is using",
            row.key,
            row.class,
            row.apply,
        );
        assert!(
            !row.class.settable_by_a_request(),
            "`Core\\Config::set(\"{key}\", …)` has to refuse, and there is no code-side spelling to \
             refuse it in favour of: `rule:security/one-tls-client` leaves whose certificates are \
             believed to the deployment alone",
        );
    }

    // Every row carved under `[http.client]`, filtered to the ones a change cannot reach without a
    // restart. Asserted over the block rather than over this one row, because `Boot` here is not the
    // consistent answer for its neighbours — the proxy is `System` and reaches the next call — so a
    // row added beside it takes `Reload` from the blanket unless somebody decided otherwise.
    let mut boot: Vec<&str> = DIRECTIVES
        .iter()
        .filter(|row| row.key.starts_with("http.client") && row.apply == Apply::Boot)
        .map(|row| row.key)
        .collect();
    boot.sort_unstable();
    assert_eq!(
        boot,
        ["http.client.tls"],
        "of the rows carved under `[http.client]`, this is the only one an operator cannot change \
         without restarting — a second one arriving here is a key whose new value a reload would \
         report and no outbound call would use",
    );
}

/// `rule:http-server/an-outbound-proxy-is-operator-configured`: a call leaves through a forward
/// proxy when an operator wrote `[http.client.proxy]` and never otherwise, so the block is `System`
/// down to every key of it.
///
/// **The ground is not the pool's**, which is what makes this worth a case beside the two above.
/// The caps are `System` because they bound a core's memory; this block is `System` because where
/// every outbound byte goes is a deployment's decision, and a per-call spelling would be a per-call
/// way to narrow `rule:security/net-address-policy` — the widening that rule exists to refuse. A
/// registry reading one ground off the other would be right today and wrong the moment a key
/// arrives that costs no memory at all: `bypass` and `resolve` hold nothing per core.
///
/// **`Reload` is the half that separates it from the block beside it.** `[http.client.tls]` is the
/// other `System` exception under `[http.client]` and it is `Boot`, because the one `ClientConfig`
/// every session shares is built once and handed out by `Arc`, so a new snapshot has nothing to
/// apply a changed anchor set to. The proxy has no such object: the next call reads the block, and
/// the pool's key carries the proxy, so nothing the old value made can serve one. The pair is
/// asserted together because a registry that answered `Boot` here would look like consistency.
///
/// The keys are read back out of the block rather than listed, so one added to `[http.client.proxy]`
/// joins this census in the commit that adds it — including a secret's `_file` sibling, which is the
/// spelling `rule:config/a-secret-is-a-file-whose-content-is-the-value` wants an audited deployment
/// to use and is no more a request's than the inline one.
// covers: directive:http.client.proxy
#[test]
fn every_key_of_the_proxy_block_is_the_deployments_and_reaches_the_next_call() {
    let keys = keys_in("http.client.proxy");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        [
            "url",
            "resolve",
            "bypass",
            "username",
            "password",
            "password_file"
        ],
        "a key added to `[http.client.proxy]` joins this census in the commit that adds it, and \
         the secret's `_file` sibling sitting beside the inline one is what the rule's table asks \
         an audited deployment to write",
    );

    let mut asked = vec!["http.client.proxy".to_string()];
    asked.extend(keys.iter().map(|key| format!("http.client.proxy.{key}")));
    for key in asked {
        let row = governing(&key);
        assert_eq!(
            (row.key, row.class, row.apply),
            ("http.client.proxy", Class::System, Apply::Reload),
            "`{key}` resolves through `{}` to {:?}/{:?}, and every key of this block answers one \
             question — where this deployment's outbound bytes go — so a key answering it a second \
             way is a second place a call could be steered from",
            row.key,
            row.class,
            row.apply,
        );
        assert!(
            !row.class.settable_by_a_request(),
            "`Core\\Config::set(\"{key}\", …)` has to refuse: a per-call proxy is a per-call way \
             to choose who resolves the destination, which is a per-call way to narrow \
             `rule:security/net-address-policy`",
        );
    }

    // The block beside it, which shares the class and not the apply. Asserted here because `Boot`
    // for the proxy would read as the consistent answer and would mean a changed proxy waiting for
    // a restart that the one thing holding the old value — the pool's key — does not need.
    let tls = governing("http.client.tls.roots");
    assert_eq!(
        (tls.key, tls.class, tls.apply),
        ("http.client.tls", Class::System, Apply::Boot),
        "`[http.client.tls]` settles the one `ClientConfig` the process shares by `Arc`, so a new \
         snapshot has nothing to apply a changed anchor set to — the proxy is the same class for a \
         different reason and reaches the next call without one",
    );
}

/// The one block under `[http.client]` that is **not** an exception, and the row that says so.
///
/// `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap` puts
/// `max_message` and `send_timeout` on `[http.client] deadline`'s footing: each bounds one call, a
/// program that knows its own peer names its own value at the call site, and neither is a resource
/// one request could spend on another's behalf. The `http` blanket already answers `Runtime` for
/// them, so the row restates an answer the registry would give anyway — which is the point. It sits
/// after three `System` blocks, and a block in that position is where the next reader assumes the
/// exception carries on.
///
/// So the sweep is over `[http.client]`'s sub-blocks rather than over this one alone: of the three
/// that carry a row, exactly one is settable by a request, and the case reads that out of the
/// registry rather than asserting it of the block it already named. A fourth block added under
/// `[http.client]` lands on one side of that line in the commit that adds it.
// covers: directive:http.client.socket
#[test]
fn the_socket_block_is_the_one_under_http_client_a_request_may_still_set() {
    let keys = keys_in("http.client.socket");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["max_message", "send_timeout"],
        "a key added to `[http.client.socket]` joins this census in the commit that adds it, and \
         both of these bound the one socket the call opened",
    );

    for key in &keys {
        let dotted = format!("http.client.socket.{key}");
        let row = governing(&dotted);
        assert_eq!(
            (row.key, row.class, row.apply),
            ("http.client.socket", Class::Runtime, Apply::Reload),
            "`{dotted}` resolves through `{}` to {:?}, and a bound over one call is that call's to \
             name — a row here reading `System` would refuse a program the one spelling the rule \
             gives it, `maxMessage` and `sendTimeout` at the call site",
            row.key,
            row.class,
        );
        assert!(
            row.class.settable_by_a_request(),
            "`{dotted}` is what a program that knows its own peer writes for itself, and neither \
             value is held past the socket it was written for",
        );
    }

    // Every row carved under `[http.client]`, read out of the registry: the line through them is
    // what this one exists to keep visible, and a row added there lands on one side of it in the
    // commit that adds it rather than by inheriting whatever its neighbour answered.
    let mut carved: Vec<(&str, bool)> = DIRECTIVES
        .iter()
        .filter(|row| row.key.starts_with("http.client."))
        .map(|row| (row.key, row.class.settable_by_a_request()))
        .collect();
    carved.sort_unstable();
    assert_eq!(
        carved,
        [
            ("http.client.pool_idle", false),
            ("http.client.pool_idle_timeout", false),
            ("http.client.proxy", false),
            ("http.client.socket", true),
            ("http.client.tls", false),
        ],
        "of the rows carved under `[http.client]`, exactly one is still the request's — and it is \
         the last of them, which is why it carries a row restating an answer the blanket already \
         gives rather than letting a reader carry four exceptions on to a fifth",
    );
}

/// `rule:config/three-changeability-classes` names a response header as its counter-example to `System`, and `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s policy
/// blocks are what that names: a request may set any of them for itself, because it could already
/// write the header directly. The registry states that as the one `http` row covering the whole
/// block, so the claim holds only through the longest-prefix rule — a later, more specific row
/// under `[http]` would take a key back out of `Runtime` without failing anything else here.
///
/// The keys come from `keys_in`, so this asserts the class **every** key of those blocks resolves
/// to rather than the class of the ones somebody listed.
#[test]
fn every_http_response_directive_is_runtime_class() {
    // `rule:http-server/secure-headers-with-nothing-written`, `rule:http-server/cors-is-closed-until-origins-are-named` and `rule:http-server/cookies-are-secure-httponly-and-lax`, the blocks a *response* reads. `[http.client]` (§ 5) is the outbound
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

/// The code a `[http.client.socket]` block spelling `written` is refused with, and `None` for one
/// this boot accepts.
///
/// The block goes through the same `file::parse` every other case here uses and then through
/// `http::validate`, which is the function a boot calls: what is asserted below is the refusal an
/// operator gets, not a helper's return value.
fn socket_refusal(written: &str) -> Option<Code> {
    let text = format!("[http.client.socket]\n{written}\n");
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", &text);
    let config = parsed.unwrap_or_else(|err| panic!("{text}-- did not parse: {}", err.message));
    nvs_config::http::validate(&config, &BTreeMap::new())
        .err()
        .map(|refused| refused.code.expect("a boot refusal carries its code"))
}

/// `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`: the one
/// block inside `[http.client]` that is not a `System` exception, and the one whose keys have no
/// spelling for removing themselves.
///
/// Both halves in one case because either alone reads as the whole claim and is not. A `Runtime`
/// key an operator may write as `false` hands a *request* a bound with nothing in it, which is
/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` reached the long way round; a
/// bounded key that resolved to `System` is a program that cannot name the message size it knows
/// its own peer sends, which is the whole reason the block is not in `[http.client.tls]`'s company.
///
/// The keys come from `keys_in`, so the class half is asserted of **every** key the block accepts,
/// and the bounds half is counted rather than read off a line: a check that grew a hole still
/// answers plausibly for every value that is not in it.
#[test]
fn http_client_socket_max_message_and_send_timeout_are_bounded_and_runtime_class() {
    for key in keys_in("http.client.socket") {
        let dotted = format!("http.client.socket.{key}");
        let row = governing(&dotted);
        assert_eq!(
            row.class,
            Class::Runtime,
            "`{dotted}` resolves through `{}` to {:?}, and each of this block's keys bounds one \
             call the way `[http.client] deadline` does — a program that knows its own peer names \
             its own value at the call site, and spends nothing another request then goes without",
            row.key,
            row.class,
        );
        assert_eq!(
            row.apply,
            Apply::Reload,
            "`{dotted}` is read out of the snapshot when a socket opens, so a new snapshot applies \
             it (`rule:config/reloadability-is-its-own-field`)",
        );
    }

    assert_eq!(
        socket_refusal("max_message = 4194304\nsend_timeout = \"30s\""),
        None,
        "the shipped pair is what a deployment inherits, so writing it back is never a refusal",
    );

    let no_bound = [
        "max_message = false",
        "max_message = 0",
        "send_timeout = false",
        "send_timeout = 0",
        "send_timeout = \"0s\"",
    ];
    let accepted: Vec<&str> = no_bound
        .iter()
        .copied()
        .filter(|written| socket_refusal(written).is_none())
        .collect();
    assert!(
        accepted.is_empty(),
        "a socket reassembles a message into the opening task's memory and waits to write one, so \
         neither bound has a spelling that removes it — and these were accepted: {accepted:?}",
    );
    for written in no_bound {
        assert_eq!(
            socket_refusal(written),
            Some(code::E_SOCKET_BOUND_REMOVED),
            "`{written}` is a bound with nothing in it, which is the block's own refusal rather \
             than the value band's: what is wrong is not how the number is spelled",
        );
    }
}

/// [ADR 0154] § 5: `[queue]` is in the table, key by key. The keys come from `keys_in`, so the
/// claim is about **every** key the block accepts rather than about the ones listed here — a fifth
/// key added to `[queue]` with no row of its own fails this rather than reaching `lookup` and
/// getting `None`, which is what a reload that changes a key it cannot apply and says nothing looks
/// like from the outside.
///
/// Each key answers with its *own* row and not through a prefix: the block has no `queue` row,
/// because its four keys are not one apply class (`connection_and_workers_are_boot_…` below).
///
/// [ADR 0154]: ../../../docs/decisions/0154.md
#[test]
fn every_queue_key_has_a_directive_row_and_lookup_answers_for_all_four() {
    let keys = keys_in("queue");
    assert_eq!(
        keys.len(),
        4,
        "`[queue]` accepts {keys:?}, and ADR 0154 § 5's table has four rows: a key added to the \
         block joins the table in the commit that adds it",
    );

    for key in keys {
        let dotted = format!("queue.{key}");
        let row = governing(&dotted);
        assert_eq!(
            row.key, dotted,
            "`{dotted}` resolves through `{}` rather than through a row of its own, and the four \
             keys of `[queue]` are not one apply class",
            row.key,
        );
    }
}

/// The split ADR 0154 § 5 states, which is the whole reason `[queue]` is four rows and not one:
/// `connection` and `workers` are what a worker is built out of, and the other two are read per job
/// out of the snapshot. Asserted on both sides, because a table that made the whole block `Boot`
/// would make a `max_attempts` an operator changed need a restart, and one that made it all
/// `Reload` would let a `workers` change look applied while no task was started or stopped.
// covers: directive:queue.connection, directive:queue.workers, directive:queue.max_attempts, directive:queue.visibility
#[test]
fn connection_and_workers_are_boot_and_max_attempts_and_visibility_are_reload() {
    for key in ["queue.connection", "queue.workers"] {
        assert_eq!(
            governing(key).apply,
            Apply::Boot,
            "`{key}` is what a worker is built out of: applying a change starts or stops tasks, or \
             strands every claim in flight (`rule:config/reloadability-is-its-own-field`)",
        );
    }
    for key in ["queue.max_attempts", "queue.visibility"] {
        assert_eq!(
            governing(key).apply,
            Apply::Reload,
            "`{key}` is read per job out of the snapshot, so applying it re-creates nothing \
             (`rule:config/reloadability-is-its-own-field`)",
        );
    }
}

/// `rule:core-classes/queue-storage-is-a-table` puts the jobs in a connection the operator names, so every key of the block is
/// `System`: work a request could redirect is work a request could redirect into a database it was
/// never granted. Asked of the *rows* rather than of a list of keys, so a fifth row added under
/// `[queue]` at any class fails here as well.
#[test]
fn every_queue_row_is_system_class() {
    let rows: Vec<&Directive> = DIRECTIVES
        .iter()
        .filter(|row| row.key == "queue" || row.key.starts_with("queue."))
        .collect();
    assert_eq!(
        rows.len(),
        4,
        "ADR 0154 § 5's table has four rows: {rows:?}"
    );

    for row in rows {
        assert_eq!(
            row.class,
            Class::System,
            "`{}` is `System` — the queue is armed at boot and a request may not move it \
             (`rule:core-classes/queue-storage-is-a-table`)",
            row.key,
        );
        assert!(!row.class.settable_by_a_request());
    }
}

/// The shipped default file against the reader that refuses an unknown key
/// (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`).
///
/// What this pins is that the bytes a project command writes can never themselves be the reason a
/// boot refuses. It resolves to [`Config::default`] because every key in the file is commented out,
/// which is the property `rule:config/no-configuration-file-is-a-complete-configuration` rests the
/// whole write on: taking the file changes nothing about the run that took it.
#[test]
fn the_default_file_parses_with_deny_unknown_fields() {
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(
        &mut sources,
        "crates/nvs-config/src/default.toml",
        nvs_config::default_file(),
    );

    let config =
        parsed.unwrap_or_else(|err| panic!("the shipped file was refused: {}", err.message));
    assert_eq!(
        config,
        Config::default(),
        "a live key in the shipped file is a value every deployment that takes it inherits",
    );
}

/// Whether a `#` line is a setting rather than the prose above one.
///
/// The `#` with nothing between it and the key is the whole difference, which is why every prose
/// line in that file opens `# ` and every commented-out key opens `#key`. `tools/directives.py`
/// draws the line in the same place.
fn is_setting(after_hash: &str) -> bool {
    let Some((key, _)) = after_hash.split_once('=') else {
        return false;
    };
    !after_hash.starts_with(' ')
        && !key.is_empty()
        && key.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && key
            .trim_end()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Every setting the default file spells, as the block it is written under, its own name, and the
/// prose block standing above it.
///
/// A blank line or a block header ends a prose block, so what comes back beside a key is what an
/// operator reads immediately before writing it — which is the only place a warning about the key
/// is any use. The header is carried along because the block and the name together are the *dotted*
/// key, which `tools/directives.py` otherwise has to walk the field graph to learn.
fn settings_with_prose() -> Vec<(String, String, String)> {
    let mut found = Vec::new();
    let mut prose = String::new();
    let mut block = String::new();
    for line in nvs_config::default_file().lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('#') else {
            prose.clear();
            continue;
        };
        if rest.starts_with('[') {
            block = rest.trim_matches(|c| c == '[' || c == ']').to_string();
            prose.clear();
            continue;
        }
        if is_setting(rest) {
            let (key, _) = rest.split_once('=').expect("a setting carries its `=`");
            found.push((block.clone(), key.trim().to_string(), prose.clone()));
        } else {
            prose.push(' ');
            prose.push_str(rest.trim());
        }
    }
    found
}

/// The two slots a doc comment's `[unread:]` trailer declares, for a comment that carries one:
/// *why* nothing reads the key yet, and *who* closes the gap.
///
/// A trailer that opens and never reaches `owner:` comes back with that slot empty rather than as
/// no trailer at all, so a malformed declaration is reported below as the declaration it is instead
/// of being passed over as a field that declared nothing.
fn unread_trailer(doc: &str) -> Option<(String, String)> {
    let (body, _) = doc.split_once("[unread:")?.1.split_once(']')?;
    Some(match body.split_once("owner:") {
        Some((why, owner)) => (why.trim().to_string(), owner.trim().to_string()),
        None => (body.trim().to_string(), String::new()),
    })
}

/// Whether an owner names something a reader can go and open: a rule id, or the four-digit number
/// of a decision record.
///
/// Those are the two things in this repository that own a gap. A name, a crate path or a sentence
/// is a dead end for the operator who followed a `NOT IMPLEMENTED` note here from the default file.
fn names_an_owner(owner: &str) -> bool {
    if let Some(id) = owner.strip_prefix("rule:") {
        return match id.split_once('/') {
            Some((topic, slug)) => {
                !topic.is_empty() && !slug.is_empty() && !id.contains(char::is_whitespace)
            }
            None => false,
        };
    }
    owner.len() == 4 && owner.chars().all(|c| c.is_ascii_digit())
}

/// Every field the tree declares unread, paired with the two slots its declaration names.
///
/// Read out of `tree.rs`'s own text rather than listed here, so that a key gaining a trailer joins
/// this case in the commit that declares it and a key losing one leaves. Which *dotted* key a field
/// is belongs to `tools/directives.py`, which walks the field graph a string scan cannot see; what
/// is asserted below is the declaration the file owes either way.
fn declared_unread() -> Vec<(String, String, String)> {
    const TREE: &str = include_str!("../src/tree.rs");

    let mut found = Vec::new();
    let mut doc = String::new();
    for line in TREE.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("///") {
            doc.push(' ');
            doc.push_str(rest.trim());
            continue;
        }
        if line.starts_with("#[") {
            continue;
        }
        if let Some((field, _)) = line
            .strip_prefix("pub ")
            .and_then(|rest| rest.split_once(':'))
            && let Some((why, owner)) = unread_trailer(&doc)
        {
            found.push((
                field.trim().trim_start_matches("r#").to_string(),
                why,
                owner,
            ));
        }
        doc.clear();
    }
    found
}

/// `rule:config/no-configuration-file-is-a-complete-configuration` read at its worst case: a key the
/// file offers, the boot accepts, and nothing acts on.
///
/// An operator who writes such a key has configured nothing and has no way to find that out, which
/// is worse than the key not being there at all — so the tree's own `[unread:]` declaration has to
/// reach the line the operator is about to write.
///
/// Both directions, because each is a way the file rots and the second is what keeps the first
/// honest: a scan that found no declaration at all would satisfy the loop below and fail the loop
/// after it. A note that outlives the gap it describes is the same defect read backwards — it tells
/// an operator not to write a key that now works.
#[test]
fn every_unimplemented_key_in_the_default_file_is_marked_as_one() {
    let settings = settings_with_prose();
    let unread = declared_unread();

    for (field, _, owner) in &unread {
        assert!(
            settings.iter().any(|(_, key, prose)| key == field
                && prose.contains("NOT IMPLEMENTED")
                && prose.contains(owner)),
            "`{field}` is declared unread in the tree, and no `{field} = ` line in the default \
             file stands under a `NOT IMPLEMENTED` note naming `{owner}`",
        );
    }

    for (_, key, prose) in settings
        .iter()
        .filter(|(_, _, p)| p.contains("NOT IMPLEMENTED"))
    {
        assert!(
            unread
                .iter()
                .any(|(field, _, owner)| field == key && prose.contains(owner)),
            "the default file marks `{key}` unimplemented and `crates/nvs-config/src/tree.rs` \
             declares no `[unread:]` for it",
        );
    }
}

/// The roster half of `rule:config/three-changeability-classes`, asked of the declaration rather
/// than of its presence: a trailer that names nothing is the same silence as no trailer at all.
///
/// `tools/directives.py`'s gate reads a trailer that *parses* — an `[unread:]` with anything at all
/// in its two slots — and a regex cannot go further than that. What it cannot ask is whether the two
/// slots say something. The *why* is what a later session reads instead of re-deriving the gap from
/// the absence of a reader, and the *owner* is where the operator who followed a `NOT IMPLEMENTED`
/// note out of the default file arrives, so it has to be a thing that can be opened: a rule id or a
/// decision record's number.
///
/// Whether a rule an owner names still exists is `python tools/rules.py --check`'s question, asked
/// of every `rule:` citation in the repository rather than of these alone.
#[test]
fn every_unread_key_names_what_is_missing_and_who_owns_it() {
    for (field, why, owner) in declared_unread() {
        assert!(
            !why.is_empty(),
            "`{field}`'s `[unread:]` trailer declares no reason, so the tree records that the key \
             reaches nothing and not what is missing: `[unread: <why> owner: <who>]`",
        );
        assert!(
            names_an_owner(&owner),
            "`{field}` is owned by `{owner}`, which is neither a `rule:<topic>/<slug>` id nor a \
             four-digit record number — an operator sent here by the default file's \
             `NOT IMPLEMENTED` note has nothing to open",
        );
    }
}

/// The dotted key of every setting the default file marks unimplemented.
///
/// The block header and the setting name are the dotted key, so the file hands this case what
/// `tools/directives.py` walks the field graph to derive — and the pair of cases above holds that
/// set equal to the tree's own `[unread:]` trailers, so it is the roster read off the artifact an
/// operator reads.
fn keys_marked_unimplemented() -> Vec<String> {
    settings_with_prose()
        .iter()
        .filter(|(_, _, prose)| prose.contains("NOT IMPLEMENTED"))
        .map(|(block, key, _)| {
            if block.is_empty() {
                key.clone()
            } else {
                format!("{block}.{key}")
            }
        })
        .collect()
}

/// `tree.rs` is the roster itself, and the directive and capability registries name every key
/// without reading one, so a match in any of the three is a classification rather than a read
/// (`tools/directives.py`'s module doc, § *A registry row names a key*).
const NOT_READERS: [&str; 3] = [
    "crates/nvs-config/src/tree.rs",
    "crates/nvs-config/src/directive.rs",
    "crates/nvs-config/src/capability.rs",
];

/// Every `.rs` file the workspace compiles into a crate or a bench, as its path and its code.
///
/// A whole-line comment is dropped and the file is cut at its first `#[cfg(test)]`, so neither the
/// prose that cites a key nor a unit test that round-trips one reads as a reader of it.
fn workspace_source() -> Vec<(String, String)> {
    let mut found = Vec::new();
    for group in ["crates", "benches"] {
        let base = nvs_repo::path(group);
        let Ok(entries) = fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.flatten() {
            collect_source(&entry.path().join("src"), (group, &base), &mut found);
        }
    }
    found
}

/// One directory of [`workspace_source`]'s walk, and every directory under it. `group` is the
/// top-level directory the walk started in, as its name and its path, and a file is named from it.
fn collect_source(dir: &Path, group: (&str, &Path), found: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_source(&path, group, found);
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let (name, base) = group;
        let inside = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");
        let rel = format!("{name}/{inside}");
        if NOT_READERS.contains(&rel.as_str()) {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("`{rel}`: {err}"));
        let code = text
            .lines()
            .take_while(|line| line.trim() != "#[cfg(test)]")
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        found.push((rel, code));
    }
}

/// The first file that spells `dotted` as a string literal, which is a read of that key by name.
fn reads_the_key<'a>(source: &'a [(String, String)], dotted: &str) -> Option<&'a str> {
    let literal = format!("\"{dotted}\"");
    source
        .iter()
        .find(|(_, code)| code.contains(&literal))
        .map(|(rel, _)| rel.as_str())
}

/// The other direction of `rule:config/three-changeability-classes`'s roster, and the worse of the
/// two: a note that outlives its gap tells an operator not to write a key that now works.
///
/// **This is the narrower of the two claims the name allows, and deliberately.** A reader is counted
/// three ways — the typed field, the dotted key as a literal, and a bare name that is unique in the
/// roster — and only the second is a spelling a scan can answer exactly. The field name is not the
/// key: `[metrics] listen` and `[debug] mode` share their names with `capabilities.net.listen`,
/// `server.listen` and `[app] mode`, so a scan for a field access answers about whichever field it
/// found and fails for a reason that has nothing to do with the key. So what is asserted here is one
/// sound half — no key the default file marks unimplemented is read anywhere by name — and the
/// census over all three spellings stays `tools/directives.py --check`'s, which `tools/verify.py`
/// runs. The half asserted here is the one a field search cannot see, which is why it is worth
/// having twice: `Core\Storage` reaches its disk root as `config.get("storage.{disk}.root")`.
#[test]
fn no_key_with_a_reader_still_claims_to_be_unread() {
    let source = workspace_source();
    let marked = keys_marked_unimplemented();
    assert!(
        !source.is_empty() && !marked.is_empty(),
        "the walk found {} file(s) and {} marked key(s), so this case is asserting nothing",
        source.len(),
        marked.len(),
    );

    for dotted in &marked {
        assert!(
            reads_the_key(&source, dotted).is_none(),
            "`{dotted}` is marked `NOT IMPLEMENTED` in `crates/nvs-config/src/default.toml` and \
             `{}` reads it by name. Delete the note and the field's `[unread:]` trailer in \
             `crates/nvs-config/src/tree.rs` — a file that marks a live key unimplemented is worse \
             than one that omits it.",
            reads_the_key(&source, dotted).unwrap_or_default(),
        );
    }

    // The matcher against a key that *is* read, so a walk that silently collected nothing readable
    // — a moved crate root, a strip that ate every line — fails here rather than passing the loop
    // above by finding nothing anywhere.
    let every_key: Vec<String> = settings_with_prose()
        .iter()
        .map(|(block, key, _)| format!("{block}.{key}"))
        .collect();
    assert!(
        every_key
            .iter()
            .any(|key| reads_the_key(&source, key).is_some()),
        "no key of the default file is spelled as a literal anywhere in the workspace, so the \
         source walk is reading something other than this workspace's crates",
    );
}

/// `rule:config/include-takes-a-path-or-a-dir` gives an entry its key set, and
/// `rule:config/any-file-in-the-tree-may-set-any-directive` lets an included file say anything the
/// root file can — so the whole array of tables is one `System` row and every key of an entry has
/// to resolve through it. The failure this stands in the way of is a `path` or a `dir` that
/// `Core\Config::set` answers for want of a row: a request naming a file the server reads its
/// directives out of has chosen every capability grant and every ceiling in it, out of a file no
/// account ever had to own in order to write.
// covers: directive:include
#[test]
fn every_key_of_an_include_entry_resolves_through_the_one_system_row() {
    let keys = keys_listed("[[include]]\nnvs_no_such_key = true\n");
    assert_eq!(
        keys.iter().map(String::as_str).collect::<Vec<_>>(),
        ["path", "dir", "optional"],
        "a key added to an `[[include]]` entry joins this census in the commit that adds it, and \
         each one is part of naming a file the server reads configuration out of",
    );

    let mut asked = vec!["include".to_string()];
    asked.extend(keys.iter().map(|key| format!("include.{key}")));
    for key in asked {
        let row = governing(&key);
        assert_eq!(
            (row.key, row.class, row.apply),
            ("include", Class::System, Apply::Reload),
            "`{key}` resolves through `{}`, and the `include` row is the only one that may answer \
             for it: `Runtime` would hand a request the whole configuration, and `Boot` would say \
             a file dropped into a `dir` entry needs a restart when the next reload reads it",
            row.key,
        );
        assert!(
            !row.class.settable_by_a_request(),
            "`Core\\Config::set(\"{key}\", …)` has to refuse — an include is trusted because \
             whoever wrote it cleared `rule:config/ownership-is-the-trust-boundary`, and a request \
             clears nothing",
        );
    }
}

/// `rule:core-classes/temporary-dir-sweep` rests the whole safety of its two sweeps on the runtime
/// owning the temporary root outright, so the root is the operator's (`System`) and a new one is a
/// restart (`Boot`) — the orphan sweep runs once as `nvs serve` boots, over the root it started
/// with. Asserted beside `[debug] keep_temporary`, which is the other way to the same place: a
/// program able to exempt its own directories from the sweep can be made to hoard them. The block
/// carries exactly one row, so the second half of this is that nothing blankets `[io]`: a key added
/// there is refused for want of a row rather than inheriting this one's class.
// covers: directive:io.temp_root
#[test]
fn the_temporary_root_is_the_operators_and_a_new_one_is_a_restart() {
    let root = governing("io.temp_root");
    assert_eq!(
        (root.key, root.class, root.apply),
        ("io.temp_root", Class::System, Apply::Boot),
        "`io.temp_root` resolves through `{}` to {:?}/{:?}, and each half being wrong is its own \
         failure: `Runtime` would let a request choose where every other request's scratch files \
         land, and `Reload` would promise a swapped root that the boot sweep never ran over",
        root.key,
        root.class,
        root.apply,
    );

    for absent in ["io", "io.scratch_root", "io.temp"] {
        assert!(
            lookup(absent).is_none(),
            "`{absent}` is governed by a row, so something blankets `[io]` — a second key in that \
             block has to be refused for want of a row rather than inheriting the root's class",
        );
    }

    let keep = governing("debug.keep_temporary");
    assert_eq!(
        (keep.key, keep.class),
        ("debug.keep_temporary", Class::System),
        "`{}` governs the key that turns the per-script sweep off, and it is the same class as the \
         root for the same reason — both are the operator deciding what happens to scratch files \
         that are not the program's to decide about",
        keep.key,
    );
    for row in [root, keep] {
        assert!(
            !row.class.settable_by_a_request(),
            "`Core\\Config::set(\"{}\", …)` has to refuse: `rule:core-classes/temporary-dir-sweep` \
             says there is no in-language setter for either half of this, because a program that \
             can exempt its own files can be made to hoard them",
            row.key,
        );
    }
}

/// `rule:config/three-changeability-classes`: `[limits]` is the block a request *may* write, and the
/// whole of its safety is that `[limits.hard]` holds the same key names under a different class.
/// Pinned as the pair, because either row alone still reads right — a `[limits]` gone `System` takes
/// away the per-request default the block exists to be, and a `[limits.hard]` gone `Runtime` lets a
/// request raise the ceiling it is being bounded by. The sweep underneath is what a new key lands
/// on: every key of the block that no row carves out has to reach the `Runtime` row, and every row
/// that does carve one out has to be a class a request cannot write.
// covers: directive:limits
#[test]
fn the_default_limits_are_the_requests_and_everything_carved_out_of_them_is_not() {
    let soft = governing("limits.memory");
    assert_eq!(
        (soft.key, soft.class, soft.apply),
        ("limits", Class::Runtime, Apply::Reload),
        "`limits.memory` resolves through `{}`, and it has to be the `limits` row at `Runtime`: \
         this block is the per-request default a program lowers for itself, which is the one thing \
         `rule:config/three-changeability-classes` names it as",
        soft.key,
    );
    let hard = governing("limits.hard.memory");
    assert_eq!(
        (hard.key, hard.class, hard.apply),
        ("limits.hard", Class::System, Apply::Reload),
        "`limits.hard.memory` resolves through `{}`, and the longest-row lookup is the only thing \
         keeping it off the `Runtime` row above — a request that reached that row would be raising \
         the ceiling it is bounded by, one key name apart from lowering its own default",
        hard.key,
    );
    assert!(
        soft.class.settable_by_a_request() && !hard.class.settable_by_a_request(),
        "the pair answers the same way to `Core\\Config::set`, so the two halves of `[limits]` have \
         collapsed into one class and the block is either a ceiling nobody can use or a ceiling \
         anybody can move",
    );

    let carved: BTreeMap<&str, Class> = DIRECTIVES
        .iter()
        .filter(|row| row.key.starts_with("limits."))
        .map(|row| (row.key, row.class))
        .collect();
    assert!(
        carved.len() > 1,
        "only `{:?}` is carved out of `[limits]`, so the reserve keys and the recursion ceiling \
         `rule:errors/on-limit` puts in the operator's hands have gone back to being a request's",
        carved.keys().collect::<Vec<_>>(),
    );
    for (key, class) in &carved {
        assert!(
            !class.settable_by_a_request(),
            "`{key}` is carved out of `[limits]` at {class:?}, which a request can write — a row \
             here exists to take a key *away* from the block's `Runtime` blanket, so a settable \
             one is a longer row that changed nothing but the registry's line count",
        );
    }

    for key in keys_in("limits") {
        let dotted = format!("limits.{key}");
        if carved.contains_key(dotted.as_str()) {
            continue;
        }
        let row = governing(&dotted);
        assert_eq!(
            (row.key, row.class),
            ("limits", Class::Runtime),
            "`{dotted}` is a key of `[limits]` that no row carves out, so it has to land on the \
             block's own row and did not — a key added here is a per-request default until \
             somebody decides otherwise, and `{}` is not that decision",
            row.key,
        );
    }
}

/// `rule:config/three-changeability-classes`'s ceiling, asserted as the agreement it has to be with
/// the block underneath it: `[limits.hard]` holds the same key names as `[limits]`, and the pair
/// answers `Core\Config::set` differently for every one of them. Asked of every key rather than
/// written out for `memory`, because a ceiling that governs the one key somebody tested and
/// blankets nothing else reads exactly right on that line — and the budget it quietly stopped
/// covering is one a request may then raise as far as it likes.
// covers: directive:limits.hard
#[test]
fn every_budget_with_a_ceiling_answers_the_request_and_the_operator_differently() {
    let ceiling = governing("limits.hard");
    assert_eq!(
        (ceiling.key, ceiling.class, ceiling.apply),
        ("limits.hard", Class::System, Apply::Reload),
        "`limits.hard` resolves through `{}` to {:?}/{:?}: `Runtime` would be a request raising \
         the ceiling that bounds it, and `Boot` would tell an operator that lowering a ceiling \
         under a running server takes a restart when the next request reads the new snapshot",
        ceiling.key,
        ceiling.class,
        ceiling.apply,
    );

    // A ceiling exists for exactly the budgets a request may set, and for nothing else: the keys
    // `[limits]` carves out with a row of its own are already the operator's, so a second operator
    // key bounding them would be a ceiling over a value no request can move.
    let carved: Vec<&str> = DIRECTIVES
        .iter()
        .filter(|row| row.key.starts_with("limits."))
        .map(|row| row.key)
        .collect();
    let keys = keys_in("limits.hard");
    let settable: Vec<String> = keys_in("limits")
        .into_iter()
        .filter(|key| !carved.contains(&format!("limits.{key}").as_str()))
        .collect();
    assert_eq!(
        keys, settable,
        "`[limits.hard]` bounds a set of keys that is no longer the set a request may write. A key \
         missing from the ceiling block is a budget a program may raise as far as it likes; an \
         extra one is a ceiling over a value no request can move in the first place",
    );

    let mut compared = 0;
    for key in &keys {
        let under = format!("limits.hard.{key}");
        let row = governing(&under);
        assert_eq!(
            (row.key, row.class),
            ("limits.hard", Class::System),
            "`{under}` lands on `{}` rather than on the ceiling row, so this budget's ceiling is \
             something a request can reach — one key name away from the default it is supposed to \
             be bounding",
            row.key,
        );

        let twin = format!("limits.{key}");
        if carved.contains(&twin.as_str()) {
            continue;
        }
        assert!(
            governing(&twin).class.settable_by_a_request() && !row.class.settable_by_a_request(),
            "`{twin}` and `{under}` answer `Core\\Config::set` the same way, so this budget has no \
             ceiling worth the name: either the program cannot lower its own or it can raise the \
             operator's",
        );
        compared += 1;
    }
    assert!(
        compared > 0,
        "every key of `[limits]` is carved out by a row of its own, so `[limits.hard]` bounds \
         nothing a request is still allowed to set and the pair has stopped being a pair",
    );
}

/// `rule:errors/on-limit`'s reserved slice, asserted as the carve-out it is. The memory half is
/// written inside `[limits]`, whose blanket row is `Runtime`, so the only thing keeping a script
/// from sizing its own safety net is a longer row — and the only thing keeping it in the operator's
/// hands afterwards is that nothing in `[limits.hard]` bounds it, which is an absence rather than an
/// omission: a ceiling exists to bound a value a request can move, and this is not one.
///
/// The unit is the third assertion and the one a reader would not think to make. The slice is
/// subtracted from `[limits] memory` at request start, so the two have to be the same quantity; a
/// reserve that parsed `"2M"` as anything but octets would be arithmetic between two units, and it
/// would look right on every line of the registry.
// covers: directive:limits.fatal_reserve_memory
#[test]
fn the_reserved_slice_is_carved_out_of_the_block_and_has_no_ceiling_of_its_own() {
    let reserve = governing("limits.fatal_reserve_memory");
    assert_eq!(
        (reserve.key, reserve.class, reserve.apply),
        ("limits.fatal_reserve_memory", Class::System, Apply::Reload),
        "`limits.fatal_reserve_memory` resolves through `{}`: the blanket row above it is \
         `Runtime`, so a slice that lands there is a script choosing how much room the handler \
         reporting its exhausted heap gets — and `Boot` would tell an operator that resizing the \
         net takes a restart, when the next request reads the new snapshot",
        reserve.key,
    );
    assert!(
        governing("limits.memory").class.settable_by_a_request()
            && !reserve.class.settable_by_a_request(),
        "the slice and the budget it is carved out of answer `Core\\Config::set` the same way, so \
         either a program can no longer lower its own heap or it can set the size of its own \
         safety net",
    );

    assert!(
        !keys_in("limits.hard").contains(&"fatal_reserve_memory".to_string()),
        "`[limits.hard]` has grown a ceiling over the reserved slice, which is a ceiling over a \
         value no request can move: `rule:errors/on-limit` puts the sizing in the operator's hands \
         already, so a second operator key bounding the first says nothing and reads as though the \
         slice were a request's",
    );

    assert_eq!(
        (
            unit_of("limits.fatal_reserve_memory"),
            unit_of("limits.memory"),
        ),
        (Some(Unit::Bytes), Some(Unit::Bytes)),
        "the slice is read in a different unit from the heap it is subtracted from, so `\"2M\"` \
         written here and `\"2M\"` written there are two different quantities",
    );
}

/// The time half of the same slice, asserted as the agreement it has to be with the memory half.
/// `rule:errors/on-limit` makes them one net — two reserves and not one per limit, because those
/// are the only two resources a handler cannot run without spending — so every registry answer
/// about one is an answer about the other, and a net with one half in the operator's hands and one
/// half in the program's is not a net.
///
/// The unit is where the two are *required* to differ, and asserting that beside the agreement is
/// what stops this case from passing over a pair that had collapsed into one quantity: a reserve of
/// octets carved out of a CPU ceiling would leave `[limits] cpu_time` with no slice at all and
/// nothing in the registry looking wrong.
// covers: directive:limits.fatal_reserve_time
#[test]
fn the_time_half_of_the_reserve_answers_exactly_as_the_memory_half_does() {
    let time = governing("limits.fatal_reserve_time");
    let memory = governing("limits.fatal_reserve_memory");
    assert_eq!(
        (time.key, time.class, time.apply),
        ("limits.fatal_reserve_time", Class::System, Apply::Reload),
        "`limits.fatal_reserve_time` resolves through `{}`, and a row of its own is what it needs: \
         the memory half is not a prefix of it, so nothing else would keep it out of `[limits]`' \
         `Runtime` blanket",
        time.key,
    );
    assert_eq!(
        (time.class, time.apply),
        (memory.class, memory.apply),
        "the two halves of one net answer differently, so `rule:errors/on-limit`'s decision has \
         been made twice and the halves have stopped being the same slice",
    );
    assert!(
        !time.class.settable_by_a_request() && !memory.class.settable_by_a_request(),
        "one half of the reserve is a request's to set, which is the whole of the refusal gone: a \
         handler with no clock and a handler with no heap are the same handler",
    );
    assert!(
        !keys_in("limits.hard").contains(&"fatal_reserve_time".to_string()),
        "`[limits.hard]` has grown a ceiling over the time half, for the reason its memory half's \
         case gives: there is no request-set value here for a ceiling to bound",
    );

    assert_eq!(
        (
            unit_of("limits.fatal_reserve_time"),
            unit_of("limits.cpu_time"),
        ),
        (Some(Unit::Duration), Some(Unit::Duration)),
        "the time half is not read as a duration, or the ceiling it is carved out of is not — \
         either way the subtraction at request start is between two different quantities",
    );
    assert_ne!(
        unit_of("limits.fatal_reserve_time"),
        unit_of("limits.fatal_reserve_memory"),
        "both halves of the net are read in one unit, so one of them is being subtracted from the \
         wrong ceiling — the halves agree on everything the registry says about them *except* what \
         they measure",
    );
}

/// The nesting ceiling, asserted as the pair of claims it is: who may write it, and what it is
/// counted in. The first is `[limits]`' `Runtime` blanket again — no other row is a prefix of this
/// key, so without one of its own the block answers for it — and the ground is the one the reserves
/// do not share: *both* directions are refused here. A chain able to raise the bound it is about to
/// cross has no bound, and the two values that remove the ceiling outright are reachable from the
/// same `set`.
///
/// The unit is the other half, and a `Count` is what keeps the two plausible mis-readings out. Read
/// as `Bytes`, `"64M"` would be sixty-seven million levels — a ceiling no chain reaches and so no
/// ceiling at all. Read as `Unit::Ratio`, whose values run between zero and one, every depth an
/// operator would write is refused instead.
// covers: directive:limits.max_script_depth
#[test]
fn the_nesting_ceiling_is_the_operators_in_both_directions_and_is_counted_in_levels() {
    let depth = governing("limits.max_script_depth");
    assert_eq!(
        (depth.key, depth.class, depth.apply),
        ("limits.max_script_depth", Class::System, Apply::Reload),
        "`limits.max_script_depth` resolves through `{}`, and a row of its own is what it needs: \
         nothing else here is a prefix of it, so `[limits]`' `Runtime` blanket is what would answer",
        depth.key,
    );
    assert!(
        keys_in("limits").contains(&"max_script_depth".to_string()),
        "`[limits]` no longer accepts `max_script_depth`, so this row governs a key the file \
         refuses and every deployment runs at the shipped depth with no way to state another",
    );
    for written in [
        "limits.max_script_depth",
        "app.limits.max_script_depth",
        "app.0.limits.max_script_depth",
        "limits.hard.max_script_depth",
    ] {
        let row = governing(written);
        assert!(
            !row.class.settable_by_a_request(),
            "`{written}` lands on `{}` at {:?}, which a request may write — one spelling of this \
             ceiling a program can reach is the ceiling gone, whichever block it is written in",
            row.key,
            row.class,
        );
    }
    assert!(
        !keys_in("limits.hard").contains(&"max_script_depth".to_string()),
        "`[limits.hard]` has grown a ceiling over the nesting depth, which has nothing to stand \
         over: there is no request-set value here for it to bound, and the reserves' case gives \
         the rest of the argument",
    );

    let quantity = |text: &str| {
        nvs_config::Quantity::parse(
            depth.key,
            Unit::Count,
            &nvs_config::Setting::Text(text.to_string()),
        )
    };
    assert_eq!(
        unit_of("limits.max_script_depth"),
        Some(Unit::Count),
        "the depth is not read as a count of levels, so the number an operator writes is measured \
         in something the chain it bounds is not",
    );
    assert!(
        matches!(quantity("64"), Ok(nvs_config::Quantity::Count(64))),
        "sixty-four levels is not sixty-four of anything this parse recognises",
    );
    assert!(
        matches!(quantity("false"), Ok(nvs_config::Quantity::Unbounded)),
        "`false` is refused or read as a magnitude, and it is the operator's one way to say that a \
         runaway recursion should arrive as an exhausted heap rather than as a depth",
    );
    assert!(
        quantity("64M").is_err(),
        "a size's suffix is read as a depth of sixty-seven million levels rather than refused, \
         which is a ceiling no chain reaches and so no ceiling at all",
    );
}

/// `rule:core-classes/decompression-bound`'s absolute half, asserted through the spellings that
/// would take it back. The key is one `[limits]` itself accepts, so the block's `Runtime` blanket
/// would answer for it with no row of its own — and `[[app]]`'s copy of the block is the second way
/// in, since `[app.limits]` is read key for key as `[limits]` is. An application handed a looser
/// decompression bound than the deployment's is the same hole one scope down.
///
/// The ground is worth stating because the wrong one has the same answer here: this is not the
/// operator's because `[limits]` is — half that block is a request's. It is the operator's because
/// a call already lowers what it decompresses through its own argument, so a request-set ceiling
/// would be a second spelling for the ask, and the only direction a `Runtime` row could be moved in
/// is the direction the rule forbids.
// covers: directive:limits.max_decompressed
#[test]
fn the_decompression_ceiling_is_the_operators_under_every_block_that_states_it() {
    let ceiling = governing("limits.max_decompressed");
    assert_eq!(
        (ceiling.key, ceiling.class, ceiling.apply),
        ("limits.max_decompressed", Class::System, Apply::Reload),
        "`limits.max_decompressed` resolves through `{}`: on the `limits` row it is a value a \
         request raises to whatever the archive it was handed asks for, and at `Boot` it is a \
         ceiling an operator cannot lower under a running server",
        ceiling.key,
    );
    assert!(
        keys_in("limits").contains(&"max_decompressed".to_string()),
        "`[limits]` no longer accepts `max_decompressed`, so this row governs a key the file \
         refuses and every deployment's bound is the shipped one with nothing able to change it",
    );
    for written in [
        "limits.max_decompressed",
        "app.limits.max_decompressed",
        "app.0.limits.max_decompressed",
        "limits.hard.max_decompressed",
    ] {
        let row = governing(written);
        assert!(
            !row.class.settable_by_a_request(),
            "`{written}` lands on `{}` at {:?}, which a request may write — one spelling of this \
             bound that a program can reach is the whole bound gone, whichever block it is written \
             in",
            row.key,
            row.class,
        );
    }

    assert_eq!(
        unit_of("limits.max_decompressed"),
        Some(Unit::Bytes),
        "the absolute half is not read as a size, so `\"64M\"` is not 64 MiB of output — a ceiling \
         on how much heap one archive may claim has to be the same quantity as the heap",
    );
}

/// The ratio half, asserted as the pair the rule says it is. What a decode is measured against is
/// `min(input × ratio, ceiling)`, so the two halves are one bound: either of them a request could
/// move is the minimum moved, and a bound with one half missing is the other half on its own —
/// an absolute ceiling every request reaches whatever it sent, or a ratio a large upload buys a
/// proportionately large output with.
///
/// The units are where the halves differ, and the difference is the content: the ratio is a
/// multiplier of output per octet of input, not a fraction of one, so `Count` is the reading and
/// `Ratio` — whose values run between zero and one — would quietly turn 1000:1 into a bound no
/// archive could pass.
// covers: directive:limits.max_decompression_ratio
#[test]
fn both_halves_of_the_decompression_bound_answer_the_same_way_in_two_different_quantities() {
    let ratio = governing("limits.max_decompression_ratio");
    let ceiling = governing("limits.max_decompressed");
    assert_eq!(
        (ratio.key, ratio.class, ratio.apply),
        (
            "limits.max_decompression_ratio",
            Class::System,
            Apply::Reload
        ),
        "`limits.max_decompression_ratio` resolves through `{}`, and a row of its own is what it \
         needs: the absolute half is not a prefix of it, so nothing else keeps it out of the \
         `Runtime` blanket `[limits]` states",
        ratio.key,
    );
    assert_eq!(
        (ratio.class, ratio.apply),
        (ceiling.class, ceiling.apply),
        "the two halves of one bound answer differently, so a deployment can state one of them and \
         a request the other — and what a decode is compared against is the smaller of the two",
    );
    assert!(
        !ratio.class.settable_by_a_request() && !ceiling.class.settable_by_a_request(),
        "one half of the bound is a request's to set, which is the bound gone: a program that \
         raises the ratio buys the same output a program that raises the ceiling does",
    );

    assert_eq!(
        (
            unit_of("limits.max_decompression_ratio"),
            unit_of("limits.max_decompressed"),
        ),
        (Some(Unit::Count), Some(Unit::Bytes)),
        "the halves are read in one quantity, or the ratio is read as `Unit::Ratio` — a multiplier \
         whose values are taken to run between zero and one is a bound every archive passes or \
         none does",
    );
}

/// The block the escalation ladder is configured in, and the one row of this group a request may
/// write. What is asserted is where the line between the two halves falls rather than the class of
/// any single key: `rule:config/a-mode-is-five-defaults`'s mode table holds two of these keys and no row in that table is
/// `System`, so a program that wants its own run traced turns its own level down — while a program
/// able to send the record of its own failure somewhere nobody reads is what every other key here
/// is `System` to prevent.
///
/// Counted rather than read off a line, because a blanket row is exactly where a registry answers
/// plausibly key by key and wrongly overall: a carve-out nobody wrote inherits `Runtime` from the
/// block and looks like an ordinary row from every angle but this one.
// covers: directive:log
#[test]
fn the_log_blanket_is_a_requests_and_exactly_the_mode_tables_two_rows_stay_that_way() {
    let block = governing("log");
    assert_eq!(
        (block.key, block.class, block.apply),
        ("log", Class::Runtime, Apply::Reload),
        "`[log]` resolves through `{}`: the block's own row is what answers for `format` and \
         `level`, which are `rule:config/a-mode-is-five-defaults`'s rows and so are nobody's to make `System`",
        block.key,
    );
    for written in ["log.format", "log.level"] {
        assert_eq!(
            governing(written).key,
            "log",
            "`{written}` has grown a row of its own, so the blanket is no longer what makes it a \
             request's — and a mode table row with a class written twice is a class written once \
             in each of two places that can disagree",
        );
    }

    let mut movable: Vec<String> = keys_in("log")
        .into_iter()
        .filter(|key| {
            governing(&format!("log.{key}"))
                .class
                .settable_by_a_request()
        })
        .collect();
    movable.sort();
    assert_eq!(
        movable,
        vec!["format".to_string(), "level".to_string()],
        "the blanket answers for a key that is not one of the mode table's two, so a rung of the \
         ladder is a request's to move: every other key in this block names the script run when a \
         request fails, what is held back for that script, or where the record itself goes",
    );

    assert!(
        lookup("format").is_none() && lookup("level").is_none(),
        "a bare `format` or `level` resolves to a row, so this block has a second spelling with no \
         block in it — a bare name is a limit's name, which `nvs_config::request`'s module doc owns, \
         and nothing outside `[limits]` gets one",
    );
}

/// The tier-3 handler: the row in `[log]` whose class had to be written down, because the block
/// answers `Runtime` for everything that does not write one. `rule:errors/handler-script` names a script to *run*, so the
/// ground is `limits.fatal_reserve_memory`'s one rung along — a program naming who reports its own
/// failure is the case where the choice most needs to be made by somebody else.
///
/// The second half is the boundary this block is looked up on, and this row is where a wrong one
/// would show. `log.handler` is a character prefix of `log.handler_reserve_memory` and not a dotted
/// one, so a lookup matching on spelling would hand both reserves this row — and would hand
/// `log.handlerx`, which nobody wrote, a class it has no claim to.
// covers: directive:log.handler
#[test]
fn the_tier_three_handler_is_the_operators_and_its_row_ends_at_the_dot() {
    let handler = governing("log.handler");
    assert_eq!(
        (handler.key, handler.class, handler.apply),
        ("log.handler", Class::System, Apply::Reload),
        "`log.handler` resolves through `{}`: with no row of its own the `[log]` blanket answers \
         `Runtime` for it, and the script that reports a failure becomes the failing program's to \
         name",
        handler.key,
    );
    assert!(
        !handler.class.settable_by_a_request() && governing("log").class.settable_by_a_request(),
        "the handler and the block around it are on the same side of `rule:config/three-changeability-classes`'s line, so one \
         of the two is written for a reason that has stopped holding",
    );
    assert_eq!(
        handler.apply,
        Apply::Reload,
        "`Boot` would mean a handler replaced under a running server is not the one the next \
         failure runs, and this path is read when a failure reaches the ladder rather than held \
         from boot",
    );

    for written in ["log.handler_reserve_memory", "log.handler_reserve_time"] {
        let row = governing(written);
        assert_eq!(
            row.key, written,
            "`{written}` is answered by `{}`, which is the spelling it shares and not the block it \
             is in — `directive::governs`' dot boundary is what keeps the two apart",
            row.key,
        );
    }
    assert_eq!(
        governing("log.handlerx").key,
        "log",
        "a key nobody wrote takes the handler's row because it starts with the same letters, which \
         is that same boundary gone in the other direction",
    );

    assert_eq!(
        unit_of("log.handler"),
        None,
        "the handler is read as a measurement, so a path is being compared against a ceiling — \
         this row's class is the whole of what stands between a program and the script that \
         reports it",
    );
    assert!(
        keys_in("log").contains(&"handler".to_string()),
        "`[log]` no longer accepts `handler`, so this row governs a key the file refuses and a \
         deployment has no way to name the script at all",
    );
}

/// The room that script runs in. Its class is the handler's for the handler's reason at one remove —
/// a program able to shrink the room its own report is written in has made the report fail rather
/// than named who writes it — and `rule:errors/handler-script` is where the allotment being the engine's rather than
/// the failing request's is decided.
///
/// What this asserts past the class is that the two reserves are two: `[limits]
/// fatal_reserve_memory` is carved out of the request's own heap for a closure it already holds,
/// and this one is the engine's for a script that has yet to be compiled. They are the same size
/// twice in a registry that lost one of the rows, and every deployment that wrote either would then
/// be writing both.
// covers: directive:log.handler_reserve_memory
#[test]
fn the_handlers_room_is_a_second_reserve_and_not_the_one_carved_out_of_the_request() {
    let room = governing("log.handler_reserve_memory");
    assert_eq!(
        (room.key, room.class, room.apply),
        ("log.handler_reserve_memory", Class::System, Apply::Reload),
        "`log.handler_reserve_memory` resolves through `{}`: with no row of its own the `[log]` \
         blanket answers `Runtime`, and the program being reported on sizes the report",
        room.key,
    );
    assert_ne!(
        room.key,
        governing("limits.fatal_reserve_memory").key,
        "one row answers for both reserves, so the slice carved out of a request's own heap and \
         the engine's allotment for a whole script are one number written in two blocks",
    );
    assert!(
        keys_in("log").contains(&"handler_reserve_memory".to_string())
            && !keys_in("limits").contains(&"handler_reserve_memory".to_string()),
        "`[limits]` accepts this key or `[log]` does not, and either way the allotment is being \
         written where the request's own budget is stated — which is the one thing it is not \
         taken from",
    );
    assert_eq!(
        unit_of("log.handler_reserve_memory"),
        None,
        "`value::unit_of`'s table has grown a `[log]` key. It is the limits blocks' table and \
         nothing else on purpose: a bare name it answers for is rewritten to `limits.<name>`, so a \
         row added here would give this allotment a spelling that lands in `[limits]`, where the \
         blanket is `Runtime`. The bytes reading lives at the one caller that knows it is bytes, \
         `nvs_runtime::Ctx::configured_handler_reserve`",
    );
}

/// The clock half of the same allotment, asserted as the pair it belongs to. Both halves answer
/// identically everywhere but the unit, for the reason `[limits]`' own two halves do: one room with
/// two measurements is still one room, and a half a request could move is the room gone.
///
/// The four reserves this repository ships are four rows — two for a handler the request already
/// holds, two for a script the engine has to start — and the failure worth pinning is the one where
/// two of them collapse into one. A registry that governed both time halves from a single row would
/// answer plausibly for either question asked on its own.
// covers: directive:log.handler_reserve_time
#[test]
fn the_handlers_clock_answers_as_its_memory_half_does_and_stays_a_row_of_its_own() {
    let clock = governing("log.handler_reserve_time");
    let room = governing("log.handler_reserve_memory");
    assert_eq!(
        (clock.key, clock.class, clock.apply),
        ("log.handler_reserve_time", Class::System, Apply::Reload),
        "`log.handler_reserve_time` resolves through `{}`, and a row of its own is what it needs: \
         the memory half is not a dotted prefix of it, so the `[log]` blanket is what would answer",
        clock.key,
    );
    assert_eq!(
        (clock.class, clock.apply),
        (room.class, room.apply),
        "the two halves of one allotment answer differently, so a deployment can state one of them \
         and the program being reported on the other",
    );

    let reserves = [
        "limits.fatal_reserve_memory",
        "limits.fatal_reserve_time",
        "log.handler_reserve_memory",
        "log.handler_reserve_time",
    ];
    let mut rows: Vec<&str> = reserves.iter().map(|key| governing(key).key).collect();
    rows.sort_unstable();
    rows.dedup();
    assert_eq!(
        rows.len(),
        reserves.len(),
        "the four reserves resolve through fewer than four rows, so one of them is being governed \
         by another's — and the pair that collapsed is the pair a deployment would then be unable \
         to size apart",
    );
    for key in reserves {
        assert!(
            !governing(key).class.settable_by_a_request(),
            "`{key}` is a request's to set, and a request that sizes any of the four has sized the \
             room its own failure is reported in",
        );
    }

    assert_eq!(
        (
            unit_of("log.handler_reserve_time"),
            unit_of("limits.fatal_reserve_time"),
        ),
        (None, Some(Unit::Duration)),
        "the two time halves are read the same way, and they are not: the `[limits]` one is a \
         quantity the block's own table names, and this one is a `[log]` key the table is not \
         written for — its memory half's case owns why that boundary is where it is",
    );
}

/// The code a `[log]` block whose `target` is `written` is refused with, and `None` for one this
/// boot accepts.
///
/// Through `file::parse` and then `log::validate`, which is the pair a boot runs, so what the case
/// below asserts is the answer an operator gets rather than a grammar helper's return value.
fn target_refusal(written: &str) -> Option<Code> {
    let text = format!("[log]\ntarget = \"{written}\"\n");
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", &text);
    let config = parsed.unwrap_or_else(|err| panic!("{text}-- did not parse: {}", err.message));
    nvs_config::log::validate(&config, &BTreeMap::new())
        .err()
        .map(|refused| refused.code.expect("a boot refusal carries its code"))
}

/// `rule:errors/engine-floor`'s destination, asserted as the two claims its page makes: who may
/// move it, and when a value naming no destination is refused.
///
/// The first needs a row of its own, and that is the failure worth pinning here rather than the
/// class in isolation — `[log]`'s blanket is `Runtime` and every other key in the block resolves
/// through it, so this row deleted is not a compile error or a missing key but a request that can
/// send the record of its own failure somewhere nobody reads. Asserted over every spelling that
/// reaches the same destination, because the per-application copy of the block is a second way in
/// and lands on a different row.
///
/// The second half is the *boot*, and both sides of the bound are named. The one moment the engine
/// cannot afford to raise a diagnostic about its configuration is the moment it is already
/// reporting a failure, so a destination is resolved where it is written; a check that only
/// accepted would pass on a grammar that accepts anything, and one that only refused would pass on
/// a grammar that accepts nothing and never boots.
// covers: directive:log.target
#[test]
fn the_floors_destination_is_the_operators_and_a_value_naming_none_refuses_the_tree() {
    let target = governing("log.target");
    assert_eq!(
        (target.key, target.class, target.apply),
        ("log.target", Class::System, Apply::Reload),
        "`log.target` resolves through `{}`: without a row of its own the `[log]` blanket answers \
         for it at `Runtime`, and `rule:errors/engine-floor`'s sink is then the failing program's \
         to move",
        target.key,
    );
    assert!(
        keys_in("log").contains(&"target".to_string()),
        "`[log]` no longer accepts `target`, so this row governs a key the file refuses and every \
         deployment writes its floor records wherever the engine defaults to",
    );
    for written in ["log.target", "app.log.target", "app.0.log.target"] {
        let row = governing(written);
        assert!(
            !row.class.settable_by_a_request(),
            "`{written}` lands on `{}` at {:?}, which a request may write — one spelling of this \
             destination a program can reach is the report of its own failure redirected, \
             whichever block it is written in",
            row.key,
            row.class,
        );
    }

    for written in ["stderr", "syslog", "file:/var/log/nvs.log"] {
        assert_eq!(
            target_refusal(written),
            None,
            "`{written}` is one of `rule:errors/engine-floor`'s three destinations and this boot \
             refuses it, so a deployment that spelled its sink correctly does not start",
        );
    }
    for written in ["stdout", "STDERR", "journald", "file:", ""] {
        assert_eq!(
            target_refusal(written),
            Some(code::E_UNSPELLED_LOG_TARGET),
            "`{written}` names no destination and boots anyway, so the tree is green and the \
             records go wherever a value nobody resolved leads — which is discovered by a \
             deployment that has already failed twice",
        );
    }
}

/// The code a tree whose `[block]` holds `body` is refused with, and `None` for one this boot
/// accepts.
///
/// Through `file::parse` and then `export::validate`, which is the pair a boot runs, for
/// `target_refusal`'s reason: what is asserted is the answer an operator gets.
fn export_refusal(block: &str, body: &str) -> Option<Code> {
    let text = format!("[{block}]\n{body}\n");
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", &text);
    let config = parsed.unwrap_or_else(|err| panic!("{text}-- did not parse: {}", err.message));
    nvs_config::export::validate(&config, &BTreeMap::new())
        .err()
        .map(|refused| refused.code.expect("a boot refusal carries its code"))
}

/// `rule:observability/metrics-and-trace-blocks-are-system`'s first block, asserted as the two
/// claims its page makes: who may write it, and which protocols it names.
///
/// The class is counted over every key the block accepts rather than read off the `exporter` row,
/// because a blanket is exactly where a registry answers plausibly key by key and wrongly overall:
/// a carve-out nobody wrote inherits nothing from the block, and a key that became a request's is
/// a program deciding how closely it is watched — which produces *silence*, and silence is
/// indistinguishable from a deployment with nothing to say.
///
/// The roster is the other half, and it is asserted as the asymmetry it is rather than as a list.
/// This is the only one of the two blocks a collector may scrape, so `prometheus` accepted here
/// and refused one block over is the claim; a check that only accepted would pass on a grammar
/// that accepts anything, and `false` is checked beside them because the disabled state is a
/// boolean rather than a third protocol.
// covers: directive:metrics
#[test]
fn every_key_of_the_metrics_block_is_the_operators_and_only_it_may_be_scraped() {
    let block = governing("metrics");
    assert_eq!(
        (block.key, block.class, block.apply),
        ("metrics", Class::System, Apply::Reload),
        "`[metrics]` resolves through `{}`, and the block's own row is what answers for every key \
         in it — none of them has one of its own",
        block.key,
    );

    let keys = keys_in("metrics");
    for expected in ["exporter", "listen", "endpoint", "max_series"] {
        assert!(
            keys.contains(&expected.to_string()),
            "`[metrics]` no longer accepts `{expected}`, so the page describes a key the file \
             refuses: {keys:?}",
        );
    }
    for key in &keys {
        let written = format!("metrics.{key}");
        let row = governing(&written);
        assert!(
            !row.class.settable_by_a_request(),
            "`{written}` lands on `{}` at {:?}, which a request may write — a program that sizes \
             or redirects its own measurement has chosen how closely it is watched",
            row.key,
            row.class,
        );
    }

    for body in [
        "exporter = \"prometheus\"",
        "exporter = \"otlp\"",
        "exporter = false",
        "exporter = \"otlp\"\nmax_series = 10000",
    ] {
        assert_eq!(
            export_refusal("metrics", body),
            None,
            "`{body}` is what `rule:observability/metrics-and-trace-blocks-are-system` gives this \
             block, and this boot refuses it",
        );
    }
    for body in [
        "exporter = \"graphite\"",
        "exporter = \"PROMETHEUS\"",
        "exporter = true",
    ] {
        assert_eq!(
            export_refusal("metrics", body),
            Some(code::E_UNSPELLED_EXPORTER),
            "`{body}` names no protocol and boots anyway, so a collector is never written to and \
             every dashboard over it is empty rather than wrong",
        );
    }
    assert_eq!(
        export_refusal("trace", "exporter = \"prometheus\""),
        Some(code::E_UNSPELLED_EXPORTER),
        "a scrape is accepted in `[trace]` as well, so the two rosters have become one: a span is \
         a finished record and there is no current value of one for a scrape to answer with",
    );
}

/// `rule:http-server/the-mode-ceiling-defaults-to-the-startup-mode`'s line, asserted beside the key
/// it bounds rather than on its own. `[mode]` is two keys of two different classes, and that split
/// *is* the feature: the mode an application starts in is a request's to flip, and how far it may
/// flip is not. Either row read alone looks like an ordinary directive, and the pair is the claim.
///
/// The second half is that the block carries no blanket, which is what makes the split enforceable.
/// Every other block here has a row over the whole name, so a key added later inherits a class
/// nobody chose for it; a third key under `[mode]` would inherit nothing instead, and is asserted
/// over the keys the block *accepts* rather than over the two this case could have named — a
/// carve-out added without a row is exactly the edit that passes a named check.
// covers: directive:mode.ceiling
#[test]
fn the_mode_ceiling_is_the_operators_half_of_a_block_with_no_blanket_over_it() {
    let ceiling = governing("mode.ceiling");
    assert_eq!(
        (ceiling.key, ceiling.class, ceiling.apply),
        ("mode.ceiling", Class::System, Apply::Reload),
        "`mode.ceiling` resolves through `{}`: a request that reaches this key raises the bound on \
         its own next flip, and `Boot` would make an operator restart a host to lower it",
        ceiling.key,
    );
    assert_eq!(
        governing("mode.default").class,
        Class::Runtime,
        "`mode.default` is no longer a request's to set, so the ceiling bounds a flip that can no \
         longer happen and `rule:config/a-program-may-read-and-flip-its-mode` has nothing left to \
         bound",
    );
    for written in ["mode.ceiling", "app.mode.ceiling", "app.0.mode.ceiling"] {
        let row = governing(written);
        assert!(
            !row.class.settable_by_a_request(),
            "`{written}` lands on `{}` at {:?}, which a request may write — one spelling of this \
             line a program can reach is the line gone, and the flip it refuses is a call away",
            row.key,
            row.class,
        );
    }

    assert!(
        lookup("mode").is_none(),
        "`[mode]` has grown a blanket row, so a key added to the block inherits that row's class \
         instead of being named: the two halves here are two classes, and there is no third one a \
         blanket could be right about",
    );
    let keys = keys_in("mode");
    assert!(
        keys.contains(&"ceiling".to_string()) && keys.contains(&"default".to_string()),
        "`[mode]` no longer accepts both of its keys, so this pair governs something the file \
         refuses: {keys:?}",
    );
    for key in &keys {
        let written = format!("mode.{key}");
        assert_eq!(
            governing(&written).key,
            written,
            "`{written}` is a key `[mode]` accepts and no row is written for it, so nothing states \
             who may set it",
        );
    }
}

/// The other half of that block, read through the field the mode's own tests do not read.
/// `every_derived_default_names_a_directive_the_registry_holds` asks whether each of
/// `rule:config/a-mode-is-five-defaults`'s five rows is held and settable; this one asks what applying
/// one costs, which is the question that decides whether `mode.default` can be one line.
///
/// A mode is a shorthand for five defaults, so a `Boot` row among them would make the shorthand
/// something a running host can only half-apply: `nvs ctl reload` over an edited `[mode] default`
/// would move four directives and name the fifth, and a request's own flip would silently be four
/// fifths of a mode. Each row reads plausibly on its own line either way — the claim is that the six
/// of them agree.
// covers: directive:mode.default
#[test]
fn a_mode_change_applies_whole_because_every_default_it_selects_reloads() {
    let default = governing(nvs_config::mode::KEY);
    assert_eq!(
        (
            nvs_config::mode::KEY,
            default.key,
            default.class,
            default.apply
        ),
        (
            "mode.default",
            "mode.default",
            Class::Runtime,
            Apply::Reload
        ),
        "the key the mode is read and written at \
         (`rule:config/a-program-may-read-and-flip-its-mode`) is no longer the row the registry \
         governs by that name, so one of the two homes is describing a key the other does not have",
    );

    for row in nvs_config::mode::DERIVED {
        let held = governing(row.key);
        assert_eq!(
            held.apply,
            Apply::Reload,
            "`{}` is a default `mode.default` selects and applying it takes a restart, so the one \
             line an operator edits is a mode a running host can only partly be in",
            row.key,
        );
    }
}

/// `rule:observability/metrics-and-trace-blocks-are-system`'s second block, asserted as the two
/// things only this one has: a fraction, and a header an outbound call carries.
///
/// The class is counted over the keys the block accepts rather than read off `exporter`, for the
/// reason the `[metrics]` case beside this one gives — but the consequence here is the sharper of
/// the two. A program that could move `sample` selects which requests are worth recording, and one
/// that could drop `propagate` leaves its own hop out of somebody else's story, both of which are
/// the reconnaissance channel `rule:testing/debug-mode-directive` already refuses.
///
/// `Reload` beside `System` is not decoration: `export::head_sample` reads the fraction per request
/// off the published snapshot, so an operator turning recording down on a host under load reaches
/// the next request rather than the next boot.
///
/// The last claim is the one an operator gets wrong rather than an attacker: the exporter and the
/// fraction are two lines, and the fraction is none until it is written, so naming a collector and
/// stopping there produces a collector nothing ever arrives at.
// covers: directive:trace
#[test]
fn every_key_of_the_trace_block_is_the_operators_and_an_exporter_alone_records_nothing() {
    let block = governing("trace");
    assert_eq!(
        (block.key, block.class, block.apply),
        ("trace", Class::System, Apply::Reload),
        "`[trace]` resolves through `{}`: the fraction is read off the standing snapshot per \
         request, and `Boot` would make an operator restart a host to stop recording it",
        block.key,
    );

    let keys = keys_in("trace");
    for expected in ["exporter", "endpoint", "sample", "propagate"] {
        assert!(
            keys.contains(&expected.to_string()),
            "`[trace]` no longer accepts `{expected}`, so the page describes a key the file \
             refuses: {keys:?}",
        );
    }
    for key in &keys {
        let written = format!("trace.{key}");
        let row = governing(&written);
        assert!(
            !row.class.settable_by_a_request(),
            "`{written}` lands on `{}` at {:?}, which a request may write — a program that picks \
             its own sample has chosen which of its requests anybody gets to read about",
            row.key,
            row.class,
        );
    }

    let text = "[trace]\nexporter = \"otlp\"\nendpoint = \"https://collector.example/v1\"\n";
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text);
    let named_a_collector =
        parsed.unwrap_or_else(|err| panic!("{text}-- did not parse: {}", err.message));
    assert_eq!(
        nvs_config::export::validate(&named_a_collector, &BTreeMap::new())
            .err()
            .map(|refused| refused.code),
        None,
        "a tree naming a collector and no fraction is refused at boot, so the pair below is being \
         asserted about a tree no operator can write",
    );
    assert_eq!(
        nvs_config::export::head_sample(&named_a_collector),
        0.0,
        "an exporter written on its own now records something, so the fraction has grown a second \
         home — the one an operator reads is the line they did not write",
    );
}

/// `rule:config/opcache-revalidation-is-system-class` and
/// `rule:config/opcache-file-cache-directives-are-system` are one class over one block, and this is
/// the class rather than either rule's semantics — `the_opcache_block_is_read_into_a_revalidation_policy`
/// and `the_validate_default_is_selected_by_the_run_mode` own what the keys mean.
///
/// `Class::System` exactly, and not merely a class no request may set: `RuntimeTighten` is the one
/// that reads plausibly here and is the mistake this case exists to catch. Every other key a request
/// may only tighten has a safe direction — a dump turned off, a grant dropped — and this block has
/// none, because "check my sources *more* often" is a `stat` storm on a hot file and "read my code
/// from a smaller cache" is still a program choosing how its own code is loaded.
///
/// Counted over the keys the block accepts rather than over the seven the two rules name, for the
/// reason the blocks above are: an eighth key added to the file inherits the blanket, and a
/// carve-out written beside it inherits nothing at all.
// covers: directive:opcache
#[test]
fn every_key_of_the_opcache_block_is_system_class_because_it_has_no_safe_direction() {
    let block = governing("opcache");
    assert_eq!(
        (block.key, block.class, block.apply),
        ("opcache", Class::System, Apply::Reload),
        "`[opcache]` resolves through `{}`: what the block decides is read by the next compile, and \
         `Boot` would make an operator restart a host to re-check a file sooner",
        block.key,
    );

    let keys = keys_in("opcache");
    for expected in [
        "validate",
        "revalidate_freq",
        "file_cache",
        "file_cache_dir",
        "file_cache_max_size",
        "file_cache_gc_probability",
        "file_cache_gc_divisor",
    ] {
        assert!(
            keys.contains(&expected.to_string()),
            "`[opcache]` no longer accepts `{expected}`, so the two rules over this block describe \
             a key the file refuses: {keys:?}",
        );
    }
    for key in &keys {
        let written = format!("opcache.{key}");
        let row = governing(&written);
        assert_eq!(
            row.class,
            Class::System,
            "`{written}` lands on `{}` at {:?} — a request that may tighten this block is a request \
             that may pin the code it likes or choose the cache its code is read from, and neither \
             direction of either key is safe",
            row.key,
            row.class,
        );
    }
}

/// The one key of that block that is not `Reload`, asserted beside the sibling whose name it is a
/// prefix of. `reloadability_is_a_field_of_its_own_and_not_the_changeability_class` names this row in
/// `rule:config/reloadability-is-its-own-field`'s `Boot` list; what is asked here is why it is a row
/// at all — `file_cache` and `file_cache_dir` are two keys one `starts_with` apart, and a registry
/// that governed the first through the second would make turning the cache off wait for a restart.
///
/// The second half is `rule:config/opcache-file-cache-directives-are-system`'s *only spelling*
/// clause. `[cache]` is `Core\Cache`'s tiers and holds nothing about compiled units, so an operator
/// who writes the artifact directory there has written a key the file does not know — and the
/// failure this pins is the helpful repair where it starts to work, which would put the directory a
/// request may not set beside the tiers it may.
// covers: directive:opcache.file_cache_dir
#[test]
fn the_artifact_directory_is_one_boot_row_in_a_reload_block_and_has_no_second_spelling() {
    let directory = governing("opcache.file_cache_dir");
    assert_eq!(
        (directory.key, directory.class, directory.apply),
        ("opcache.file_cache_dir", Class::System, Apply::Boot),
        "`opcache.file_cache_dir` resolves through `{}`: every unit this process has already mapped \
         was read out of the standing directory, so a reload cannot be what moves it",
        directory.key,
    );

    let bool_beside_it = governing("opcache.file_cache");
    assert_eq!(
        (bool_beside_it.key, bool_beside_it.apply),
        ("opcache", Apply::Reload),
        "`opcache.file_cache` is governed by `{}`, so a prefix that did not end on a dot has caught \
         the key whose name it opens — turning the cache off now waits for a restart it does not need",
        bool_beside_it.key,
    );

    assert!(
        lookup("cache.dir").is_none(),
        "`[cache] dir` now names a directive, so the artifact cache has a second spelling under the \
         block that holds `Core\\Cache`'s tiers — and the one a request may not set is beside two it may",
    );
    let tiers = keys_in("cache");
    assert!(
        !tiers.contains(&"dir".to_string()),
        "`[cache]` accepts `dir`, so an operator writing the artifact directory there is told \
         nothing: {tiers:?}",
    );
}
