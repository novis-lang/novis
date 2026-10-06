//! `rule:config/every-block-is-argued-where-it-is-added`'s block roster as a type, and § 3's unknown-key refusal that comes with it.
//!
//! The roster is the security-relevant half: a key this tree does not name is a key an operator may
//! write and no code will ever read, so the sweep below asserts by **counting** the blocks that
//! parse rather than reading one off a line — a tree that grew a typo in one block header still
//! deserializes every other block plausibly.

use std::path::{Path, PathBuf};

use nvs_config::resolve::{Disk, Files};
use nvs_config::{Config, Roots, Setting};
use nvs_diagnostics::{SourceMap, code};

/// Parses `text` as the typed tree, panicking with the refusal's message when it does not.
fn tree(text: &str) -> Config {
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text);
    parsed.unwrap_or_else(|err| panic!("{text}\n-- refused: {}", err.message))
}

/// The refusal `text` produces, panicking when it is accepted instead.
fn refusal(text: &str) -> nvs_diagnostics::Diagnostic {
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text);
    parsed
        .err()
        .unwrap_or_else(|| panic!("{text}\n-- was accepted, and should not have been"))
}

/// One line of TOML per block `rule:config/every-block-is-argued-where-it-is-added`'s table names, in that table's order. Every one must
/// parse; a block missing from [`Config`] fails here rather than at some operator's boot.
#[rustfmt::skip] // one row per block: this is a table and reads as one, like the registry it mirrors.
const BLOCKS: &[(&str, &str)] = &[
    ("[[include]]", "[[include]]\npath = \"conf.d/production.toml\"\noptional = true\n"),
    ("[[include]] dir", "[[include]]\ndir = \"conf.d\"\n"),
    ("[[app]]", "[[app]]\nroot = \"/srv/www\"\nmode = \"production\"\norigin = \"https://example.test\"\n"),
    ("[[app]] entry", "[[app]]\nentry = \"/srv/www/shop/bin/import.nvs\"\n[app.limits]\nwall_time = \"600s\"\n"),
    ("[app.capabilities]", "[[app]]\nroot = \"/srv/www/shop\"\n[app.capabilities]\nprocess.exec = true\n"),
    ("[app.limits.hard]", "[[app]]\nroot = \".\"\n[app.limits.hard]\nmemory = \"1G\"\n"),
    ("[limits]", "[limits]\nmemory = \"128M\"\ncpu_time = \"5s\"\nwall_time = \"30s\"\nmax_tasks = 64\nmax_output = \"32M\"\n"),
    ("[limits.hard]", "[limits.hard]\nmemory = \"2G\"\ncpu_time = \"60s\"\nwall_time = \"300s\"\nmax_tasks = 4096\nmax_output = \"512M\"\n"),
    ("[mode]", "[mode]\ndefault = \"production\"\nceiling = \"development\"\n"),
    ("[capabilities]", "[capabilities]\nscript.spawn = [\"/srv/www/jobs\"]\nprocess.exec = true\ndebug.trace = [\"/var/log/nvs/trace\"]\nfs.read = [\"/srv\"]\nfs.write = [\"/var/tmp\"]\nnet.connect = [\"reports.internal\"]\ndb.connect = [\"main\"]\ndb.open = [\"*.tenants.internal\"]\ndb.schema = [\"main\"]\ndebug.profile = [\"/var/log/nvs/profile\"]\n"),
    ("[[extension]]", "[[extension]]\npath = \"image.nvsx\"\nsha256 = \"9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08\"\nmemory = \"64M\"\n"),
    ("[debug]", "[debug]\nmode = [\"coverage\", \"branch\"]\n"),
    ("[log]", "[log]\nhandler = \"handler.nvs\"\nhandler_reserve_memory = \"8M\"\nhandler_reserve_time = \"2s\"\ntarget = \"stderr\"\nformat = \"json\"\nlevel = \"warning\"\n"),
    ("[errors]", "[errors]\ndeprecated = \"throw\"\n"),
    ("[http.errors]", "[http.errors]\ndetail = \"generic\"\n"),
    ("[http.headers]", "[http.headers]\ncontent_type_options = true\nframe_ancestors = \"none\"\nreferrer_policy = \"strict-origin-when-cross-origin\"\nhsts = \"365d\"\nhsts_subdomains = false\ncontent_security_policy = \"\"\npermissions_policy = \"\"\n"),
    ("[http.cors]", "[http.cors]\norigins = []\nmethods = [\"GET\", \"HEAD\", \"POST\"]\nheaders = []\nexpose = []\ncredentials = false\nmax_age = \"10m\"\n"),
    ("[http.cookies]", "[http.cookies]\nsecure = true\nhttp_only = true\nsame_site = \"Lax\"\npath = \"/\"\n"),
    ("[http.client]", "[http.client]\nconnect_timeout = \"5s\"\ndeadline = \"30s\"\nidle = \"30s\"\nmax_duration = \"5m\"\nmax_redirects = 0\npool_idle = 16\npool_idle_timeout = \"30s\"\n"),
    ("[db.<name>]", "[db.main]\ndriver = \"pgsql\"\nhost = \"db\"\nport = 5432\nuser = \"app\"\npassword_file = \"/run/secrets/db\"\ndatabase = \"shop\"\n"),
    ("[db.<name>] sqlite", "[db.local]\npath = \"data/app.sqlite\"\n"),
    ("[db.<name>.pool]", "[db.main]\ndriver = \"pgsql\"\n[db.main.pool]\nmax = 16\nidle = 2\nlifetime = \"30m\"\nacquire = \"5s\"\n"),
    ("[db.<name>] pool = false", "[db.main]\ndriver = \"pgsql\"\npool = false\n"),
    ("[deferred]", "[deferred]\nmax_concurrent = 256\ndeadline = \"30s\"\n"),
    ("[[schedule]]", "[[schedule]]\nname = \"nightly-report\"\ncron = \"0 3 * * *\"\nscript = \"jobs/report.nvs\"\nscope = \"fleet\"\ntimezone = \"Europe/Vienna\"\noverlap = \"skip\"\nlimits = {memory = \"512M\", cpu_time = \"120s\"}\ngrants = {net = {connect = [\"reports.internal\"]}}\n"),
    ("[metrics]", "[metrics]\nexporter = \"prometheus\"\nlisten = \"127.0.0.1:9090\"\nendpoint = \"\"\nmax_series = 10000\n"),
    ("[trace]", "[trace]\nexporter = \"otlp\"\nendpoint = \"\"\nsample = 0.01\npropagate = true\n"),
    ("[server]", "[server]\nroot = \"/www\"\nlisten = [\"127.0.0.1:8000\"]\nsocket_mode = \"0660\"\ndispatch = \"entry\"\nstatic = false\ntrusted_proxies = []\nhealth_path = \"\"\nmax_in_flight = 10000\nheader_timeout = \"10s\"\nbody_idle_timeout = \"30s\"\nwrite_idle_timeout = \"30s\"\nkeepalive_timeout = \"75s\"\n"),
    ("[[server.mount]]", "[[server.mount]]\nscan = \"*/public/index.nvs\"\nprefix = \"/{1}\"\norigin = \"https://{1}.example.com\"\n"),
    ("[[server.mount]] entry", "[[server.mount]]\nprefix = \"/admin\"\nentry = \"Backoffice/public/index.nvs\"\nhost = \"admin.example.com\"\n"),
    ("[cache]", "[cache.local]\nmax_size = \"32M\"\n[cache.shared]\nurl = \"redis://cache.internal\"\ntimeout = \"5s\"\n"),
    ("[control]", "[control]\nsocket = \"/run/nvs/control.sock\"\n"),
    ("[opcache]", "[opcache]\nvalidate = \"mtime\"\nrevalidate_freq = \"2s\"\nsettle = \"1s\"\nfile_cache = true\nfile_cache_dir = \"/var/cache/nvs\"\nfile_cache_max_size = \"1G\"\nfile_cache_gc_probability = 1\nfile_cache_gc_divisor = 100\n"),
];

