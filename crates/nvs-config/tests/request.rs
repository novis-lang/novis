//! `rule:config/ini-set-is-core-config-set` and `rule:config/three-changeability-classes`: what one request may move, how far, and how far that move reaches.
//!
//! The cases `m6.md`'s *Verify* names in one sentence are here under its own words, because
//! invisibility to the next request on the same core is the one a shared mutable registry passes
//! every other test while getting wrong. It is asked of two `Request`s built from **one**
//! `Arc<Snapshot>`, which is exactly what two requests on a core hold.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use nvs_config::capability::{Cap, Scope};
use nvs_config::directive::{Class, DIRECTIVES};
use nvs_config::resolve::{Files, Roots, resolve};
use nvs_config::snapshot::Snapshot;
use nvs_config::trust::Untrusted;
use nvs_config::{Request, Setting};
use nvs_diagnostics::SourceMap;

/// A path written the way an ADR writes one, as a path the host spells its own way.
fn p(path: &str) -> PathBuf {
    path.split('/').collect()
}

/// One `nvs.toml` and nothing else — every case here is about what a request does with a tree,
/// never about how the tree was assembled.
struct One(String);

impl Files for One {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        self.canonical(path).map_err(Untrusted::Unreadable)
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        let mut out = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    out.pop();
                }
                other => out.push(other.as_os_str()),
            }
        }
        if out == p("nvs.toml") {
            Ok(out)
        } else {
            Err("no such file".to_string())
        }
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        self.canonical(path).map(|_| self.0.clone())
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.read(path).map(String::into_bytes)
    }

    fn exposure(&self, _path: &Path) -> Option<String> {
        None
    }

    fn list(&self, _dir: &Path) -> Result<Vec<PathBuf>, String> {
        Ok(vec![p("nvs.toml")])
    }

    fn exists(&self, path: &Path) -> bool {
        self.canonical(path).is_ok()
    }
}

/// The snapshot `text` produces for `nvs.toml` itself, panicking with the refusal when there is
/// none.
fn snapshot_of(text: &str) -> Arc<Snapshot> {
    let files = One(text.to_string());
    let mut sources = SourceMap::new();
    let resolved = resolve(&Roots::Files(vec![p("nvs.toml")]), &mut sources, &files)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes));
    Snapshot::build(&resolved, &p("nvs.toml"), &files)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes))
}

/// `examples/config.nvs`'s own bound, and the repository's own `nvs.toml` in miniature: a default
/// the request starts at and a ceiling the operator kept above it.
const BOUNDED: &str = "\
[limits]
memory = \"256M\"

[limits.hard]
memory = \"512M\"
";

/// `rule:config/three-changeability-classes`: `[limits]` states the **default**, and a request may move above it for itself as far
/// as the ceiling — the first of `m6.md`'s clauses.
#[test]
fn config_set_above_the_default_takes_effect() {
    let mut request = Request::new(snapshot_of(BOUNDED));

    assert_eq!(request.get("memory").as_deref(), Some("256M"));
    assert!(request.set("memory", "512M"));
    assert_eq!(request.get("memory").as_deref(), Some("512M"));
}

/// The second clause: above the `[limits.hard]` ceiling it is `false` **and the previous value is
/// still there** — a refusal that half-applied would pass a test asserting only the `false`.
#[test]
fn config_set_above_the_hard_ceiling_returns_false_with_the_previous_value_intact() {
    let mut request = Request::new(snapshot_of(BOUNDED));

    assert!(request.set("memory", "512M"));
    assert!(!request.set("memory", "1G"));
    assert_eq!(request.get("memory").as_deref(), Some("512M"));

    // And from the file's own value, so the "previous" that survives is whatever was in force and
    // not just the one an earlier `set` wrote.
    let mut fresh = Request::new(snapshot_of(BOUNDED));
    assert!(!fresh.set("memory", "1G"));
    assert_eq!(fresh.get("memory").as_deref(), Some("256M"));
}

