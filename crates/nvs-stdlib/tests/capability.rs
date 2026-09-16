//! `rule:security/capability-check-at-the-door`'s claims, as tests: the refusal names the capability, a path escape does not match a
//! granted root, the declaration table names real members, every member of a class that bears one
//! declares its own, no member reaches the operating system except through the door, and no member
//! reads a URI scheme off a path.
//!
//! The middle two are § 7's closure claim and § 2's, and neither subsumes the other: the first
//! catches a member that goes through a door without being declared as doing so, the second a
//! member that reaches the operating system with no door at all. The last is
//! `rule:security/a-path-is-not-a-url`'s rather than 0118's, and sits here
//! because it is the same question one layer up: a door that asks about the path it was handed is
//! no protection if the path the caller wrote names somewhere else entirely.
//!
//! The path cases split deliberately. The `..` half runs against the **real** filesystem, because a
//! canonicalizer that resolves `..` textually rather than by asking the OS is exactly the bug the rule
//! exists to prevent and a fake would hide it. The symlink half runs against a fake canonicalizer, for
//! a reason that is about the host and not about the rule: creating a symlink on Windows needs a
//! privilege CI does not have, and skipping the case there would leave the claim untested on the
//! platform the loop runs on. What the fake asserts is the same thing the symlink would — that
//! `allows` compares the canonicalizer's *answer* and never the argument it was handed.
//!

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
        panic!(
            "a denial is a throw, never a fatal or a pending status — `rule:security/denial-is-a-runtime-error`"
        );
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
                "every refusal `within` makes is catchable — `rule:security/denial-is-a-runtime-error` for the capability half, \
                 `rule:security/launderers-are-sink-named` for the containment half"
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
    // Spec § 14's *Resolution* bullet over `rule:security/launderers-are-sink-named`: the launderer answers a path that is
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

/// `rule:testing/capability-closure-test`'s closure, and it has no exception list to read: a member of a capability-bearing
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
    // `rule:testing/capability-closure-test`'s direction: not "does every entry name a member" — that is
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
    // `rule:security/capability-check-at-the-door`: a `Core` member cannot forget the check, because the only way to perform the
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
                     `rule:security/capability-check-at-the-door` puts that behind a door in `nvs_runtime::capability`",
                    file.display(),
                    number + 1
                );
            }
        }
    }
}

