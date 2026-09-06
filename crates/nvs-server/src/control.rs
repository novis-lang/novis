//! [ADR 0078] §§ 3, 5 and 6's operator surface: the one local endpoint, the one operation a client
//! connected to it may ask for, and what a reload reports back.
//!
//! **The endpoint itself is [`nvs_config::control`] and is re-exported here**, because creating an
//! object no other account can reach is a mode on Unix and a DACL on Windows and that crate already
//! owns both spellings for [ADR 0103] § 6's trust boundary. What is *here* is the half that is about
//! being a server: which requests arriving on that endpoint mean anything.
//!
//! **`reload` is the only operation, and that is a scope decision rather than the whole of § 3.**
//! That section also reserves `ctl config` — [ADR 0103] § 9's read of the live snapshot with each
//! directive's origin — and this surface does not answer it yet. Nothing here is shaped to prevent
//! it: [`Operation`] is an enum with one variant and gains a second when that lands, and the
//! refusal below names the operation it did not know rather than claiming the roster is closed.
//!
//! **No control operation runs Novis code, ever**, which is § 3's own sentence and the reason the
//! roster is an enum rather than a route table: a control surface that could dispatch is
//! `rule:security/no-eval`'s `eval` door with a different name on it. There is no path from here into the
//! compiler, and there is deliberately nothing to add one to.
//!
//! [ADR 0078]: /docs/adr/0078-config-reload-and-control-socket.md
//! [ADR 0103]: /docs/adr/0103-configuration-is-a-tree-of-files.md

pub use nvs_config::control::{Address, Endpoint, Refusal, Report, bind, boundary, reload};

/// What a request arriving on the control endpoint asked for — [ADR 0078] § 3.
///
/// [ADR 0078]: /docs/adr/0078-config-reload-and-control-socket.md
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// `POST /reload`: re-read the whole configuration tree and publish it, reporting [`Report`].
    Reload,
}

/// Why a request arriving on the control endpoint asked for nothing this surface has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Denied {
    /// The target names no operation. It carries the target so the answer can name it back, which
    /// is what tells an operator holding a `nvs ctl` from a newer build that they have one.
    NoSuchOperation(String),
    /// The target names an operation, and the method is not the one it is performed with. A reload
    /// changes what the process is serving, so `GET` is refused rather than treated as a synonym —
    /// exactly the reason [ADR 0074] § 1 gives for safe methods being safe.
    ///
    /// [ADR 0074]: /docs/adr/0074-http-defaults-safe-and-finite.md
    WrongMethod {
        /// What the operation is performed with, for the `Allow` header the answer carries.
        allow: &'static str,
    },
}

impl Denied {
    /// The status this refusal is sent as.
    ///
    /// This surface sends its own statuses, and that is not [ADR 0102] § 1's rule being bent: that
    /// rule is about a *request the program serves*, where the server matches and the program
    /// decides. Nothing an operator sends here reaches a program at all — § 3 says no control
    /// operation runs Novis code — so there is nobody else who could answer.
    ///
    /// [ADR 0102]: /docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            Self::NoSuchOperation(_) => 404,
            Self::WrongMethod { .. } => 405,
        }
    }
}