/// Every block an ADR writes out parses, asserted by counting rather than by reading one off a
/// line: a roster is only as good as its worst entry, and a sweep that stopped at the first failure
/// would report one missing block per run.
#[test]
fn every_block_an_adr_writes_out_is_in_the_tree() {
    let refused: Vec<&str> = BLOCKS
        .iter()
        .filter(|(_, text)| {
            let mut sources = SourceMap::new();
            nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text)
                .1
                .is_err()
        })
        .map(|(label, _)| *label)
        .collect();

    assert!(
        refused.is_empty(),
        "`rule:config/every-block-is-argued-where-it-is-added` names these blocks and the tree refuses them: {refused:?}",
    );
    assert_eq!(
        BLOCKS.len(),
        33,
        "a block was added to or removed from the sweep without the count moving",
    );
}

/// `rule:http-server/a-mount-carries-no-policy`: a mount routes and carries nothing else. § 3's routing keys say where a request
/// arrives and which file answers it; every directive saying what the code answering it *may do*
/// belongs to `rule:config/an-application-is-its-entry-file-path`'s `[[app]]` block, keyed on the entry file path. A `[[server.mount]]`
/// that grew one would be a second home for a fact `rule:config/three-changeability-classes` owns, and the per-app block
/// re-implemented one block over.
///
/// Asserted on both sides, because either half alone reads as correct: § 3's routing keys parse
/// together, and each policy directive is refused under `[[server.mount]]` **while the same
/// directive parses under `[[app]]`** — so a refusal that came from the directive being unspellable
/// anywhere, rather than from the mount declining to hold policy, fails here too.
///
#[test]
fn a_mount_carries_no_policy_of_its_own() {
    let routing = tree(concat!(
        "[[server.mount]]\n",
        "scan = \"*/public/index.nvs\"\n",
        "prefix = \"/{1}\"\n",
        "host = \"{1}.example.com\"\n",
        "origin = \"https://{1}.example.com\"\n",
        "[[server.mount]]\n",
        "prefix = \"/admin\"\n",
        "entry = \"Backoffice/public/index.nvs\"\n",
    ));
    let mounts = routing.server.expect("[server] parses").mount;
    assert_eq!(
        mounts.len(),
        2,
        "§ 3's five routing keys are the whole of what a mount holds",
    );

    // One policy directive per row: how it would be written under a mount, and the `[[app]]`
    // spelling that is its real home. The `[[app]]` half is the roster's own, so a directive that
    // moved keeps this test honest rather than turning it green by going missing everywhere.
    const POLICY: &[(&str, &str)] = &[
        ("mode = \"production\"\n", "mode = \"production\"\n"),
        (
            "[server.mount.limits]\nwall_time = \"600s\"\n",
            "[app.limits]\nwall_time = \"600s\"\n",
        ),
        (
            "[server.mount.limits.hard]\nmemory = \"1G\"\n",
            "[app.limits.hard]\nmemory = \"1G\"\n",
        ),
        (
            "[server.mount.capabilities]\nprocess.exec = true\n",
            "[app.capabilities]\nprocess.exec = true\n",
        ),
        (
            "[server.mount.log]\nlevel = \"warning\"\n",
            "[app.log]\nlevel = \"warning\"\n",
        ),
    ];

    for (on_the_mount, on_the_app) in POLICY {
        let diagnostic = refusal(&format!(
            "[[server.mount]]\nprefix = \"/shop\"\nentry = \"shop/public/index.nvs\"\n{on_the_mount}"
        ));
        assert_eq!(
            diagnostic.code,
            Some(code::E_BAD_DIRECTIVE),
            "a policy directive under a mount is an unknown key like any other: {on_the_mount:?}",
        );
        assert!(
            diagnostic.message.contains("unknown field"),
            "and the message says so: {:?}",
            diagnostic.message,
        );

        let app = tree(&format!("[[app]]\nroot = \"/srv/www/shop\"\n{on_the_app}"));
        assert_eq!(
            app.app.len(),
            1,
            "and `rule:config/an-application-is-its-entry-file-path`'s block is where it does parse: {on_the_app:?}",
        );
    }
}

