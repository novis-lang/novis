//! ADR 0064 § 5 and ADR 0005: what one request may move, how far, and how far that move reaches.
//!
//! The three cases `m6.md`'s *Verify* names in one sentence are here under its own words, because
//! the third — invisibility to the next request on the same core — is the one a shared mutable
//! registry passes every other test while getting wrong. It is asked of two `Request`s built from
//! **one** `Arc<Snapshot>`, which is exactly what two requests on a core hold.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

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

/// ADR 0005: `[limits]` states the **default**, and a request may move above it for itself as far
/// as the ceiling — the first of `m6.md`'s three clauses.
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

/// A `System` directive is refused whatever its value: ADR 0005's class answers *who may set it*
/// before any ceiling is consulted, so the ceiling of the ceiling is not a question.
#[test]
fn a_system_directive_is_not_settable_by_a_request() {
    let mut request = Request::new(snapshot_of(BOUNDED));

    assert!(!request.set("limits.hard.memory", "16M"));
    assert!(!request.set("cache.dir", "/tmp"));
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

/// ADR 0074 §§ 2-3, as the **agreement** the ADR's own sentence asks for: each pair is refused "at
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
    // The row's own line is written out twice rather than appended to the block, because ADR 0064
    // § 3 still refuses a key set twice **in one file**: an appended override would refuse the boot
    // as a duplicate and the two mechanisms would agree for a reason that has nothing to do with
    // ADR 0074.
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
             `Core\\Config::set` ({by_set}); ADR 0074 §§ 2-3 make those one rule",
        );
        agreed += 1;
        refused += usize::from(by_set);

        if by_set {
            assert_eq!(
                request.get(key),
                before,
                "a refused `set` leaves the previous value in force (ADR 0005)",
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