/// `rule:security/a-path-is-not-a-url`'s closed door — a path is a filesystem path — asserted **by construction** rather
/// than by a blocklist of the schemes PHP shipped. `phar://` is not named as forbidden anywhere
/// below, because a rule that refused a roster of scheme names would be exactly the registry § 2
/// refuses to have: what is asserted is that nothing reads a scheme off a path at all.
///
/// Three claims, each total over what it sweeps. **The signature half:** a member that dispatched
/// on a scheme would have to take one, and no member of a path-taking class names a scheme, a
/// wrapper, a protocol or a URL in its signature column. **The implementation half:** those
/// classes' modules contain no `://` on any line a release build compiles. **The behavioural
/// half:** `php://filter/resource=registry.rs` names no file this crate has, at the launderer and
/// at the door alike.
///
/// What is path-taking is *derived*, not listed, and the **member** is the unit: a member with an
/// `fs.read`/`fs.write` row in [`nvs_stdlib::registry::CAPABILITIES`], plus all of `Core\Path`'s,
/// which has no row precisely because it touches nothing. Pure string algebra over paths is where a
/// textual scheme parse would hide, and no capability row can find it. A class is the unit for the
/// implementation half alone, where the subject is a module and not a signature.
///
/// The member rather than its class, because one class holds both: `Core\Response::sendFile`
/// reaches the filesystem and `Core\Response::redirect` names a destination the *peer* fetches,
/// which is a URL by `rule:security/a-path-is-not-a-url`'s own reading and carries the spelling
/// legitimately. Sweeping the class would refuse the one member on the other one's account.
///
/// The spellings this refuses are legitimate one class over, which is what makes the sweep a claim
/// about *paths* rather than a ban on a word: `Core\Uri` has a `scheme` member because a URI is its
/// subject, and `Core\Http::allowUrl` takes a URL because `rule:http-server/allow-url-pins-the-address`'s outbound door is where a URL
/// belongs. Neither takes a path.
///
#[test]
fn no_member_dispatches_on_a_uri_scheme() {
    const SPELLINGS: &[&str] = &["scheme", "wrapper", "protocol", "url", "uri"];

    /// Whether this member is one that opens a name — the row it has in the roster, which is the
    /// only machine-readable statement that a parameter of it is a path.
    fn opens_a_name(class: &str, member: &str) -> bool {
        nvs_stdlib::registry::CAPABILITIES
            .iter()
            .any(|(name, row, cap)| {
                *name == class && *row == member && matches!(cap, Some(Cap::FsRead | Cap::FsWrite))
            })
    }

    /// Whether this member's signature is swept — `Core\Path`'s whole roster is paths, and
    /// everywhere else the row is what says so.
    fn takes_a_path(class: &str, member: &str) -> bool {
        class == "Core\\Path" || opens_a_name(class, member)
    }

    let path_taking: Vec<_> = nvs_stdlib::registry::CLASSES
        .iter()
        .filter(|class| {
            class
                .members()
                .any(|member| takes_a_path(class.name, member.name))
        })
        .collect();

    // The positive control on the derivation itself: the class that reaches the filesystem and the
    // class that is nothing but paths are both in it, so neither half below sweeps an empty set.
    for expected in ["Core\\IO", "Core\\Path"] {
        assert!(
            path_taking.iter().any(|class| class.name == expected),
            "`{expected}` takes a path, so it is what this test is about; a run where it is not in \
             the derived set means the derivation has stopped finding the classes it is a claim over"
        );
    }

    // And the control on the unit, which is the half a class-wide sweep cannot state: the one class
    // holding a member of each kind has exactly one of them in the derived set.
    assert!(
        opens_a_name("Core\\Response", "sendFile"),
        "`Core\\Response::sendFile` is the body member that opens a file, so it is swept here; a \
         run where it is not means its `fs.read` row has gone and the sweep is no longer a claim \
         about it"
    );
    assert!(
        !opens_a_name("Core\\Response", "redirect"),
        "`redirect` names a destination the peer fetches and opens nothing, which is why it may \
         spell a URL; a row granting it the filesystem would make it a path member with a `url` \
         parameter, which is the shape this test exists to refuse"
    );

    for class in &path_taking {
        for member in class.members() {
            if !takes_a_path(class.name, member.name) {
                continue;
            }
            let names = std::iter::once(member.name).chain(member.names.iter().copied());
            for name in names {
                let lowered = name.to_ascii_lowercase();
                for spelling in SPELLINGS {
                    assert!(
                        !lowered.contains(spelling),
                        "`{}::{}` takes a path, and `{name}` names a {spelling}; `rule:security/a-path-is-not-a-url` \
                         refuses dispatch on the textual content of a path, so there is no \
                         argument for a member like this to read one from",
                        class.name,
                        member.name
                    );
                }
            }
        }
    }

    // And the contrast that keeps the sweep from being a ban on a word nobody writes: `Core\Uri`
    // holds the spelling, on the class whose subject is a URI and which touches no disk.
    let uri = nvs_stdlib::registry::CLASSES
        .iter()
        .find(|class| class.name == "Core\\Uri")
        .expect("`Core\\Uri` is the class a scheme belongs to");
    assert!(
        uri.members().any(|member| member.name == "scheme"),
        "a scheme is a URI's component and `Core\\Uri` is where it is read; a run where it is not \
         means the sweep above forbids a spelling the roster no longer contains"
    );

    // The implementation half. `://` is the structural spelling of a scheme prefix and not one
    // scheme's name — a bare `:` is not the pattern, because `Core\Path` parses a Windows drive
    // letter on every platform (see its module doc) and reads one legitimately.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    /// The file under `src` that implements `class`.
    ///
    /// A nested class is written in one of three places and this scan has to
    /// read whichever it is: its own module under its parent's directory, its
    /// own module at the top, or its parent's module — which is where a class
    /// that is a *value* of another one usually lives, `Core\Http\Part` in
    /// `http.rs` being the case in hand. The first candidate that is there is
    /// the answer, and the first one named is what a failure reports.
    fn module_of(src: &Path, class: &str) -> std::path::PathBuf {
        let mut segments = class.split('\\').skip(1).map(str::to_ascii_lowercase);
        let first = segments.next().expect("a class name has a segment");
        let mut candidates = vec![src.join(format!("{first}.rs"))];
        let mut directory = src.join(&first);
        for segment in segments {
            candidates.insert(0, directory.join(format!("{segment}.rs")));
            candidates.insert(1, src.join(format!("{segment}.rs")));
            directory = directory.join(&segment);
        }
        candidates
            .iter()
            .find(|candidate| candidate.is_file())
            .unwrap_or(&candidates[0])
            .clone()
    }

    for class in &path_taking {
        let file = module_of(&src, class.name);
        let text = std::fs::read_to_string(&file).unwrap_or_else(|error| {
            panic!(
                "`{}` is implemented by `{}`, which this scan has to read: {error}",
                class.name,
                file.display()
            )
        });
        // The same two conventions as `nvs_stdlib_reaches_the_os_only_through_the_gate` above, for
        // the same reasons: a `#[cfg(test)]` module is not in a released binary, and a comment
        // naming what is refused is this rule's own documentation.
        let shipped = text
            .lines()
            .take_while(|line| line.trim_start() != "#[cfg(test)]");
        for (number, line) in shipped.enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            assert!(
                !line.contains("://"),
                "{}:{} spells a URI scheme in a module whose subject is a path; `rule:security/a-path-is-not-a-url` is \
                 that a path naming one is a file with that name and nothing more",
                file.display(),
                number + 1
            );
        }
    }

    // The behavioural half, at the launderer every supplied path goes through. The pair is the
    // whole claim: the same trailing name resolves when it is a name, and does not when a scheme
    // is glued in front of it.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base = manifest.join("src");
    let mut ctx = ctx_reading(&[&canonical(manifest)]);
    let plain = within(&mut ctx, &base, "registry.rs").expect("a name inside the base");
    let dispatched = within(&mut ctx, &base, "php://filter/resource=registry.rs").ok();
    assert_ne!(
        dispatched.as_deref(),
        Some(plain.as_str()),
        "a member that read the scheme off `php://filter/resource=registry.rs` would have found \
         `registry.rs` behind the filter, which is the `php://filter` chain `rule:security/a-path-is-not-a-url` names. \
         Whether the odd file name is refused or resolved as the name it is, what it must never \
         resolve to is the file spelled after the scheme"
    );

    // And at the door, where the grant is compared: a granted root spelled inside a URL is text in
    // a file name, so it carries none of the grant it quotes.
    let caps = reading(&["/granted"]);
    assert!(caps.allows(Cap::FsRead, Scope::Path(Path::new("/granted/own")), &Fake));
    assert!(
        !caps.allows(
            Cap::FsRead,
            Scope::Path(Path::new("php://filter/resource=/granted/own")),
            &Fake
        ),
        "the grant is a prefix of a resolved path and never a substring of the argument; a URL \
         quoting a granted root is a relative name under whatever the process's directory is"
    );
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