/// `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`: an unknown key is refused, under the same code a bad value gets, and the refusal
/// says which **block** it was found in — `unknown field \`memory\`` is unreadable until you know it
/// was written under `[metrics]`.
// covers: tools:config/the-blocks-the-binary-accepts
#[test]
fn an_unknown_key_is_refused_naming_the_block_that_has_no_such_directive() {
    let diagnostic = refusal("[metrics]\nexporter = false\nmemory = \"256M\"\n");

    assert_eq!(diagnostic.code, Some(code::E_BAD_DIRECTIVE));
    assert!(
        diagnostic.message.contains("unknown field"),
        "the message should say what was wrong: {:?}",
        diagnostic.message,
    );
    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("[metrics]")),
        "the refusal should name the block the key was found in: {:?}",
        diagnostic.notes,
    );
    assert!(
        diagnostic.primary_span().is_some(),
        "and the line, which is what `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one` asks for",
    );
}

/// The typo § 3 names by name. It is refused at the root table, where there is no enclosing block,
/// so the note is absent rather than wrong — a refusal claiming a block it did not find would be
/// worse than one that stays quiet.
// covers: tools:config/the-blocks-the-binary-accepts
#[test]
fn a_typoed_block_header_is_refused_and_claims_no_block() {
    let diagnostic = refusal("[capabilties]\nscript.spawn = [\"/srv\"]\n");

    assert_eq!(diagnostic.code, Some(code::E_BAD_DIRECTIVE));
    assert!(
        diagnostic.message.contains("capabilties"),
        "the message should name the typo: {:?}",
        diagnostic.message,
    );
    assert!(
        !diagnostic
            .notes
            .iter()
            .any(|note| note.contains("[capabilties]")),
        "a block header that is itself the unknown key is not a block to blame: {:?}",
        diagnostic.notes,
    );
}

