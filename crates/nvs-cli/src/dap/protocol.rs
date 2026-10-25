//! DAP's wire format: the `Content-Length` frame, and the messages inside it.
//!
//! The framing is the one LSP uses: a header block of `Name: value` lines, a blank line, and a JSON
//! body of exactly `Content-Length` bytes. The messages are serde types written here rather than a
//! protocol crate's, because the adapter reads a handful of requests and writes a handful of
//! answers, and a dependency that parses a schema that small is surface for nothing.
//!
//! A frame is refused past [`MAX_FRAME`] bytes before its body is read, so a client that names a
//! huge length cannot make the adapter allocate it. The largest request a client sends is a
//! `setBreakpoints` for one file, far below the bound.

use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The largest body the adapter reads. Spends at most this much per frame in flight, and one
/// frame is in flight at a time.
pub(crate) const MAX_FRAME: usize = 16 << 20;

/// One message a client sent. A client sends only requests, because the adapter sends no
/// reverse request for it to answer, so a message of another `type` is read and ignored.
#[derive(Deserialize)]
pub(crate) struct Incoming {
    pub(crate) seq: i64,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) command: String,
    #[serde(default)]
    pub(crate) arguments: Value,
}

/// One message the adapter writes. `seq` is the writer's, assigned under its lock, so two
/// threads writing at once still number their frames in the order they reach the wire.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum Outgoing<'a> {
    Response {
        seq: i64,
        request_seq: i64,
        success: bool,
        command: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
    },
    Event {
        seq: i64,
        event: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
    },
}

/// What `initialize` answers: the capabilities the adapter has today. A capability that is not
/// named is one the client must not use, which is DAP's own default.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Capabilities {
    pub(crate) supports_configuration_done_request: bool,
    pub(crate) supports_terminate_request: bool,
    pub(crate) supports_conditional_breakpoints: bool,
    pub(crate) supports_hit_conditional_breakpoints: bool,
    pub(crate) supports_log_points: bool,
}

/// The `launch` request's arguments, as `rule:ide/the-debug-adapter-does-not-wait-for-an-editor`
/// names them. A field the adapter does not read yet is accepted and ignored, as DAP lets a client
/// send fields of its own.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchArguments {
    #[serde(default)]
    pub(crate) mode: Mode,
    pub(crate) program: PathBuf,
    #[serde(default)]
    pub(crate) args: Vec<String>,
    #[serde(default)]
    pub(crate) cwd: Option<PathBuf>,
    /// The variables set in the program's environment before it starts. A `null` value removes
    /// the variable, as VS Code's `launch.json` writes it.
    #[serde(default)]
    pub(crate) env: BTreeMap<String, Option<String>>,
    #[serde(default)]
    pub(crate) stop_on_entry: bool,
    /// The `--filter` of a `test` launch.
    #[serde(default)]
    pub(crate) filter: Option<String>,
    /// The globs of the files a step does not stop in.
    #[serde(default)]
    pub(crate) skip_files: Vec<String>,
}

/// What a launch runs the program as: `nvs run`, `nvs test` or `nvs serve`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    #[default]
    Run,
    Test,
    Serve,
}

/// Reads the next frame's body off `input`, or `None` where the client closed the stream between
/// frames.
///
/// # Errors
///
/// A header block with no `Content-Length`, a length past [`MAX_FRAME`], a stream that ends inside
/// a frame, or the read's own error.
pub(crate) fn read_frame(input: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut length = None;
    let mut line = String::new();
    let mut first = true;
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return if first {
                Ok(None)
            } else {
                Err(invalid("the stream ended inside a frame's header"))
            };
        }
        first = false;
        let header = line.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            let value: usize = value
                .trim()
                .parse()
                .map_err(|_| invalid("a `Content-Length` that is not a number"))?;
            length = Some(value);
        }
    }
    let length = length.ok_or_else(|| invalid("a frame with no `Content-Length`"))?;
    if length > MAX_FRAME {
        return Err(invalid("a frame longer than the adapter reads"));
    }
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    Ok(Some(body))
}

/// Writes `body` as one frame and flushes it, so a client waiting on it is not waiting on a
/// buffer.
///
/// # Errors
///
/// The write's own error, which is a client that has gone.
pub(crate) fn write_frame(output: &mut impl Write, body: &[u8]) -> io::Result<()> {
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(body)?;
    output.flush()
}

fn invalid(what: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame round-trips, a second frame follows it, and the end of the stream between frames is
    /// `None` rather than an error.
    #[test]
    fn a_written_frame_reads_back_and_the_stream_ends_between_frames() {
        let mut wire = Vec::new();
        write_frame(&mut wire, br#"{"seq":1}"#).unwrap();
        write_frame(&mut wire, b"{}").unwrap();
        let mut input = io::Cursor::new(wire);
        assert_eq!(read_frame(&mut input).unwrap().unwrap(), br#"{"seq":1}"#);
        assert_eq!(read_frame(&mut input).unwrap().unwrap(), b"{}");
        assert!(read_frame(&mut input).unwrap().is_none());
    }

    /// A length past the bound is refused before a byte of the body is read.
    #[test]
    fn a_frame_past_the_bound_is_refused() {
        let header = format!("Content-Length: {}\r\n\r\n", MAX_FRAME + 1);
        let error = read_frame(&mut io::Cursor::new(header.into_bytes())).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
