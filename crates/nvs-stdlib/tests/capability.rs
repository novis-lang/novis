//! [ADR 0118]'s claims, as tests: the refusal names the capability, a path escape does not match a
//! granted root, the declaration table names real members, every member of a class that bears one
//! declares its own, and no member reaches the operating system except through the door.
//!
//! The last two are § 7's closure claim and § 2's, and neither subsumes the other: the first catches
//! a member that goes through a door without being declared as doing so, the second a member that
//! reaches the operating system with no door at all.
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
    let denied =
        nvs_runtime::capability::require(&ctx, Cap::FsWrite, Scope::Path(path), "Core\\IO::write")
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
        message.contains("Core\\IO::write") && message.contains("/var/tmp/x"),
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

/// A context granting `fs.read` under `roots` and nothing else, with the roots canonicalized the
/// way [`nvs_config::tree::Capabilities::canonicalize`] does when a real snapshot is built — a root
/// still spelled the way this file typed it is a comparison against the wrong thing.
fn ctx_reading(roots: &[&str]) -> nvs_runtime::Ctx {
    let mut caps = reading(roots);
    caps.canonicalize(&Disk);
    let mut ctx = nvs_runtime::Ctx::buffered();
    ctx.set_config(std::sync::Arc::new(nvs_config::Snapshot {
        config: nvs_config::tree::Config {
            capabilities: Some(caps),
            ..nvs_config::tree::Config::default()
        },
        ..nvs_config::Snapshot::default()
    }));
    ctx
}

/// `Core\IO::within($base, $path)` as the member itself, not as a re-derivation of it: the answer,
/// or the message the refusal carried.
fn within(ctx: &mut nvs_runtime::Ctx, base: &Path, path: &str) -> Result<String, String> {
    let args = [
        nvs_runtime::Value::str(nvs_runtime::NvsStr::new(base.to_string_lossy().as_bytes())),
        nvs_runtime::Value::str(nvs_runtime::NvsStr::new(path.as_bytes())),
    ];
    match nvs_runtime::call(nvs_stdlib::io::nvs_core_io_within, ctx, &args) {
        Ok(answer) => Ok(answer
            .as_text()
            .expect("`within` answers a `string` or throws")
            .to_owned()),
        Err(status) => {
            assert_eq!(
                status,
                nvs_runtime::THROWN,
                "every refusal `within` makes is catchable — ADR 0118 § 5 for the capability half, \
                 ADR 0024 § 3 for the containment half"
            );
            Err(ctx
                .take_pending()
                .expect("a throw leaves its message on the context")
                .into_owned())
        }
    }
}

#[test]
fn within_resolves_and_then_proves_containment() {
    // Spec § 14's *Resolution* bullet over ADR 0024 § 3: the launderer answers a path that is
    // resolved — every `..` and every symlink already gone — and proved to be under the base. Both
    // halves matter, and the order is what separates this from `Core\Path::normalize`.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");
    let mut ctx = ctx_reading(&[&canonical(manifest)]);

    // Resolved: the answer is the canonical spelling and not the argument re-joined. That is the
    // property a caller relies on when it hands the result to a door.
    let answer = within(&mut ctx, &src, "registry.rs").expect("a name inside the base");
    assert_eq!(answer, canonical(&src.join("registry.rs")));

    // And resolved through a `..` that stays inside, which is the case a member refusing every `..`
    // textually would get wrong in the safe direction and still get wrong.
    let inside = within(&mut ctx, &src, "../src/registry.rs").expect("a detour that comes back");
    assert_eq!(inside, answer);

    // A name that does not exist yet resolves through its deepest existing ancestor, so `within`
    // answers for a path a program is about to create rather than only for one it can already open.
    let fresh = within(&mut ctx, &src, "not-written-yet.rs").expect("a name inside the base");
    assert!(
        fresh.starts_with(&canonical(&src)),
        "{fresh} is not under {}",
        canonical(&src)
    );
}

