//! ADR 0064 § 2a's block roster as a type, and § 3's unknown-key refusal that comes with it.
//!
//! The roster is the security-relevant half: a key this tree does not name is a key an operator may
//! write and no code will ever read, so the sweep below asserts by **counting** the blocks that
//! parse rather than reading one off a line — a tree that grew a typo in one block header still
//! deserializes every other block plausibly.

use nvs_config::{Config, Setting};
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

/// One line of TOML per block ADR 0064 § 2a's table names, in that table's order. Every one must
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
    ("[capabilities]", "[capabilities]\nscript.spawn = [\"/srv/www/jobs\"]\nprocess.exec = true\ndebug.trace = [\"/var/log/nvs/trace\"]\nfs.read = [\"/srv\"]\nfs.write = [\"/var/tmp\"]\nnet.connect = [\"reports.internal\"]\ndb.connect = [\"main\"]\ndb.open = [\"*.tenants.internal\"]\ndebug.profile = [\"/var/log/nvs/profile\"]\n"),
    ("[[extension]]", "[[extension]]\npath = \"image.nvsx\"\nsha256 = \"abc\"\n"),
    ("[debug]", "[debug]\nmode = [\"coverage\", \"branch\"]\n"),
    ("[log]", "[log]\nhandler = \"handler.nvs\"\nhandler_reserve_memory = \"8M\"\nhandler_reserve_time = \"2s\"\ntarget = \"stderr\"\nformat = \"json\"\nlevel = \"warning\"\n"),
    ("[http.errors]", "[http.errors]\ndetail = \"generic\"\n"),
    ("[http.headers]", "[http.headers]\ncontent_type_options = true\nframe_ancestors = \"none\"\nreferrer_policy = \"strict-origin-when-cross-origin\"\nhsts = \"365d\"\nhsts_subdomains = false\ncontent_security_policy = \"\"\npermissions_policy = \"\"\n"),
    ("[http.cors]", "[http.cors]\norigins = []\nmethods = [\"GET\", \"HEAD\", \"POST\"]\nheaders = []\nexpose = []\ncredentials = false\nmax_age = \"10m\"\n"),
    ("[http.cookies]", "[http.cookies]\nsecure = true\nhttp_only = true\nsame_site = \"Lax\"\npath = \"/\"\n"),
    ("[http.client]", "[http.client]\nconnect_timeout = \"5s\"\ndeadline = \"30s\"\nmax_redirects = 0\n"),
    ("[db.<name>]", "[db.main]\ndriver = \"pgsql\"\nhost = \"db\"\nport = 5432\nuser = \"app\"\npassword_file = \"/run/secrets/db\"\ndatabase = \"shop\"\n"),
    ("[db.<name>] sqlite", "[db.local]\npath = \"data/app.sqlite\"\n"),
    ("[deferred]", "[deferred]\nmax_concurrent = 256\ndeadline = \"30s\"\n"),
    ("[[schedule]]", "[[schedule]]\nname = \"nightly-report\"\ncron = \"0 3 * * *\"\nscript = \"jobs/report.nvs\"\nscope = \"fleet\"\ntimezone = \"Europe/Vienna\"\noverlap = \"skip\"\nlimits = {memory = \"512M\", cpu_time = \"120s\"}\ngrants = {net = {connect = [\"reports.internal\"]}}\n"),
    ("[metrics]", "[metrics]\nexporter = \"prometheus\"\nlisten = \"127.0.0.1:9090\"\nendpoint = \"\"\nmax_series = 10000\n"),
    ("[trace]", "[trace]\nexporter = \"otlp\"\nendpoint = \"\"\nsample = 0.01\npropagate = true\n"),
    ("[server]", "[server]\nroot = \"/www\"\nlisten = [\"127.0.0.1:8000\"]\nsocket_mode = \"0660\"\ndispatch = \"entry\"\nstatic = false\ntrusted_proxies = []\nhealth_path = \"\"\nmax_in_flight = 10000\nheader_timeout = \"10s\"\nbody_idle_timeout = \"30s\"\nwrite_idle_timeout = \"30s\"\nkeepalive_timeout = \"75s\"\n"),
    ("[[server.mount]]", "[[server.mount]]\nscan = \"*/public/index.nvs\"\nprefix = \"/{1}\"\norigin = \"https://{1}.example.com\"\n"),
    ("[[server.mount]] entry", "[[server.mount]]\nprefix = \"/admin\"\nentry = \"Backoffice/public/index.nvs\"\nhost = \"admin.example.com\"\n"),
    ("[cache]", "[cache]\ndir = \"/var/cache/nvs\"\n"),
    ("[control]", "[control]\nsocket = \"/run/nvs/control.sock\"\n"),
    ("[opcache]", "[opcache]\nvalidate = \"never\"\nrevalidate_freq = \"2s\"\nfile_cache = true\nfile_cache_dir = \"/var/cache/nvs\"\nfile_cache_max_size = \"1G\"\nfile_cache_gc_probability = 1\nfile_cache_gc_divisor = 100\n"),
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
        "ADR 0064 § 2a names these blocks and the tree refuses them: {refused:?}",
    );
    assert_eq!(
        BLOCKS.len(),
        30,
        "a block was added to or removed from the sweep without the count moving",
    );
}

/// ADR 0064 § 3: an unknown key is refused, under the same code a bad value gets, and the refusal
/// says which **block** it was found in — `unknown field \`memory\`` is unreadable until you know it
/// was written under `[metrics]`.
#[test]
fn an_unknown_key_is_refused_naming_its_block() {
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
        "and the line, which is what ADR 0064 § 3 asks for",
    );
}

/// The typo § 3 names by name. It is refused at the root table, where there is no enclosing block,
/// so the note is absent rather than wrong — a refusal claiming a block it did not find would be
/// worse than one that stays quiet.
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

/// ADR 0064 § 2: a capability's name is dotted, and a dotted TOML key *is* table nesting, so the two
/// spellings are the same input. Asserted as agreement rather than as two separate answers — a tree
/// that grew a second path for one of them fails here while both still look right alone.
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

/// ADR 0005: `[limits.hard] memory = false` removes the ceiling entirely, and ADR 0064 § 1 chose
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
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../nvs.toml"),
    )
    .expect("the repository root holds an nvs.toml");
    let config = tree(&text);

    let app = config.app.first().expect("ADR 0104 § 1's `[[app]]` block");
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

/// An empty file is a legal configuration, which is what makes an `[[include]]` of a placeholder
/// legal rather than a parse failure (ADR 0103 § 2).
#[test]
fn an_empty_file_is_the_default_tree() {
    assert_eq!(tree(""), Config::default());
}

/// ADR 0064 § 3's *other* refusal still holds over the typed tree: a duplicate key inside one file
/// is an error, and it is a distinct code from an unknown one because it is a distinct mistake.
#[test]
fn a_duplicate_key_is_still_its_own_refusal_over_the_typed_tree() {
    let diagnostic = refusal("[limits]\nmemory = \"128M\"\nmemory = \"256M\"\n");

    assert_eq!(diagnostic.code, Some(code::E_DUPLICATE_DIRECTIVE));
}
