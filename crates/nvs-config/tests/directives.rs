//! The registry's two fields are two fields — `rule:config/reloadability-is-its-own-field` against `rule:config/three-changeability-classes` — plus the lookup rule
//! the module doc states.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use nvs_config::Config;
use nvs_config::directive::{Apply, Class, DIRECTIVES, Directive, lookup};
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for group in ["crates", "benches"] {
        let Ok(entries) = fs::read_dir(root.join(group)) else {
            continue;
        };
        for entry in entries.flatten() {
            collect_source(&entry.path().join("src"), &root, &mut found);
        }
    }
    found
}

/// One directory of [`workspace_source`]'s walk, and every directory under it.
fn collect_source(dir: &Path, root: &Path, found: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_source(&path, root, found);
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");
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
