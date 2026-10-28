//! A package's `[[download]]` files through `nvs pkg fetch`, `nvs pkg pack`, `nvs pkg download`
//! and `nvs pkg provide`
//! (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`,
//! `rule:packaging/nothing-runs-before-the-program`).
//!
//! Each test builds a `file:` registry from a package that `nvs pkg pack` packs, an application
//! beside it, and an HTTPS host on loopback that serves the files. The application's `nvs.toml`
//! trusts the host's certificate and writes `internal = ["127.0.0.1"]`, the one key that lets a
//! download reach loopback. The form check and the cache are unit tests in
//! `src/package/download.rs`; these tests show what the binary places and what it refuses, and
//! that the package's code reads a placed file with an empty grant line.

use std::fs;
use std::io::{BufRead as _, BufReader, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use sha2::{Digest as _, Sha256};

/// The longest the host waits for a request it has accepted. A bound on a stuck client, never a
/// wait the tests rely on.
const BOUND: Duration = Duration::from_secs(30);

/// The path every package in these tests downloads its file to.
const PATH: &str = "fonts/Big.otf";

/// One reply of the [`Host`]: a status, a `Location` for a redirect, and a body.
struct Reply {
    status: u16,
    location: Option<String>,
    body: Vec<u8>,
}

impl Reply {
    /// `200` with `body`.
    fn ok(body: &[u8]) -> Self {
        Self {
            status: 200,
            location: None,
            body: body.to_vec(),
        }
    }

    /// `302` to `to`.
    fn redirect(to: &str) -> Self {
        Self {
            status: 302,
            location: Some(to.to_owned()),
            body: Vec::new(),
        }
    }

    /// `404`, the reply to every path the table does not name.
    fn gone() -> Self {
        Self {
            status: 404,
            location: None,
            body: Vec::new(),
        }
    }
}

/// An HTTPS host on loopback with a certificate for `127.0.0.1`, serving a fixed table of paths
/// one connection after another on a thread of its own. Dropping it stops the thread.
struct Host {
    port: u16,
    /// The certificate's PEM file, which the application's `roots` names.
    roots: PathBuf,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Host {
    /// Issues the certificate into `dir`, binds a free port, and serves `routes`, each a path and
    /// its reply.
    fn start(dir: &Path, routes: Vec<(&str, Reply)>) -> Self {
        let issued = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()])
            .expect("the certificate is issued");
        let roots = dir.join("roots.pem");
        fs::write(&roots, issued.cert.pem()).expect("the test writes the certificate");
        let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(issued.signing_key.serialize_der()),
        );
        let config = Arc::new(
            rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("the provider speaks the default versions")
            .with_no_client_auth()
            .with_single_cert(vec![issued.cert.der().clone()], key)
            .expect("the certificate and its key match"),
        );
        let routes: Vec<(String, Reply)> = routes
            .into_iter()
            .map(|(path, reply)| (path.to_owned(), reply))
            .collect();
        let listener = TcpListener::bind("127.0.0.1:0").expect("the test listens");
        let port = listener.local_addr().expect("it has an address").port();
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopped);
        let thread = thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                // A request that fails here fails the download, which the case asserts.
                let _ = serve(stream, Arc::clone(&config), &routes);
            }
        });
        Self {
            port,
            roots,
            stopped,
            thread: Some(thread),
        }
    }

    /// The `https:` URL of `path` on this host.
    fn url(&self, path: &str) -> String {
        format!("https://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for Host {
    /// Ends the thread: the flag is set, and one connection wakes its `accept`.
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// One request over TLS on `stream`, answered with the reply `routes` names for its path, or
/// `404`.
fn serve(
    stream: TcpStream,
    config: Arc<rustls::ServerConfig>,
    routes: &[(String, Reply)],
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(BOUND))?;
    let conn = rustls::ServerConnection::new(config).map_err(std::io::Error::other)?;
    let mut tls = BufReader::new(rustls::StreamOwned::new(conn, stream));
    let mut request = String::new();
    tls.read_line(&mut request)?;
    loop {
        let mut line = String::new();
        if tls.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
    }
    let path = request.split_whitespace().nth(1).unwrap_or_default();
    let gone = Reply::gone();
    let reply = routes
        .iter()
        .find(|(route, _)| route == path)
        .map_or(&gone, |(_, reply)| reply);
    let mut head = format!(
        "HTTP/1.1 {} Test\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len()
    );
    if let Some(location) = &reply.location {
        use std::fmt::Write as _;
        // Writing into a `String` does not fail.
        let _ = write!(head, "Location: {location}\r\n");
    }
    head.push_str("\r\n");
    let tls = tls.get_mut();
    tls.write_all(head.as_bytes())?;
    tls.write_all(&reply.body)?;
    tls.conn.send_close_notify();
    tls.flush()
}

/// The lowercase hex SHA-256 hash of `body`.
fn sha256(body: &[u8]) -> String {
    Sha256::digest(body)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A `[[download]]` table for the bytes `body` at `path`, fetched from `urls` in order.
fn entry(path: &str, urls: &[String], body: &[u8]) -> String {
    format!(
        "\n[[download]]\npath = \"{path}\"\nurls = {urls:?}\nsha256 = \"{}\"\nsize = {}\n",
        sha256(body),
        body.len()
    )
}

/// The package's `src/Main.nvs` when a case does not run it.
const EMPTY_MAIN: &str = "<?nvs\nnamespace Acme\\Http;\n\nfinal class Main\n{\n}\n";

/// A registry, an application that needs `Acme\Http` from it, and a private data folder.
struct Tree {
    root: nvs_repo::Scratch,
    data: nvs_repo::Scratch,
}

impl Tree {
    /// An empty registry, and an application whose `package.toml` needs `Acme\Http` 1.0.0.
    fn new(name: &str) -> Self {
        let tree = Self {
            root: nvs_repo::scratch(name),
            data: nvs_repo::scratch_private(&format!("{name}-data")),
        };
        fs::create_dir_all(tree.registry()).expect("the test makes the registry");
        fs::write(
            tree.registry().join("registry.json"),
            r#"{ "schema": 1, "hosts": [] }"#,
        )
        .expect("the test writes the root file");
        fs::create_dir_all(tree.app()).expect("the test makes the application");
        fs::write(
            tree.app().join("package.toml"),
            "[dependencies]\n'Acme\\Http' = \"1.0.0\"\n\n\
             [registries]\nAcme = { url = \"file:../registry\" }\n",
        )
        .expect("the test writes the manifest");
        tree
    }

    fn registry(&self) -> PathBuf {
        self.root.join("registry")
    }

    fn app(&self) -> PathBuf {
        self.root.join("app")
    }

    /// The installed package's folder under the application's `vendor/`.
    fn package(&self) -> PathBuf {
        self.app().join("vendor/acme/http")
    }

    /// The application's `nvs.toml`: it trusts `host`'s certificate and lets a download reach
    /// loopback.
    fn trust(&self, host: &Host) {
        fs::write(
            self.app().join("nvs.toml"),
            format!(
                "[http.client.tls]\nroots = ['{}']\n\n\
                 [capabilities.net]\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n",
                host.roots.display()
            ),
        )
        .expect("the test writes the configuration");
    }

    /// The folder of `Acme\Http` 1.0.0, whose `package.toml` ends with `downloads`, and
    /// `nvs pkg pack`'s run on it.
    fn pack(&self, downloads: &str) -> (Output, PathBuf) {
        self.pack_with(downloads, EMPTY_MAIN)
    }

    /// [`Self::pack`], with `main` as the text of `src/Main.nvs`.
    fn pack_with(&self, downloads: &str, main: &str) -> (Output, PathBuf) {
        let dir = self.root.join("packages/acme-http-1.0.0");
        fs::create_dir_all(dir.join("src")).expect("the test makes the package");
        fs::write(
            dir.join("package.toml"),
            format!(
                "[package]\nname = 'Acme\\Http'\nversion = \"1.0.0\"\n\n\
                 [capabilities]\nrequired = []\n{downloads}"
            ),
        )
        .expect("the test writes the package's manifest");
        fs::write(dir.join("LICENSE"), "MIT\n").expect("the test writes the license");
        fs::write(dir.join("src/Main.nvs"), main).expect("the test writes the source");
        let tar = self.root.join("packages/acme-http-1.0.0.tar");
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["pkg", "pack"])
            .arg(&dir)
            .arg("--out")
            .arg(&tar)
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        (out, tar)
    }

    /// `Acme\Http` 1.0.0 with `downloads`, packed by the binary and listed in the registry as
    /// released.
    fn publish(&self, downloads: &str) {
        self.publish_with(downloads, EMPTY_MAIN);
    }

    /// [`Self::publish`], with `main` as the text of `src/Main.nvs`.
    fn publish_with(&self, downloads: &str, main: &str) {
        let (out, tar) = self.pack_with(downloads, main);
        assert!(out.status.success(), "{}", shown(&out));
        let bytes = fs::read(&tar).expect("the archive was written");
        let digest = blake3::hash(&bytes).to_hex().to_string();
        let archive = self.registry().join("archive");
        fs::create_dir_all(&archive).expect("the test makes the archive folder");
        fs::write(archive.join(format!("{digest}.tar")), &bytes).expect("the test copies it");
        let index = serde_json::json!({
            "schema": 1,
            "name": "Acme\\Http",
            "owners": ["owner"],
            "versions": [{
                "version": "1.0.0",
                "digest": digest,
                "state": "released",
                "dependencies": {},
                "capabilities": { "required": [], "optional": [] },
                "repository": "https://git.example.com/acme/http",
                "commit": "0".repeat(40),
            }],
        });
        let path = self.registry().join("index/acme/http.json");
        fs::create_dir_all(path.parent().expect("an index file has a folder"))
            .expect("the test makes the index folder");
        fs::write(path, index.to_string()).expect("the test writes the index");
    }

    /// `nvs <args>` in the application's folder, with the tree's own data folder.
    fn nvs(&self, args: &[&str]) -> Output {
        self.nvs_in(&self.app(), args)
    }

    /// `nvs <args>` in `dir`, with the tree's own data folder.
    fn nvs_in(&self, dir: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*self.data)
            .arg("--no-init")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("the `nvs` binary this test was built beside runs")
    }
}