impl Operation {
    /// The operation `method` and `target` name, or why they name none.
    ///
    /// The target is compared whole and never normalized, which is the opposite of what a router
    /// does and is deliberate: this roster has one entry, so there is no traversal to resolve and
    /// nothing a `..` or an escape could resolve *to*. A target that is not exactly `/reload` is
    /// refused rather than folded into it, so no spelling of the one operation exists that a log
    /// line would not show verbatim.
    ///
    /// # Errors
    ///
    /// [`Denied`], whose two halves are an unknown target and the wrong method for a known one.
    pub fn of(method: &str, target: &str) -> Result<Self, Denied> {
        // A query string is not part of the operation's name, and a control operation takes no
        // parameters at all — so one that arrives is part of the target that must not match.
        match target {
            "/reload" if method == "POST" => Ok(Self::Reload),
            "/reload" => Err(Denied::WrongMethod { allow: "POST" }),
            other => Err(Denied::NoSuchOperation(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use nvs_config::snapshot::{Current, Snapshot};
    use nvs_config::tree::{Config, Control, Setting};
    use nvs_config::trust::Untrusted;

    use super::{Address, Denied, Operation, Refusal, bind, boundary, reload};

    /// A directory of this case's own, empty, beside the test binary under `target/`.
    ///
    /// Not `std::env::temp_dir()`, which is what this was: on Unix that is `/tmp`, mode 1777, and
    /// the ADR 0103 § 6 check § 3's socket directory is held to refuses a directory the world can
    /// write. It runs on the named directory *and* on its parent, so a scratch directory of our own
    /// is refused for the `/tmp` above it however tight its own bits are. That refusal is § 3
    /// working, so the case names a directory the rule accepts rather than asking for a rule that
    /// accepts `/tmp`. `target/` is owned by this account and writable by neither its group nor the
    /// world, which is the same bar the operator's `/run/nvs` has to clear.
    fn scratch(name: &str) -> PathBuf {
        let beside = std::env::current_exe().expect("the test binary knows its own path");
        let dir = beside
            .parent()
            .expect("a test binary sits in a directory")
            .join(format!("nvs-control-{}-{name}", std::process::id()));
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(&dir).expect("a scratch directory of this case's own");
        dir
    }

    /// The endpoint one case creates, in the platform's own namespace: a socket under `dir` on
    /// Unix, a pipe name on Windows, where `dir` exists only to be the thing § 3's directory rule
    /// is about.
    fn endpoint(dir: &std::path::Path, name: &str) -> PathBuf {
        #[cfg(unix)]
        {
            dir.join(format!("{name}.sock"))
        }
        #[cfg(windows)]
        {
            let _ = dir;
            PathBuf::from(format!(
                r"\\.\pipe\nvs-control-{}-{name}",
                std::process::id()
            ))
        }
    }

    /// Whether anything at all answers at `name`, and this account is the one it answers to — the
    /// question "was the endpoint created", asked in each platform's own terms, because a pipe is
    /// not a directory entry.
    ///
    /// `ERROR_PIPE_BUSY` counts as yes, and distinguishing it is the whole reason this is a match
    /// rather than an `is_ok`. One `CreateNamedPipeW` creates **one** instance, and reading a
    /// pipe's security descriptor opens it, so a case that has just asked what the DACL says finds
    /// the only instance already taken. The three answers that matter are all different numbers:
    /// `2` is a name that is not there, `5` is one this account is refused, and `231` is one that
    /// exists and let this account through.
    fn exists(name: &std::path::Path) -> bool {
        #[cfg(unix)]
        {
            name.exists()
        }
        #[cfg(windows)]
        {
            const ERROR_PIPE_BUSY: i32 = 231;

            match fs::OpenOptions::new().read(true).write(true).open(name) {
                Ok(_) => true,
                Err(err) => err.raw_os_error() == Some(ERROR_PIPE_BUSY),
            }
        }
    }

    /// A `[control]` block holding one written value.
    fn written(socket: Setting) -> Config {
        Config {
            control: Some(Control {
                socket: Some(socket),
            }),
            ..Config::default()
        }
    }

    /// A snapshot of one TOML document, typed and untyped halves agreeing — which is what
    /// `Snapshot::build` produces and what `Current::publish` compares key by key.
    fn snapshot(document: &str) -> Snapshot {
        Snapshot {
            config: toml::from_str(document).expect("the case's own TOML deserializes"),
            table: document.parse().expect("the case's own TOML parses"),
            ..Snapshot::default()
        }
    }

    /// ADR 0078 § 3: the socket is created mode `0600`, owned by the runtime's account — which is
    /// the whole authentication story, since there is no token and no auth middleware to fall back
    /// on.
    ///
    /// The claim is one and the spelling is two, exactly as ADR 0103 § 6's boundary is: on Unix a
    /// mode is readable off the socket file, and on Windows the equivalent question is whether any
    /// of § 6's untrusted principals reaches the object at all, which
    /// `nvs_config::trust::exposure` answers from its DACL. The pipe is opened afterwards on that
    /// platform because a descriptor that locked *this* account out would satisfy the first half
    /// and be useless.
    #[test]
    fn the_control_socket_is_created_0600() {
        let dir = scratch("created");
        let name = endpoint(&dir, "created");

        let created = bind(&name, boundary).expect("this account owns the directory it names");
        assert_eq!(
            created.name(),
            name,
            "it reports the name it was created under"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(&name)
                .expect("the socket is on disk")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(
                mode, 0o600,
                "nobody but the runtime's own account may connect: mode {mode:04o}",
            );
        }
        #[cfg(windows)]
        {
            assert_eq!(
                nvs_config::trust::exposure(&name),
                None,
                "no account outside § 6's boundary reaches the pipe",
            );
            assert!(
                exists(&name),
                "and the descriptor did not lock this account out of its own endpoint",
            );
        }

        drop(created);
        drop(fs::remove_dir_all(&dir));
    }

    /// ADR 0078 § 3's other half of the same sentence: the server refuses to start if the socket's
    /// directory is world-writable, because an account that can write that directory can put a
    /// socket of its own there and speak for the server.
    ///
    /// The verdict is injected rather than constructed, for a reason the platforms force: Windows
    /// keeps named pipes in a kernel namespace with no directory to put in that state at all. What
    /// is asserted portably is the wiring — the refusal comes before anything is created — and the
    /// Unix half then runs the real `boundary` against a real directory in the real state, which is
    /// what stops the injection from being the only thing under test.
    #[test]
    fn a_world_writable_socket_directory_refuses_the_socket() {
        let dir = scratch("exposed");
        let name = endpoint(&dir, "exposed");

        let why = bind(&name, |_| {
            Err(Untrusted::Breach(
                "is world-writable (mode 0777, gid 1000)".to_string(),
            ))
        })
        .expect_err("a directory anyone can write is outside the boundary");
        assert!(
            matches!(why, Refusal::Untrusted(_)) && why.message().contains("world-writable"),
            "the refusal is the boundary's and says what is wrong: {why:?}",
        );
        assert!(
            !exists(&name),
            "and it was refused before anything was created — an endpoint left behind by a \
             refused boot is one an operator would find and trust",
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&dir, fs::Permissions::from_mode(0o777))
                .expect("a world-writable mode");
            let why = bind(&name, boundary).expect_err("the real check reads the real directory");
            assert!(
                why.message().contains("world-writable"),
                "the real boundary refuses the same state the injected verdict describes: {}",
                why.message(),
            );
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).expect("an owned mode");
        }

        drop(fs::remove_dir_all(&dir));
    }

    /// ADR 0078 § 3: `reload` is the operation, and the goal's own standing decision is that it is
    /// the only one this surface has. Nothing is dispatched, nothing is compiled, and no Novis code
    /// runs — so a target this roster does not hold is refused rather than resolved.
    ///
    /// `ctl config` is § 3's other reserved operation and is not here yet; the module doc says so,
    /// and the refusal names the unknown target rather than declaring the roster closed.
    #[test]
    fn reload_is_the_sockets_only_operation() {
        assert_eq!(Operation::of("POST", "/reload"), Ok(Operation::Reload));

        for target in [
            "/config",     // § 3 reserves it; this surface does not answer it yet
            "/shutdown",   // the operation an operator would most expect a control socket to have
            "/eval",       // `rule:security/no-eval`'s door, under the name it would arrive as
            "/",           // the root, which names nothing
            "/reload/",    // not a spelling of the operation: the target is compared whole
            "/reload?now", // a control operation takes no parameters
            "/RELOAD",
            "//reload",
            "/reload/../config",
        ] {
            assert_eq!(
                Operation::of("POST", target),
                Err(Denied::NoSuchOperation(target.to_string())),
                "`POST {target}` names no operation this surface has",
            );
        }

        for method in ["GET", "HEAD", "PUT", "DELETE", "PATCH", "OPTIONS"] {
            let refused = Operation::of(method, "/reload")
                .expect_err("a reload changes what the process serves");
            assert_eq!(refused, Denied::WrongMethod { allow: "POST" });
            assert_eq!(
                refused.status(),
                405,
                "and the answer says which method does"
            );
        }
        assert_eq!(
            Operation::of("POST", "/shutdown").map_err(|why| why.status()),
            Err(404),
        );
    }

    /// ADR 0078 § 6: there is no network-reachable control surface, in either direction of
    /// configuration.
    ///
    /// "Either direction" is what makes this a *boot* refusal rather than an absence: nothing in
    /// this tree binds a TCP socket for control, and a directive that asks for one is refused where
    /// it is written instead of being read as the relative path a host and a port happen to spell.
    /// A file named `127.0.0.1:9000` is a legal name on Unix, and creating one is the reading that
    /// leaves an operator believing they bound a port.
    #[test]
    fn there_is_no_network_reachable_control_surface() {
        for written_value in [
            "127.0.0.1:9000",
            "0.0.0.0:9000",
            "[::1]:9000",
            ":9000",
            "localhost:9000",
            "example.com:9000",
            "tcp://127.0.0.1:9000",
            "http://localhost/reload",
            "unix://run/nvs/control.sock",
        ] {
            let refused = Address::of(&written(Setting::Text(written_value.to_string())))
                .expect_err("`{written_value}` would be reached over a network");
            assert!(
                refused.message.contains("names no local endpoint"),
                "`socket = \"{written_value}\"` is refused at boot: {}",
                refused.message,
            );
        }
        assert!(
            Address::of(&written(Setting::Integer(9000))).is_err(),
            "a bare number is a port, and there is no listener for one to be bound on",
        );
        assert!(
            Address::of(&written(Setting::Bool(true))).is_err(),
            "`true` names no endpoint; only `false` means anything here",
        );

        // And the local spellings the ADR itself writes are accepted, so the refusal above is the
        // network shape and not a directive nothing can satisfy.
        let accepted = |value: &str| {
            Address::of(&written(Setting::Text(value.to_string())))
                .expect("§ 3's own example is a local endpoint")
        };
        assert_eq!(
            accepted("/run/nvs/control.sock"),
            Address::Local(PathBuf::from("/run/nvs/control.sock")),
        );
        assert_eq!(
            accepted(r"\\.\pipe\nvs-control"),
            Address::Local(PathBuf::from(r"\\.\pipe\nvs-control")),
        );
        assert_eq!(
            Address::of(&written(Setting::Bool(false))).expect("`false` disables the surface"),
            Address::Disabled,
        );
        assert_eq!(
            Address::of(&Config::default()).expect("no `[control]` block is not a refusal"),
            Address::Disabled,
            "a tree that asked for no control surface does not get one",
        );
    }

    /// ADR 0078 § 5: a reload names the `Boot` keys whose values changed and therefore did not take
    /// effect, individually, beside what it did apply.
    ///
    /// Silently ignoring a changed `Boot` key is how a deployment ends up believing it applied a
    /// change it did not, which is § 5's own sentence. The key used here is `control.socket`
    /// itself — a `Boot` row by ADR 0078 § 2 — so the case also pins that moving the control
    /// endpoint needs a restart rather than taking effect underneath the connection asking for it.
    #[test]
    fn a_changed_boot_key_is_reported_and_does_not_take_effect() {
        let current = Current::new(Arc::new(snapshot(
            "[control]\nsocket = \"/run/nvs/control.sock\"\n[limits]\nmemory = \"128M\"\n",
        )));

        let report = reload(
            &current,
            snapshot("[control]\nsocket = \"/run/nvs/other.sock\"\n[limits]\nmemory = \"256M\"\n"),
            12,
        )
        .expect("the incoming tree is well formed");

        assert_eq!(
            report.ignored,
            vec!["control.socket"],
            "the changed `Boot` key is named individually, not counted",
        );
        assert_eq!(
            report.applied,
            vec!["limits.memory".to_string()],
            "and the `Reload` key beside it did take effect",
        );
        assert_eq!(
            current
                .load()
                .config
                .control
                .as_ref()
                .and_then(|it| it.socket.clone()),
            Some(Setting::Text("/run/nvs/control.sock".to_string())),
            "the running value is still the one the process booted on",
        );
        assert_eq!(
            report.invalidated, 0,
            "and no compiled unit was invalidated: § 4 rekeys every unit or none, on `env_hash`, \
             which a `[limits]` change is no part of",
        );
    }
}