#[test]
fn within_refuses_a_path_that_escapes_through_dotdot_or_a_symlink() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");

    // Granted wide enough that every refusal below is about *containment* and not about the grant —
    // the two are both `RuntimeError` and a case that could not tell them apart would pass on the
    // wrong one.
    let mut ctx = ctx_reading(&[&canonical(manifest)]);
    let refused = |ctx: &mut nvs_runtime::Ctx, path: &str| {
        let message = within(ctx, &src, path).expect_err("this path leaves the base");
        assert!(
            message.contains("a path must stay inside the base it is resolved against"),
            "the refusal is the containment one: {message}"
        );
        message
    };

    // The `..` escape, resolved by the operating system before the comparison.
    let message = refused(&mut ctx, "../tests/capability.rs");
    assert!(
        !message.contains(&canonical(&manifest.join("tests").join("capability.rs"))),
        "the message names the base and the caller's own argument, never the path that argument \
         resolved to — where a symlink pointed is exactly what a refusal must not disclose: \
         {message}"
    );

    // An absolute path is not an escape hatch. It replaces the base when it is joined, resolves,
    // and then fails the same check.
    refused(&mut ctx, &manifest.to_string_lossy());

    // Containment is component-wise, so a sibling whose name merely starts the same way is out.
    // (The symlink half of this claim is asserted one layer down, against the `Fake` canonicalizer
    // in `a_path_reaching_a_granted_root_through_dotdot_or_a_symlink_does_not_match`: `within` and
    // the grant check resolve through the same `nvs_config::capability::resolved`, and creating a
    // real symlink needs a privilege CI does not have on Windows.)
    refused(&mut ctx, "../srcmore/thing.rs");

    // And with the grant narrowed to the base itself, the capability refusal arrives *first* — a
    // path outside the grant is refused whether or not it would also have escaped, so `within` is
    // not a way to learn what is outside a granted root.
    let mut narrow = ctx_reading(&[&canonical(&src)]);
    let message = within(&mut narrow, &src, "../tests/capability.rs")
        .expect_err("outside the grant as well as outside the base");
    assert!(
        message.contains("fs.read"),
        "the earlier refusal names the capability an operator would have to grant: {message}"
    );
}

#[test]
fn every_capability_entry_names_a_member() {
    for (class_name, member, cap) in nvs_stdlib::registry::CAPABILITIES {
        let class = nvs_stdlib::registry::CLASSES
            .iter()
            .find(|class| class.name == *class_name)
            .unwrap_or_else(|| {
                let declares = cap.map_or("no capability", |declared| declared.name());
                panic!("`{class_name}` (for `{declares}`) is not a Core class")
            });
        assert!(
            class.members().any(|found| found.name == *member),
            "`{class_name}::{member}` is not a member of that class"
        );
    }
}

/// ADR 0118 § 7's closure, and it has no exception list to read: a member of a capability-bearing
/// class that genuinely needs none declares `None` in `registry::CAPABILITIES`, in the same table
/// under the same review as one that needs `fs.read`.
///
/// **The allowlist that used to sit here is gone rather than frozen.** § 7 forbids growing one to
/// make a run go green — a member that is hard to classify is a member whose capability has not been
/// thought about — and `Core\RateLimit::shed` was the sibling that had no other spelling: it reaches
/// nothing, and its class is a door because `consume` is. A list of two would have been a set claim
/// with a growable hole in it; a `None` row costs the would-be exemption exactly what a declaration
/// costs and leaves the claim below total. What the two rows say is in the table beside them, which
/// is the one home for a member's reason.
#[test]
fn every_capability_bearing_member_declares_its_capability() {
    // ADR 0118 § 7's direction: not "does every entry name a member" — that is
    // `every_capability_entry_names_a_member` above — but "does every member of a class that reaches
    // the operating system at all say so". A class is capability-bearing when any one of its members
    // is declared, because that is the evidence that the class is a door and not a calculator.
    let bearing: Vec<_> = nvs_stdlib::registry::CLASSES
        .iter()
        .filter(|class| {
            nvs_stdlib::registry::CAPABILITIES
                .iter()
                .any(|(declared, _, _)| *declared == class.name)
        })
        .collect();

    // The positive control, without which the whole set claim below passes by finding nothing to
    // check. `Core\IO` is the class that made § 3's table necessary.
    assert!(
        bearing.iter().any(|class| class.name == "Core\\IO"),
        "`Core\\IO` reaches the filesystem, so it is capability-bearing by construction; \
         a run where it is not means the declaration table has lost its entries"
    );

    for class in bearing {
        for member in class.members() {
            let rows = nvs_stdlib::registry::CAPABILITIES
                .iter()
                .filter(|(name, found, _)| *name == class.name && *found == member.name)
                .count();
            assert_eq!(
                rows, 1,
                "`{}::{}` is a member of a capability-bearing class, so it owes exactly one row in \
                 `registry::CAPABILITIES` — the capability it needs, or `None` beside the comment \
                 saying what it reaches instead. There is no allowlist to add it to, which is ADR \
                 0118 § 7's point: a member that is hard to classify is a member whose capability \
                 has not been thought about",
                class.name, member.name,
            );
        }
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

        // The rule is about what a `Core` member can do at run time, and a `#[cfg(test)]` module is
        // not in a released binary for one to reach: `crate::http::transport`'s cases drive a whole
        // exchange against a listener they open themselves, which is the only way to test a
        // transport at all. So the scan stops at the test module — and asserts there is exactly one
        // of them per file, since a second `#[cfg(test)]` higher up would hide everything under it
        // rather than just the tests.
        assert!(
            text.lines()
                .filter(|line| line.trim_start() == "#[cfg(test)]")
                .count()
                <= 1,
            "{} has more than one `#[cfg(test)]`, so this scan can no longer stop at the first one",
            file.display()
        );
        let shipped = text
            .lines()
            .take_while(|line| line.trim_start() != "#[cfg(test)]");

        for (number, line) in shipped.enumerate() {
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
