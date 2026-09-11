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

/// Every setting the default file spells, each with the prose block standing above it.
///
/// A blank line or a block header ends a prose block, so what comes back beside a key is what an
/// operator reads immediately before writing it — which is the only place a warning about the key
/// is any use.
fn settings_with_prose() -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut prose = String::new();
    for line in nvs_config::default_file().lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('#') else {
            prose.clear();
            continue;
        };
        if rest.starts_with('[') {
            prose.clear();
            continue;
        }
        if is_setting(rest) {
            let (key, _) = rest.split_once('=').expect("a setting carries its `=`");
            found.push((key.trim().to_string(), prose.clone()));
        } else {
            prose.push(' ');
            prose.push_str(rest.trim());
        }
    }
    found
}

/// The owner a doc comment's `[unread:]` trailer names, for a comment that carries one.
fn unread_owner(doc: &str) -> Option<String> {
    let (trailer, _) = doc.split_once("[unread:")?.1.split_once(']')?;
    Some(trailer.split_once("owner:")?.1.trim().to_string())
}

/// Every field the tree declares unread, paired with the owner that declaration names.
///
/// Read out of `tree.rs`'s own text rather than listed here, so that a key gaining a trailer joins
/// this case in the commit that declares it and a key losing one leaves. Which *dotted* key a field
/// is belongs to `tools/directives.py`, which walks the field graph a string scan cannot see; what
/// is asserted below is the pair the file owes either way.
fn declared_unread() -> Vec<(String, String)> {
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
            && let Some(owner) = unread_owner(&doc)
        {
            found.push((field.trim().trim_start_matches("r#").to_string(), owner));
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

    for (field, owner) in &unread {
        assert!(
            settings.iter().any(|(key, prose)| key == field
                && prose.contains("NOT IMPLEMENTED")
                && prose.contains(owner)),
            "`{field}` is declared unread in the tree, and no `{field} = ` line in the default \
             file stands under a `NOT IMPLEMENTED` note naming `{owner}`",
        );
    }

    for (key, prose) in settings
        .iter()
        .filter(|(_, p)| p.contains("NOT IMPLEMENTED"))
    {
        assert!(
            unread
                .iter()
                .any(|(field, owner)| field == key && prose.contains(owner)),
            "the default file marks `{key}` unimplemented and `crates/nvs-config/src/tree.rs` \
             declares no `[unread:]` for it",
        );
    }
}