/// `rule:config/lists-are-arrays-and-repeated-records-are-arrays-of-tables`: a capability's name is dotted, and a dotted TOML key *is* table nesting, so the two
/// spellings are the same input. Asserted as agreement rather than as two separate answers — a tree
/// that grew a second path for one of them fails here while both still look right alone.
// covers: tools:config/capabilities
#[test]
fn the_two_spellings_of_a_dotted_capability_agree() {
    let dotted = tree("[capabilities]\nscript.spawn = [\"/srv/www/jobs\"]\n");
    let nested = tree("[capabilities.script]\nspawn = [\"/srv/www/jobs\"]\n");

    assert_eq!(dotted, nested);
    assert_eq!(
        dotted
            .capabilities
            .and_then(|caps| caps.script)
            .and_then(|script| script.spawn),
        Some(Setting::List(vec!["/srv/www/jobs".to_string()])),
    );
}

/// `rule:config/three-changeability-classes`: `[limits.hard] memory = false` removes the ceiling entirely, and `rule:config/the-file-is-nvs-toml-and-it-is-toml` chose
/// TOML partly because "off" is a boolean there. A field typed `String` refuses the ADR's own
/// example, so the bound is asserted on both sides — the size *and* the `false`.
#[test]
fn a_ceiling_is_a_size_or_the_boolean_that_removes_it() {
    let sized = tree("[limits.hard]\nmemory = \"2G\"\n");
    let removed = tree("[limits.hard]\nmemory = false\n");

    let hard = |config: Config| {
        config
            .limits
            .and_then(|limits| limits.hard)
            .and_then(|hard| hard.memory)
    };
    assert_eq!(hard(sized), Some(Setting::Text("2G".to_string())));
    assert_eq!(hard(removed), Some(Setting::Bool(false)));
}

/// The repository's own `nvs.toml` is the tree's first real input, and it is what
/// `examples/routes.nvs` reads its origin out of. A roster that cannot hold the file in the
/// repository root is not a roster.
#[test]
fn the_repositorys_own_config_deserializes() {
    let text = std::fs::read_to_string(nvs_repo::path("nvs.toml"))
        .expect("the repository root holds an nvs.toml");
    let config = tree(&text);

    let app = config
        .app
        .first()
        .expect("`rule:config/an-application-is-its-entry-file-path`'s `[[app]]` block");
    assert_eq!(app.root.as_deref(), Some("."));
    assert_eq!(app.origin.as_deref(), Some("https://example.test"));
    assert_eq!(
        config
            .limits
            .and_then(|limits| limits.hard)
            .and_then(|hard| hard.memory),
        Some(Setting::Text("512M".to_string())),
        "and the ceiling `examples/config.nvs` is written around",
    );
}

/// The disk, as `nvs run` reads it: the canonical path in place of the ownership check, which a
/// checkout on a Windows data drive fails, and every other question asked of the real disk.
struct Checkout;

