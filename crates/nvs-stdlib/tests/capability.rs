//! [ADR 0118]'s four claims, as tests: the refusal names the capability, a path escape does not
//! match a granted root, the declaration table names real members, and no member reaches the
//! operating system except through the door.
//!
//! The path cases split deliberately. The `..` half runs against the **real** filesystem, because a
//! canonicalizer that resolves `..` textually rather than by asking the OS is exactly the bug the rule
//! exists to prevent and a fake would hide it. The symlink half runs against a fake canonicalizer, for
//! a reason that is about the host and not about the rule: creating a symlink on Windows needs a
//! privilege CI does not have, and skipping the case there would leave the claim untested on the
//! platform the loop runs on. What the fake asserts is the same thing the symlink would — that
//! `allows` compares the canonicalizer's *answer* and never the argument it was handed.
//!
//! [ADR 0118]: ../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md

use std::path::{Path, PathBuf};

use nvs_config::capability::{Cap, Scope};
use nvs_config::resolve::{Disk, Files};
use nvs_config::tree::{CapFs, Capabilities, Setting};

/// The capabilities a test grants: `fs.read` under `roots` and nothing else at all.
fn reading(roots: &[&str]) -> Capabilities {
    Capabilities {
        fs: Some(CapFs {
            read: Some(Setting::List(roots.iter().map(|&r| r.to_owned()).collect())),
            write: None,
        }),
        ..Capabilities::default()
    }
}

fn canonical(path: &Path) -> String {
    Disk.canonical(path)
        .expect("the crate's own directories exist")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn an_ungranted_capability_throws_naming_the_capability() {
    // The grant side of § 1, on a configuration that says yes to one capability and nothing about
    // its neighbour: `fs.read` is granted, `fs.write` shares the block and is not.
    let caps = reading(&["/srv/www"]);
    assert!(caps.allows(Cap::FsRead, Scope::Unscoped, &Disk));
    assert!(!caps.allows(Cap::FsWrite, Scope::Unscoped, &Disk));
    // Deny by default at every step: an absent block grants nothing either.
    assert!(!Capabilities::default().allows(Cap::ScriptSpawn, Scope::Unscoped, &Disk));

    // § 5's refusal, on a context with no configuration — the same deny-by-default an absent block
    // gets, and the message an operator reads.
    let ctx = nvs_runtime::Ctx::buffered();
    let path = Path::new("/var/tmp/x");
    let denied = nvs_runtime::capability::require(
        &ctx,
        Cap::FsWrite,
        Scope::Path(path),
        "Core\\File::write",
    )
    .expect_err("an unconfigured context grants nothing");
    let nvs_runtime::Fault::Thrown(class, message) = denied else {
        panic!("a denial is a throw, never a fatal or a pending status — ADR 0118 § 5");
    };
    assert_eq!(class, nvs_runtime::ThrownClass::Runtime);
    assert!(
        message.contains("fs.write"),
        "the message names the capability in the spelling `nvs.toml` grants it under: {message}"
    );
    assert!(
        message.contains("Core\\File::write") && message.contains("/var/tmp/x"),
        "and names who wanted it and what for: {message}"
    );
}

#[test]
fn a_path_reaching_a_granted_root_through_dotdot_or_a_symlink_does_not_match() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");
    let caps = reading(&[&canonical(&src)]);

    // The positive control, without which every assertion below passes vacuously.
    assert!(caps.allows(Cap::FsRead, Scope::Path(&src.join("registry.rs")), &Disk));

    // § 4, on the real filesystem: `..` is resolved by the OS before the comparison, so a path that
    // spells its way back out of the granted root does not reach anything under it.
    let escaped = src.join("..").join("tests").join("capability.rs");
    assert!(
        !caps.allows(Cap::FsRead, Scope::Path(&escaped), &Disk),
        "a granted root is not a prefix of what `..` walks out to"
    );

    // The symlink half, and the component-wise half, against a canonicalizer that resolves what a
    // real one would. `/granted/link` is the symlink; `/grantedmore` is the sibling whose name
    // merely starts the same way, which a byte-wise prefix test would accept.
    let fake = Fake;
    let caps = reading(&["/granted"]);
    assert!(caps.allows(Cap::FsRead, Scope::Path(Path::new("/granted/own")), &fake));
    assert!(
        !caps.allows(Cap::FsRead, Scope::Path(Path::new("/granted/link")), &fake),
        "a symlink under a granted root does not carry the grant to its target"
    );
    assert!(
        !caps.allows(Cap::FsRead, Scope::Path(Path::new("/grantedmore/x")), &fake),
        "`starts_with` compares whole components, so a name prefix is not a path prefix"
    );

    // § 4's write case: a file that does not exist yet resolves through its deepest existing
    // ancestor, so a create inside the root is allowed and one that `..` walks out of is not —
    // without either path having to exist first.
    let caps = reading(&[&canonical(manifest)]);
    assert!(caps.allows(
        Cap::FsRead,
        Scope::Path(&src.join("not-written-yet.rs")),
        &Disk
    ));
    assert!(!caps.allows(
        Cap::FsRead,
        Scope::Path(&manifest.join("..").join("..").join("not-written-yet.rs")),
        &Disk
    ));
}

