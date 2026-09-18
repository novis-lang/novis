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