/// The third clause, and the one the shape has to answer rather than the code: two requests on one
/// core hold two [`Request`]s over **one** `Arc<Snapshot>`, so a `set` that reached the snapshot
/// would be visible here.
#[test]
fn config_set_is_invisible_to_the_next_request_on_the_same_core() {
    let published = snapshot_of(BOUNDED);
    let mut first = Request::new(Arc::clone(&published));
    let next = Request::new(Arc::clone(&published));

    assert!(first.set("memory", "512M"));

    assert_eq!(first.get("memory").as_deref(), Some("512M"));
    assert_eq!(next.get("memory").as_deref(), Some("256M"));
    assert_eq!(
        published
            .config
            .limits
            .as_ref()
            .and_then(|l| l.memory.clone()),
        Some(Setting::Text("256M".to_string())),
    );
}

/// `restore` is `ini_restore`: it drops what this request set and leaves the file's value, and a
/// name never set is not an error.
#[test]
fn restore_drops_only_what_this_request_set() {
    let mut request = Request::new(snapshot_of(BOUNDED));

    assert!(request.set("memory", "512M"));
    request.restore("memory");
    assert_eq!(request.get("memory").as_deref(), Some("256M"));

    request.restore("wall_time");
    assert_eq!(request.get("wall_time"), None);
}

/// A `System` directive is refused whatever its value: `rule:config/three-changeability-classes`'s class answers *who may set it*
/// before any ceiling is consulted, so the ceiling of the ceiling is not a question.
#[test]
fn a_system_directive_is_not_settable_by_a_request() {
    let mut request = Request::new(snapshot_of(BOUNDED));

    assert!(!request.set("limits.hard.memory", "16M"));
    assert!(!request.set("opcache.file_cache_dir", "/tmp"));
    assert_eq!(request.get("limits.hard.memory").as_deref(), Some("512M"));
}

/// A value that does not spell its unit is refused even where no ceiling bounds it — `get` would
/// otherwise answer with something the boot path would have refused out of the file.
#[test]
fn a_value_that_is_not_a_quantity_is_refused() {
    let mut request = Request::new(snapshot_of("[limits]\nmemory = \"256M\"\n"));

    assert!(!request.set("memory", "later"));
    assert!(request.set("memory", "1G"), "no ceiling bounds this tree");
    assert_eq!(request.get("memory").as_deref(), Some("1G"));
}

/// A name no registry row governs is refused rather than stored: `Core\Config` is the directive
/// registry's surface and not a per-request string map.
#[test]
fn a_name_no_directive_governs_is_refused() {
    let mut request = Request::new(snapshot_of(BOUNDED));

    assert!(!request.set("nothing.like.this", "1"));
    assert_eq!(request.get("nothing.like.this"), None);
}

/// § 5's `all()`: every leaf by dotted name, with the overlay folded over it — and the bare name a
/// `set` used resolving to the same `limits.memory` the file wrote.
#[test]
fn all_is_every_leaf_by_dotted_name_with_the_overlay_over_it() {
    let mut request = Request::new(snapshot_of(BOUNDED));
    assert!(request.set("memory", "384M"));

    let all = request.all();
    assert_eq!(all.get("limits.memory").map(String::as_str), Some("384M"));
    assert_eq!(
        all.get("limits.hard.memory").map(String::as_str),
        Some("512M"),
    );
}

/// Values cross as text however the file typed them (§ 5), so an integer directive answers with
/// the way TOML spells it rather than with nothing.
#[test]
fn a_typed_directive_crosses_as_the_text_it_was_written_as() {
    let request = Request::new(snapshot_of(
        "[limits]\nmax_tasks = 64\n\n[log]\nlevel = \"warn\"\n",
    ));

    assert_eq!(request.get("max_tasks").as_deref(), Some("64"));
    assert_eq!(request.get("log.level").as_deref(), Some("warn"));
}

/// Whether `text` is refused as a whole configuration, without panicking when it is.
fn boot_refuses(text: &str) -> bool {
    let files = One(text.to_string());
    let mut sources = SourceMap::new();
    resolve(&Roots::Files(vec![p("nvs.toml")]), &mut sources, &files).is_err()
}