/// `Core\IO\File`, as the registry spells it. The constant itself is
/// `pub(crate)`, so a test outside the crate names it the way a program does.
const FILE: &str = r"Core\IO\File";

/// `Core\IO\FileMode`, likewise.
const FILE_MODE: &str = r"Core\IO\FileMode";

#[test]
fn an_open_file_is_an_object_and_never_a_resource() {
    // Spec § 14's R14. What separates an object from a `resource` is not that
    // `open` answers something with a name — a `resource` has a type name too —
    // but that every operation on it is reached *through* it. PHP's handle is
    // an integer passed back to `fread`, `fgets`, `fwrite`, `fseek` and
    // `flock`, each of which re-checks it, and each of which will take any
    // other resource just as happily.
    let io = nvs_stdlib::registry::class(r"Core\IO").expect("`Core\\IO` is registered");
    let open = io
        .methods
        .iter()
        .find(|method| method.name == "open")
        .expect("§ 14's *Handles* bullet is a member");
    assert!(
        matches!(open.return_ty, nvs_stdlib::registry::CoreTy::Instance(name) if name == FILE),
        "`open` answers a named class rather than an integer with a tag on it"
    );

    let file = nvs_stdlib::registry::class(FILE).expect("`Core\\IO\\File` is registered");
    assert!(
        !file.slots.is_empty() && !file.instance.is_empty(),
        "an open file has state and members over it — {} slot(s), {} member(s)",
        file.slots.len(),
        file.instance.len()
    );
    assert!(
        file.methods.is_empty(),
        "every member of this class takes the handle as its receiver, so none of them is static"
    );

    // The invariant the sweep is for, and the one a member added later would
    // break without any single row looking wrong: **an operation on an open
    // file is reached through it.** That is `fread($handle, 8)` refused as a
    // shape, across the whole library rather than at the one place a reviewer
    // thought to look.
    //
    // `rule:core-api/a-lifetime-is-an-object` admits a reader that neither
    // opens nor closes the handle and lives on the class owning a *grammar*,
    // and this is the roster of them: `crates/nvs-stdlib/src/csv.rs`'s own
    // decision section argues the one entry, and a second format that reads
    // incrementally joins it here rather than growing a member on
    // `Core\IO\File`. Nothing on this list may open, close or position a
    // handle — those are members of the object, which is what the rest of this
    // case pins.
    const READERS: &[&str] = &[r"Core\Csv::rows"];

    let mut takers = Vec::new();
    for class in nvs_stdlib::registry::CLASSES {
        for method in class.methods.iter().chain(class.instance) {
            let takes_a_handle = method.params.iter().any(|param| {
                matches!(param, nvs_stdlib::registry::CoreTy::Instance(name) if *name == FILE)
            });
            let named = format!("{}::{}", class.name, method.name);
            if takes_a_handle && !READERS.contains(&named.as_str()) {
                takers.push(named);
            }
        }
    }
    assert!(
        takers.is_empty(),
        "an open file is operated on through its own members, never handed to one: {takers:?}"
    );

    // And a program cannot fabricate one: there is no constructor, so the only
    // thing that produces a handle is the door that checked the capability.
    assert!(
        nvs_stdlib::registry::constructor_symbol(FILE).is_none(),
        "`new Core\\IO\\File()` would be a handle nothing had granted"
    );
}

