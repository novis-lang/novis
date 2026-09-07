//! The connection, the handshake, and the loop that reads it.
//!
//! One reader thread, one writer thread and this one — the three
//! `rule:ide/the-server-is-synchronous` names, and no fourth. A message is
//! taken off the channel, answered, and the next one is taken; there is no
//! executor to hand it to and nothing here returns a `Future`.
//!
//! **The dispatch is a refusal, and refusing is the correct answer.** A request
//! this server does not answer gets `MethodNotFound` rather than silence,
//! because a client waiting on a response that never comes is a client that
//! hangs. The set that gets a real answer grows as
//! `rule:ide/the-request-set-is-closed`'s list is implemented; the set that
//! gets a refusal is everything else, permanently.

use std::error::Error;

use lsp_server::{Connection, ErrorCode, Message, Response};
use lsp_types::InitializeParams;

use crate::capabilities::initialize_result;

/// What a failure on the wire is reported as.
///
/// A protocol error, a malformed `initialize`, and a writer thread that died
/// are three different types and one outcome — this server cannot continue —
/// so the caller gets the message rather than a taxonomy it would only print.
pub type ServerError = Box<dyn Error + Sync + Send>;

/// Serve LSP on this process's standard input and output until the client
/// says `exit`.
///
/// The two io threads own the descriptors from here on, which is the whole of
/// `rule:ide/stdout-belongs-to-the-protocol`'s mechanism: nothing else in the
/// process may write a byte to stdout while this is running, and a `println!`
/// anywhere in the graph desynchronises the framing rather than printing
/// something a user sees.
///
/// # Errors
///
/// Returns the first protocol or io failure. Both are terminal: a framing
/// error means the stream's position is no longer known.
pub fn run() -> Result<(), ServerError> {
    let (connection, io_threads) = Connection::stdio();
    serve(&connection)?;
    // After `exit`, and only after: joining before the loop returns would
    // block on a reader that is still holding stdin open.
    io_threads.join()?;
    Ok(())
}

/// Serve LSP over `connection`, whatever it is carrying.
///
/// Split from [`run`] so the handshake can be driven over
/// `lsp_server::Connection::memory()`, which is how the acceptance tests read
/// back what `initialize` declared without a subprocess or a real stdio pair.
///
/// # Errors
///
/// As [`run`].
pub fn serve(connection: &Connection) -> Result<(), ServerError> {
    let (id, params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(params)?;
    connection.initialize_finish(id, serde_json::to_value(initialize_result(&params))?)?;

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                // `shutdown` is answered and then waited on: the client sends
                // `exit` next, and `handle_shutdown` consumes it. Returning
                // here is what ends the loop.
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                let refusal = Response::new_err(
                    request.id,
                    ErrorCode::MethodNotFound as i32,
                    format!("`{}` is not answered by this server", request.method),
                );
                connection.sender.send(refusal.into())?;
            }
            // A notification this server does not act on is dropped by
            // definition — the protocol gives it no reply — and a response is
            // an answer to a request this server has not yet learned to send.
            Message::Notification(_) | Message::Response(_) => {}
        }
    }

    Ok(())
}