/// `rule:http-server/cors-is-closed-until-origins-are-named` and `rule:http-server/cookies-are-secure-httponly-and-lax`, as the **agreement** the ADR's own sentence asks for: each pair is refused "at
/// boot with the line named and at runtime by `Core\Config::set` returning `false`".
///
/// This is the case that needs one implementation rather than two. It never asserts what either
/// mechanism answered — it asserts they answered the **same**, over a table of assignments, so a
/// second copy of the condition that drifts in either direction fails here while each half still
/// looks right on its own. Each row is a tree that boots, the key a request then moves, and the
/// value that makes the pair meaningless; the same value written into the file must refuse the boot
/// exactly when `set` refuses it.
#[test]
fn the_same_two_refusals_come_from_config_set_as_from_the_boot() {
    // The row's own line is written out twice rather than appended to the block, because `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`
    // still refuses a key set twice **in one file**: an appended override would refuse the boot
    // as a duplicate and the two mechanisms would agree for a reason that has nothing to do with
    // `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`.
    //
    // (the block without the key, the line it starts at, the key, the value moved to, that line)
    let moves = [
        // § 2. The wildcard is in the block, because a request cannot set a list.
        (
            "[http.cors]\norigins = [\"*\"]\n",
            "credentials = false",
            "http.cors.credentials",
            "true",
            "credentials = true",
        ),
        (
            "[http.cors]\norigins = [\"https://app.example\"]\n",
            "credentials = true",
            "http.cors.credentials",
            "false",
            "credentials = false",
        ),
        (
            "[http.cors]\norigins = [\"https://app.example\"]\n",
            "credentials = false",
            "http.cors.credentials",
            "true",
            "credentials = true",
        ),
        // § 3, where the pair breaks from either of its two keys.
        (
            "[http.cookies]\nsame_site = \"None\"\n",
            "secure = true",
            "http.cookies.secure",
            "false",
            "secure = false",
        ),
        (
            "[http.cookies]\nsecure = false\n",
            "same_site = \"Lax\"",
            "http.cookies.same_site",
            "None",
            "same_site = \"None\"",
        ),
        (
            "[http.cookies]\nsecure = false\n",
            "same_site = \"Lax\"",
            "http.cookies.same_site",
            "Strict",
            "same_site = \"Strict\"",
        ),
    ];

    let mut agreed = 0;
    let mut refused = 0;
    for (block, from, key, value, line) in moves {
        // The starting tree boots, so what either mechanism then refuses is the move and never the
        // ground it was made from.
        let mut request = Request::new(snapshot_of(&format!("{block}{from}\n")));
        let before = request.get(key);

        let by_set = !request.set(key, value);
        // The same assignment written into the file instead.
        let by_boot = boot_refuses(&format!("{block}{line}\n"));

        assert_eq!(
            by_set, by_boot,
            "`{key} = {value}` over `{block}` is refused by the boot ({by_boot}) and by \
             `Core\\Config::set` ({by_set}); `rule:http-server/cors-is-closed-until-origins-are-named` and `rule:http-server/cookies-are-secure-httponly-and-lax` make those one rule",
        );
        agreed += 1;
        refused += usize::from(by_set);

        if by_set {
            assert_eq!(
                request.get(key),
                before,
                "a refused `set` leaves the previous value in force (`rule:config/three-changeability-classes`)",
            );
        } else {
            assert_eq!(request.get(key).as_deref(), Some(value));
        }
    }

    assert_eq!(agreed, moves.len());
    // The positive control: without it a `set` that accepted everything and a boot that refused
    // nothing would agree perfectly and this case would pass having tested nothing.
    assert_eq!(
        refused, 3,
        "three of the six moves land on a meaningless pair; the other three are the controls",
    );
}