/// Both streams, for a message on failure.
fn shown(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Asserts that `out` failed and that its standard error contains each of `said`.
fn refused(out: &Output, said: &[&str]) {
    assert!(!out.status.success(), "it succeeded: {}", shown(out));
    let stderr = String::from_utf8_lossy(&out.stderr);
    for part in said {
        assert!(stderr.contains(part), "`{part}` is not in: {}", shown(out));
    }
}

/// Asserts that nothing was placed at the download's path, not even a partial file.
fn nothing_placed(tree: &Tree) {
    let target = tree.package().join(PATH);
    assert!(!target.exists(), "`{}` was placed", target.display());
    let partial = tree.package().join(format!("{PATH}.partial"));
    assert!(!partial.exists(), "`{}` stayed", partial.display());
}

/// Bytes of the right size and the wrong hash fail the fetch, naming the package, the path and
/// what the URL returned, and nothing is placed or kept in the cache.
#[test]
fn a_file_with_the_wrong_hash_is_not_placed() {
    let tree = Tree::new("package-download-hash");
    let host = Host::start(&tree.root, vec![("/wrong", Reply::ok(b"wrong bytes"))]);
    tree.trust(&host);
    let url = host.url("/wrong");
    tree.publish(&entry(PATH, std::slice::from_ref(&url), b"right bytes"));

    let out = tree.nvs(&["pkg", "fetch"]);
    let named = format!("`{url}`: the file's SHA-256 hash is");
    refused(
        &out,
        &[
            "`Acme\\Http` needs the file `fonts/Big.otf`, and no URL gave it.",
            named.as_str(),
        ],
    );
    nothing_placed(&tree);
    let hash = sha256(b"right bytes");
    assert!(
        !tree
            .data
            .join("downloads")
            .join(&hash[..2])
            .join(&hash)
            .exists(),
        "the wrong bytes were kept in the cache"
    );
}

/// A body longer than the declared size is stopped one byte past it, and nothing is placed.
#[test]
fn a_file_past_its_declared_size_is_not_placed() {
    let tree = Tree::new("package-download-size");
    let host = Host::start(
        &tree.root,
        vec![("/long", Reply::ok(b"right bytes and then some more"))],
    );
    tree.trust(&host);
    tree.publish(&entry(PATH, &[host.url("/long")], b"right bytes"));

    let out = tree.nvs(&["pkg", "fetch"]);
    refused(
        &out,
        &[
            "no URL gave it",
            "the file is larger than its declared size of 11 bytes",
        ],
    );
    nothing_placed(&tree);
}

/// A redirect to the cloud metadata address, which `internal` does not name, is refused before
/// anything connects to it, and the fetch fails.
#[test]
fn a_redirect_to_an_internal_address_is_refused() {
    let tree = Tree::new("package-download-redirect");
    let host = Host::start(
        &tree.root,
        vec![(
            "/moved",
            Reply::redirect("https://169.254.169.254/latest/meta-data"),
        )],
    );
    tree.trust(&host);
    tree.publish(&entry(PATH, &[host.url("/moved")], b"right bytes"));

    let out = tree.nvs(&["pkg", "fetch"]);
    refused(
        &out,
        &["no URL gave it", "169.254.169.254", "[capabilities.net]"],
    );
    nothing_placed(&tree);
}

/// An `http:` URL is refused by `nvs pkg pack`, so no archive carries it, and `nvs pkg download`
/// refuses one before anything connects.
#[test]
fn an_http_url_is_refused() {
    let tree = Tree::new("package-download-http");
    let url = "http://example.com/Big.otf".to_owned();
    let (out, tar) = tree.pack(&entry(PATH, std::slice::from_ref(&url), b"right bytes"));
    refused(
        &out,
        &["`http://example.com/Big.otf`. Every URL must start with `https://`."],
    );
    assert!(!tar.exists(), "an archive was written");

    let dir = tree.root.join("packages/acme-http-1.0.0");
    let manifest = |dir: &Path| fs::read_to_string(dir.join("package.toml")).unwrap_or_default();
    fs::write(
        dir.join("package.toml"),
        "[package]\nname = 'Acme\\Http'\nversion = \"1.0.0\"\n",
    )
    .expect("the test writes the manifest");
    let before = manifest(&dir);
    let out = tree.nvs_in(&dir, &["pkg", "download", &url, "--to", PATH]);
    refused(
        &out,
        &["cannot download `http://example.com/Big.otf`: the URL does not start with `https://`"],
    );
    assert_eq!(manifest(&dir), before, "the entry was written");
    assert!(!dir.join(PATH).exists());
}

/// A first URL that answers `404` is passed over, and the second URL's file is placed with its
/// bytes.
#[test]
fn a_dead_url_is_followed_by_the_next() {
    let tree = Tree::new("package-download-mirror");
    let host = Host::start(&tree.root, vec![("/mirror", Reply::ok(b"right bytes"))]);
    tree.trust(&host);
    tree.publish(&entry(
        PATH,
        &[host.url("/gone"), host.url("/mirror")],
        b"right bytes",
    ));

    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(
        fs::read(tree.package().join(PATH)).expect("the file was placed"),
        b"right bytes"
    );
}

/// When the only URL is dead, the fetch fails but writes the lock. `nvs pkg provide` refuses a
/// file with other bytes and keeps the right one, and the next fetch places it with no network.
#[test]
fn a_provided_file_is_placed_without_its_url() {
    let tree = Tree::new("package-download-provide");
    let host = Host::start(&tree.root, Vec::new());
    tree.trust(&host);
    tree.publish(&entry(PATH, &[host.url("/gone")], b"right bytes"));

    let out = tree.nvs(&["pkg", "fetch"]);
    refused(&out, &["no URL gave it", "the status was 404"]);
    nothing_placed(&tree);
    let lock = fs::read_to_string(tree.app().join("package.lock")).unwrap_or_default();
    assert!(lock.contains("name = 'Acme\\Http'"), "no lock: {lock}");

    let wrong = tree.root.join("wrong.otf");
    fs::write(&wrong, b"wrong bytes").expect("the test writes the file");
    let out = tree.nvs(&["pkg", "provide", &wrong.to_string_lossy()]);
    refused(
        &out,
        &["no package in `package.lock` lists a file with the size 11"],
    );

    let right = tree.root.join("right.otf");
    fs::write(&right, b"right bytes").expect("the test writes the file");
    let out = tree.nvs(&["pkg", "provide", &right.to_string_lossy()]);
    assert!(out.status.success(), "{}", shown(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("is now in the download cache"),
        "{}",
        shown(&out)
    );

    // The host is stopped, so a fetch that reached the network would fail.
    drop(host);
    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(
        fs::read(tree.package().join(PATH)).expect("the file was placed"),
        b"right bytes"
    );
}

/// A zip file is placed byte for byte at its path, and nothing in it is unpacked.
#[test]
fn an_archive_is_placed_without_being_unpacked() {
    let tree = Tree::new("package-download-zip");
    let zip: &[u8] = b"PK\x03\x04\x14\x00\x00\x00\x00\x00inner.txt\x00not unpacked\x00PK\x05\x06";
    let host = Host::start(&tree.root, vec![("/fonts.zip", Reply::ok(zip))]);
    tree.trust(&host);
    let path = "archives/fonts.zip";
    tree.publish(&entry(path, &[host.url("/fonts.zip")], zip));

    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(
        fs::read(tree.package().join(path)).expect("the zip was placed"),
        zip
    );
    let folder: Vec<_> = fs::read_dir(tree.package().join("archives"))
        .expect("the folder was made")
        .flatten()
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(
        folder,
        ["fonts.zip"],
        "something beside the zip was written"
    );
    assert!(!tree.package().join("inner.txt").exists());
}

/// The package's code reads the file it downloads with the empty grant line `nvs pkg fetch`
/// adds, and the program prints it.
#[test]
fn a_package_reads_its_downloaded_file_with_no_grant() {
    let tree = Tree::new("package-download-own-file");
    let host = Host::start(&tree.root, vec![("/big", Reply::ok(b"downloaded text\n"))]);
    tree.trust(&host);
    let main = format!(
        "<?nvs\nnamespace Acme\\Http;\n\nfinal class Main\n{{\n    \
         public function constructor()\n    {{\n        \
         echo Core\\IO::readText('../{PATH}');\n    }}\n}}\n"
    );
    tree.publish_with(
        &entry(PATH, &[host.url("/big")], b"downloaded text\n"),
        &main,
    );

    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    let manifest = fs::read_to_string(tree.app().join("package.toml")).unwrap_or_default();
    assert!(manifest.contains("'Acme\\Http\\*' = []"), "{manifest}");
    fs::write(
        tree.app().join("main.nvs"),
        "<?nvs\nrequire './vendor/packages.nvs';\n\nvar $main = new Acme\\Http\\Main();\n",
    )
    .expect("the test writes the entry");

    let out = tree.nvs(&["run", "main.nvs"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "downloaded text\n");
}
