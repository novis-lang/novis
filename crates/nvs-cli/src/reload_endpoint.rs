//! The reload endpoint of a running `nvs serve`: [`nvs_host::endpoint::Endpoint`], opened in the
//! data folder's `run/` before `READY=1` and removed when the process ends
//! (`rule:config/a-reload-command-checks-now-and-waits`).
//!
//! [`open`] first removes every endpoint in `run/` that nobody listens at, which is what a process
//! leaves behind when it was killed before its drain ended. Then it binds this process's own,
//! named `serve-<pid>-<start>`, and starts one thread that accepts on it. An endpoint that cannot
//! be opened is one warning, and the server serves without it: `nvs reload` does not reach that
//! process, and the timed check still does.
//!
//! **A Unix socket path has a length limit**, 108 bytes on Linux and 104 on macOS. A data folder
//! whose `run/` path is longer gets no endpoint and the warning names the path. A shorter
//! `--data` is the fix, and no second place for the socket exists to keep track of.
//!
//! A connection reads at most [`LINE`] bytes and is closed. Cost: one thread per process, blocked
//! in its accept, and one socket or pipe instance. Nothing on the request path.

use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use nvs_host::endpoint::{self, Endpoint};

/// The most a request to the endpoint may send: `reload 1\n` and room to spare.
pub(crate) const LINE: usize = 64;

/// This process's endpoint, removed when the value is dropped.
pub(crate) struct Opened {
    endpoint: Arc<Endpoint>,
}

impl Drop for Opened {
    fn drop(&mut self) {
        if let Err(err) = self.endpoint.remove() {
            eprintln!(
                "warning: the reload endpoint `{}` was not removed: {err}",
                self.endpoint.path().display()
            );
        }
    }
}

/// Opens this process's endpoint in the data folder's `run/`, or warns once and returns `None`.
/// A process with no usable data folder has no endpoint and warns nothing more: the data folder's
/// own warning was printed at the start.
pub(crate) fn open() -> Option<Opened> {
    let folder = nvs_config::data::current()?;
    let run = folder.run();
    match bind(&run) {
        Ok(endpoint) => Some(Opened { endpoint }),
        Err(err) => {
            eprintln!(
                "warning: `nvs reload` cannot reach this server, because its endpoint in `{}` \
                 could not be opened: {err}",
                run.display()
            );
            None
        }
    }
}

/// Removes the stale endpoints in `run`, binds this process's own and starts its thread.
fn bind(run: &Path) -> std::io::Result<Arc<Endpoint>> {
    nvs_config::data::create_private_root(run)?;
    // Each live endpoint reached here closes this connection at once, and each stale one is
    // removed.
    drop(endpoint::reach_all(run)?);
    let start_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    let path = run.join(endpoint::name(std::process::id(), start_ms));
    let endpoint = Arc::new(Endpoint::bind(&path)?);
    let accepting = Arc::clone(&endpoint);
    std::thread::Builder::new()
        .name("nvs-reload".to_owned())
        .spawn(move || accept(&accepting))?;
    Ok(endpoint)
}

/// Accepts until the endpoint fails, which ends the thread. Each connection is read up to
/// [`LINE`] bytes and closed.
fn accept(endpoint: &Endpoint) {
    while let Ok(connection) = endpoint.accept() {
        let mut line = Vec::with_capacity(LINE);
        let _ = connection.take(LINE as u64).read_to_end(&mut line);
    }
}