/// m6.md's *Verify*, adversarially: **every** `System` row is refused, counted over the registry
/// rather than asked of one key.
///
/// `a_system_directive_is_not_settable_by_a_request` above pins the class rule on the keys it
/// names, which is what the rule means. What this adds is that no row escapes it: a directive
/// landing in [`DIRECTIVES`] as `System` is swept the day it lands, and a `set` that grew a
/// special case for one block fails here while still passing every case above it. Both spellings
/// of a row are asked — the key itself and a key beneath it — because `lookup` is longest-prefix
/// and a block row answers for everything inside it.
#[test]
fn a_script_attempting_to_set_a_system_directive_fails() {
    let mut request = Request::new(snapshot_of(BOUNDED));
    let before = request.all();

    let system: Vec<&'static str> = DIRECTIVES
        .iter()
        .filter(|row| row.class == Class::System)
        .map(|row| row.key)
        .collect();
    // A floor, not a census: `tests/directives.rs` owns which row is which class. What it buys here
    // is that a sweep which found nothing to sweep cannot pass as a sweep that refused everything.
    assert!(
        system.len() > 10,
        "the sweep is the registry's `System` rows and there are {}",
        system.len(),
    );

    let mut accepted: Vec<String> = Vec::new();
    for key in &system {
        for spelling in [(*key).to_string(), format!("{key}.memory")] {
            for value in ["1G", "true", "/"] {
                if request.set(&spelling, value) {
                    accepted.push(format!("{spelling} = {value}"));
                }
            }
        }
    }

    assert!(
        accepted.is_empty(),
        "a request set a `System` directive: {accepted:?}",
    );
    // Not one of them left anything behind either — a `set` that returned `false` after writing the
    // overlay would pass every assertion above.
    assert_eq!(
        request.all(),
        before,
        "a refused `set` changes nothing about what is in force (`rule:config/three-changeability-classes`)",
    );

    // The control: the `Runtime` row sitting beside them takes the same value. Without it a `set`
    // that refused everything would pass this case having tested nothing.
    assert!(request.set("memory", "512M"));
}