impl nvs_config::resolve::Files for Checkout {
    fn trust(&self, path: &Path) -> Result<PathBuf, nvs_config::trust::Untrusted> {
        Disk.canonical(path)
            .map_err(nvs_config::trust::Untrusted::Unreadable)
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        Disk.canonical(path)
    }

    fn canonical_block(&self, path: &Path) -> Result<PathBuf, String> {
        Disk.canonical_block(path)
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        Disk.read(path)
    }

    fn read_config(&self, path: &Path) -> Result<String, String> {
        Disk.read_config(path)
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        Disk.read_bytes(path)
    }

    fn exposure(&self, path: &Path) -> Option<String> {
        Disk.exposure(path)
    }

    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        Disk.list(dir)
    }

    fn exists(&self, path: &Path) -> bool {
        Disk.exists(path)
    }
}

/// The repository's own `nvs.toml` resolves as a run from the repository root resolves it: every
/// `[[app]]` block is keyed on a path that is there, no two blocks share a path, none asks for more
/// than the global ceiling, and nothing is only advised. A program records the blocks that could
/// match its own entry file and no others, so this test is what runs when a block that no program
/// matches is added or changed, or when the path a block names is removed.
#[test]
fn the_repositorys_own_config_resolves_every_block_without_a_warning() {
    let mut sources = SourceMap::new();
    let roots = Roots::Files(vec![nvs_repo::path("nvs.toml")]);
    let resolved =
        nvs_config::resolve::resolve(&roots, &mut sources, &Checkout).unwrap_or_else(|err| {
            panic!(
                "the repository's nvs.toml does not resolve: {}",
                err.message
            )
        });
    nvs_config::app::record_roster(&resolved.config);

    assert!(
        !resolved.config.app.is_empty(),
        "the file has `[[app]]` blocks"
    );
    let warnings: Vec<&str> = resolved
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect();
    assert!(
        warnings.is_empty(),
        "the repository's nvs.toml raises advisories: {warnings:?}"
    );
}

/// A run records the repository's `nvs.toml` by part: a `config` line for its global tables, and an
/// `app` line for its entry file and each directory above it. It records no `file` line for the
/// configuration, and no test of the paths the other `[[app]]` blocks are keyed on.
#[test]
fn a_run_records_the_configuration_by_part() {
    let config = nvs_repo::path("nvs.toml");
    let entry = nvs_repo::path("examples/hello.nvs");
    let other = nvs_repo::path("examples/capability.nvs");
    let (matched, lines) = nvs_footprint::capture(|| {
        let mut sources = SourceMap::new();
        let resolved = nvs_config::resolve::resolve(
            &Roots::Files(vec![config.clone()]),
            &mut sources,
            &Checkout,
        )
        .expect("the repository's nvs.toml resolves");
        nvs_config::app::matching(&resolved.config.app, &entry, &Checkout)
            .expect("the entry file is there")
    });
    assert!(!matched.is_empty(), "the `root = \".\"` block matches");

    let shown = |path: &Path| nvs_footprint::shown(&Disk.canonical(path).expect("there"));
    let line = |kind: &str, path: &Path| format!("{kind}\t{}", shown(path));
    assert!(lines.contains(&line("config", &config)), "{lines:#?}");
    assert!(!lines.contains(&line("file", &config)), "{lines:#?}");
    assert!(lines.contains(&line("app", &entry)), "{lines:#?}");
    let above = entry.parent().expect("the entry file is in a directory");
    assert!(lines.contains(&line("app", above)), "{lines:#?}");
    assert!(!lines.contains(&line("exists", &other)), "{lines:#?}");
}

/// An empty file is a legal configuration, which is what makes an `[[include]]` of a placeholder
/// legal rather than a parse failure (`rule:config/include-takes-a-path-or-a-dir`).
#[test]
fn an_empty_file_is_the_default_tree() {
    assert_eq!(tree(""), Config::default());
}

/// `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s *other* refusal still holds over the typed tree: a duplicate key inside one file
/// is an error, and it is a distinct code from an unknown one because it is a distinct mistake.
#[test]
fn a_duplicate_key_is_still_its_own_refusal_over_the_typed_tree() {
    let diagnostic = refusal("[limits]\nmemory = \"128M\"\nmemory = \"256M\"\n");

    assert_eq!(diagnostic.code, Some(code::E_DUPLICATE_DIRECTIVE));
}