#[test]
fn every_capability_entry_names_a_member() {
    for (class_name, member, cap) in nvs_stdlib::registry::CAPABILITIES {
        let class = nvs_stdlib::registry::CLASSES
            .iter()
            .find(|class| class.name == *class_name)
            .unwrap_or_else(|| panic!("`{class_name}` (for `{}`) is not a Core class", cap.name()));
        assert!(
            class.members().any(|found| found.name == *member),
            "`{class_name}::{member}` is not a member of that class"
        );
    }
}

#[test]
fn nvs_stdlib_reaches_the_os_only_through_the_gate() {
    // ADR 0118 § 2: a `Core` member cannot forget the check, because the only way to perform the
    // effect is a door in `nvs_runtime` that has already asked. This is what holds that door shut.
    //
    // The list is the effect-performing spellings, not the modules: `std::net::IpAddr` is a parser
    // (`Core\Validate::isIp` uses it and touches no socket) and `std::process::abort` is the bottom
    // of the fatal ladder rather than a way out to the operating system.
    const FORBIDDEN: &[&str] = &[
        "std::fs",
        "std::net::TcpStream",
        "std::net::TcpListener",
        "std::net::UdpSocket",
        "std::process::Command",
        "std::env::var",
        "std::env::args",
        "std::env::set_var",
        "std::env::current_dir",
    ];
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in sources(&src) {
        let text = std::fs::read_to_string(&file).expect("a source file this crate compiled");
        for (number, line) in text.lines().enumerate() {
            // A comment may name one — telling an author what to use *instead* is this rule's own
            // documentation, and it is the line below that matters.
            if line.trim_start().starts_with("//") {
                continue;
            }
            for spelling in FORBIDDEN {
                assert!(
                    !line.contains(spelling),
                    "{}:{} reaches the operating system directly (`{spelling}`); \
                     ADR 0118 § 2 puts that behind a door in `nvs_runtime::capability`",
                    file.display(),
                    number + 1
                );
            }
        }
    }
}

fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("this crate's own source directory") {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found
}

/// A canonicalizer with one symlink in it, and nothing else — see this file's own module doc for
/// why the symlink half of § 4 is asserted against a fake and the `..` half is not.
struct Fake;

impl Files for Fake {
    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        if path == Path::new("/granted/link") {
            return Ok(PathBuf::from("/elsewhere/secret"));
        }
        Ok(path.to_path_buf())
    }

    fn trust(&self, _path: &Path) -> Result<PathBuf, nvs_config::trust::Untrusted> {
        unreachable!("a capability check canonicalizes and never asks about ownership")
    }

    fn read(&self, _path: &Path) -> Result<String, String> {
        unreachable!("a capability check reads no bytes")
    }

    fn read_bytes(&self, _path: &Path) -> Result<Vec<u8>, String> {
        unreachable!("a capability check reads no bytes")
    }

    fn exposure(&self, _path: &Path) -> Option<String> {
        unreachable!("a capability check reads no bytes")
    }

    fn list(&self, _dir: &Path) -> Result<Vec<PathBuf>, String> {
        unreachable!("a capability check lists no directory")
    }

    fn exists(&self, _path: &Path) -> bool {
        unreachable!("a capability check asks the canonicalizer, which answers for absence too")
    }
}