/// `rule:config/three-changeability-classes`'s `RuntimeTighten` half, adversarially: a request cannot widen **any** capability, asked
/// once per row of [`Cap::ALL`] rather than of the one grant this tree happens to hold.
///
/// Two reasons refuse, and the fixture holds both. `capabilities.fs.read` has a grant in force and
/// is refused because a list cannot be shown to narrow — `request.rs`'s module doc owns why that is
/// the safe direction — while every other row has nothing in force and is refused for having
/// nothing to narrow *from*. Neither reason is the value's: `log.level` is the same unquantifiable
/// shape under `Runtime` and is accepted, so what refuses above is the class, which is the claim.
#[test]
fn a_script_attempting_to_widen_a_capability_fails() {
    const GRANTED: &str = "\
[limits]
memory = \"256M\"

[capabilities.fs]
read = [\"nvs.toml\"]
";
    let files = One(GRANTED.to_string());
    let mut request = Request::new(snapshot_of(GRANTED));

    assert!(
        !Cap::ALL.is_empty(),
        "the sweep is the capability roster, and an empty roster sweeps nothing",
    );
    let mut widened: Vec<String> = Vec::new();
    for cap in Cap::ALL {
        // The grant's own key and the block above it, against the widest value `rule:security/capability-check-at-the-door` spells and
        // against narrower widenings — a root outside what was granted, and the one inside it,
        // which is not narrowing either once it is the whole grant.
        for key in [
            format!("capabilities.{}", cap.name()),
            "capabilities".to_string(),
        ] {
            for value in ["true", "/", "nvs.toml"] {
                if request.set(&key, value) {
                    widened.push(format!("{key} = {value}"));
                }
            }
        }
    }
    assert!(
        widened.is_empty(),
        "a request widened a capability: {widened:?}",
    );

    // What the file granted is still exactly what is in force, asked of `capability.rs` itself
    // rather than of `get`: a list has no rendering `get` answers with, so the sweep's effect has to
    // be read where the runtime reads it.
    let granted = request
        .snapshot()
        .config
        .capabilities
        .clone()
        .unwrap_or_default();
    let root = p("nvs.toml");
    assert!(granted.allows(Cap::FsRead, Scope::Path(root.as_path()), &files));
    assert!(!granted.allows(Cap::FsWrite, Scope::Path(root.as_path()), &files));
    assert!(!granted.allows(Cap::NetConnect, Scope::Host("evil.example"), &files));

    assert!(request.set("log.level", "debug"));
}

/// `rule:config/a-mode-is-five-defaults`'s table is **closed**, and this is what makes that word mean something.
///
/// The section's second property is that *what exactly does development mode change?* has a
/// complete, mechanical answer at any moment; a sixth row added without an ADR amendment would take
/// that away silently, and every other case here would still pass. Asserting the key set — rather
/// than the count — also fails a row renamed to a directive nobody spelled.
#[test]
fn the_mode_table_is_exactly_the_five_directives_the_adr_lists() {
    let keys: Vec<&str> = nvs_config::mode::DERIVED
        .iter()
        .map(|row| row.key)
        .collect();

    assert_eq!(
        keys,
        [
            "debug.inline",
            "log.format",
            "log.level",
            "log.access",
            "http.errors.detail",
        ],
    );
    for row in nvs_config::mode::DERIVED {
        assert_ne!(
            row.production, row.development,
            "a row whose two modes agree is a default the mode does not select",
        );
    }
}

/// Each of those five rows is a **directive**, and `nvs_config::directive` is where a directive says
/// who may set it and what applying it costs.
///
/// A mode selects a *default*, so a row whose key the registry does not hold is a default for a
/// directive nobody spelled: a flip would write the key into the same overlay `Core\Config::set`
/// refuses to write it into, and the two would be answering about one name. `debug.inline` was
/// exactly that until `rule:errors/debug-dump`'s row landed, which is why the assertion is the key
/// set of one table read through the other rather than a count.
#[test]
fn every_derived_default_names_a_directive_the_registry_holds() {
    for row in nvs_config::mode::DERIVED {
        let held = nvs_config::directive::lookup(row.key)
            .unwrap_or_else(|| panic!("`{}` is a default for a directive nobody spelled", row.key));
        assert!(
            held.class.settable_by_a_request(),
            "`{}` is derived per request, so a `System` row would make the mode's own default \
             unreachable",
            row.key
        );
    }
}

/// `rule:config/a-program-may-read-and-flip-its-mode` and `rule:http-server/the-mode-ceiling-defaults-to-the-startup-mode` through the API a program actually holds: the flip is bounded by the ceiling, it
/// carries § 3's rows with it, and it is request-local like every other `set`.
///
/// The last clause is the one a shared registry would get wrong, and it is asked here for the same
/// reason `config_set_is_invisible_to_the_next_request_on_the_same_core` asks it of a limit: a mode
/// that leaked would be one request putting another into development.
#[test]
fn a_mode_flip_is_bounded_derived_and_request_local() {
    let snapshot = snapshot_of("[mode]\ndefault = \"production\"\nceiling = \"development\"\n");
    let mut request = Request::new(Arc::clone(&snapshot));
    let beside = Request::new(Arc::clone(&snapshot));

    assert!(request.set(nvs_config::mode::KEY, "development"));
    assert_eq!(request.get("log.level").as_deref(), Some("Debug"));
    assert_eq!(request.get("http.errors.detail").as_deref(), Some("full"));
    assert_eq!(
        beside.get("log.level"),
        None,
        "a flip is this request's own"
    );
    assert_eq!(
        beside.get(nvs_config::mode::KEY).as_deref(),
        Some("production")
    );

    // The ceiling is read off the snapshot and never off the overlay, so having flipped once does
    // not raise it — this tree's ceiling still refuses a mode it never named.
    assert!(!request.set(nvs_config::mode::KEY, "staging"));
    assert_eq!(
        request.get(nvs_config::mode::KEY).as_deref(),
        Some("development"),
        "a refused flip leaves the mode in force exactly where it was",
    );
}