#[test]
fn a_file_mode_is_an_enum_and_never_a_string() {
    // Spec § 14's R11. The registry half first: `open`'s second parameter is
    // the enum, so `Core\IO::open($p, "r+b")` is refused by `E0401` before the
    // program runs rather than by a parser inside the member.
    let io = nvs_stdlib::registry::class(r"Core\IO").expect("`Core\\IO` is registered");
    let open = io
        .methods
        .iter()
        .find(|method| method.name == "open")
        .expect("§ 14's *Handles* bullet is a member");
    assert!(
        matches!(open.params[1], nvs_stdlib::registry::CoreTy::Enum(name) if name == FILE_MODE),
        "the mode is a case of a closed enum and not text of any classification"
    );

    let mode = nvs_stdlib::registry::ENUMS
        .iter()
        .find(|declared| declared.name == FILE_MODE)
        .expect("`Core\\IO\\FileMode` is registered");
    let names: Vec<&str> = mode.cases.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        names,
        ["Read", "Write", "Append", "ReadWrite"],
        "four cases replace `fopen`'s twelve spellings, and `b`/`t` is not an axis at all"
    );

    // The half a registry assertion cannot reach: each case has to *do*
    // something different to the file, or the enum would be four names for one
    // behaviour. Asserted at the door rather than through the member, because
    // what the case selects is a `capability::Access` and the member adds
    // nothing to it.
    let dir = std::env::temp_dir().join("nvs-file-mode-cases");
    std::fs::create_dir_all(&dir).expect("a temporary directory the test owns");
    let path = dir.join("note.txt");
    std::fs::write(&path, b"first\n").expect("the starting content");
    let ctx = ctx_reading_and_writing(&[&canonical(&dir)]);
    let opened = |access| {
        nvs_runtime::capability::open(&ctx, &path, access, "Core\\IO::open")
            .expect("both capabilities are granted under this root")
    };

    // `Append` keeps what is there and writes past it.
    {
        use std::io::Write;
        let mut handle = opened(nvs_runtime::capability::Access::Append);
        handle.write_all(b"second\n").expect("the append");
    }
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        b"first\nsecond\n",
        "`Append` adds and never replaces"
    );

    // `ReadWrite` creates nothing here and empties nothing either — it is
    // `fopen`'s `c+` and not its `w+`, which is the whole reason the other `+`
    // forms are not cases.
    drop(opened(nvs_runtime::capability::Access::ReadWrite));
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        b"first\nsecond\n",
        "`ReadWrite` truncates nothing"
    );

    // `Write` empties it, and does so on the open rather than on the first
    // write — a caller that opened and then decided not to write has already
    // changed the file.
    drop(opened(nvs_runtime::capability::Access::Write));
    assert!(
        std::fs::read(&path).expect("the file").is_empty(),
        "`Write` truncates when the handle is made"
    );

    // `Read` on a name that is not there is the one case that refuses rather
    // than creating, which is what makes it different from the three above.
    let missing = dir.join("never-written.txt");
    let _ = std::fs::remove_file(&missing);
    nvs_runtime::capability::open(
        &ctx,
        &missing,
        nvs_runtime::capability::Access::Read,
        "Core\\IO::open",
    )
    .expect_err("`Read` opens what is there and creates nothing");

    // And `ReadWrite` on the same name does create it, which is the pair of
    // that refusal asserted from the other side.
    drop(
        nvs_runtime::capability::open(
            &ctx,
            &missing,
            nvs_runtime::capability::Access::ReadWrite,
            "Core\\IO::open",
        )
        .expect("`ReadWrite` creates what is not there"),
    );
    assert!(missing.exists(), "`ReadWrite` created it");
    let _ = std::fs::remove_file(&missing);
}

/// A context granting `fs.read` **and** `fs.write` under `roots`, which is what
/// a mode-by-mode test needs: [`ctx_reading`] would make every writing case
/// refuse for the wrong reason.
fn ctx_reading_and_writing(roots: &[&str]) -> nvs_runtime::Ctx {
    let listed: Vec<String> = roots.iter().map(|&root| root.to_owned()).collect();
    let mut caps = Capabilities {
        fs: Some(CapFs {
            read: Some(Setting::List(listed.clone())),
            write: Some(Setting::List(listed)),
        }),
        ..Capabilities::default()
    };
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
