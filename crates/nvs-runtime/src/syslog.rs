//! `[log] target = "syslog"`: each record as one datagram to the local syslog daemon's socket,
//! `rule:errors/engine-floor`'s third destination.
//!
//! **The framing is the local one, not RFC 5424's network one.** A datagram is `<PRI>nvs: ` and
//! the rendered record with its line ending removed, which is what `/dev/log` takes from the C
//! library's own `syslog(3)`: the daemon stamps the time and the host itself. `PRI` is the `user`
//! facility (1) times eight plus the record's severity, read from
//! [`Level::syslog_severity`](nvs_render::Level::syslog_severity), so `rule:errors/log-level`'s
//! table is the only place a level is turned into a number.
//!
//! **One socket for the process.** The first record opens an unbound datagram socket and every
//! writer after it sends through the same one, addressed to `ENDPOINT` each time, so a daemon
//! that restarts is reached again at its next record with nothing to reconnect. A send that fails —
//! no daemon listening, a record larger than the socket takes — is the caller's to swallow, which
//! is the floor's answer to every failed write.
//!
//! Only a Unix host has the socket. `nvs_config::log::validate` refuses the target at boot
//! anywhere else, so [`send`] there is an error nothing reaches.
//!
//! **What it spends** (`rule:programs/memory-priority`): one descriptor for the process, opened at
//! the first syslog record, and one `Vec` per record the size of its line. Nothing per request.

use nvs_render::Level;
use std::io;

/// The `user` facility's code, which is what a program that is not part of the system logs as.
const FACILITY_USER: u8 = 1;

/// The name a daemon files every record under.
const TAG: &str = "nvs";

/// The local daemon's socket: macOS keeps it under `/var/run`, every other Unix at `/dev/log`.
#[cfg(unix)]
pub const ENDPOINT: &str = if cfg!(target_os = "macos") {
    "/var/run/syslog"
} else {
    "/dev/log"
};

/// `line` at `level` as the one datagram the daemon reads: priority, tag and the line without its
/// ending.
#[must_use]
pub fn frame(level: Level, line: &str) -> Vec<u8> {
    let priority = u16::from(FACILITY_USER) * 8 + u16::from(level.syslog_severity());
    let line = line.trim_end_matches(['\n', '\r']);
    format!("<{priority}>{TAG}: {line}").into_bytes()
}

/// Sends `line` at `level` to [`ENDPOINT`].
///
/// # Errors
///
/// The socket could not be opened or the send failed, both swallowed by the floor.
#[cfg(unix)]
pub fn send(level: Level, line: &str) -> io::Result<()> {
    use std::os::unix::net::UnixDatagram;
    use std::sync::OnceLock;

    static SOCKET: OnceLock<Option<UnixDatagram>> = OnceLock::new();
    let socket = SOCKET
        .get_or_init(|| UnixDatagram::unbound().ok())
        .as_ref()
        .ok_or_else(|| io::Error::other("no datagram socket for syslog"))?;
    socket.send_to(&frame(level, line), ENDPOINT).map(|_| ())
}

/// A host with no syslog socket: the boot refuses the target, so nothing calls this.
///
/// # Errors
///
/// Always [`io::ErrorKind::Unsupported`].
#[cfg(not(unix))]
pub fn send(_level: Level, _line: &str) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The priority is the `user` facility plus `rule:errors/log-level`'s severity, and the line
    /// arrives without the ending a JSON Lines record carries.
    #[test]
    fn a_record_is_framed_with_its_priority_and_without_its_line_ending() {
        assert_eq!(
            frame(Level::Error, "{\"msg\":\"down\"}\n"),
            b"<11>nvs: {\"msg\":\"down\"}".to_vec()
        );
        assert_eq!(frame(Level::Debug, "x"), b"<15>nvs: x".to_vec());
        assert_eq!(frame(Level::Critical, "x\r\n"), b"<10>nvs: x".to_vec());
    }
}
