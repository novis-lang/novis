//! `$/cancelRequest`, and what a cancelled request is answered with.
//!
//! The server answers one message at a time, so a cancel can only stop a request
//! the server has not reached yet. These tests queue the whole exchange before
//! the server starts, which puts every request behind its cancel
//! deterministically, with no timing involved.

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::InitializeParams;

/// Queue `messages` between the handshake and `shutdown`/`exit`, run the server
/// over all of them, and return every response but the handshake's and the
/// shutdown's, in the order they were sent.
fn serve_queued(messages: Vec<Message>) -> Vec<Response> {
    let (server, client) = Connection::memory();
    let send = |message: Message| client.sender.send(message).expect("the channel is open");

    send(Message::Request(Request::new(
        RequestId::from(1),
        "initialize".to_owned(),
        InitializeParams::default(),
    )));
    send(Message::Notification(Notification::new(
        "initialized".to_owned(),
        serde_json::json!({}),
    )));
    for message in messages {
        send(message);
    }
    send(Message::Request(Request::new(
        RequestId::from(1000),
        "shutdown".to_owned(),
        serde_json::json!(null),
    )));
    send(Message::Notification(Notification::new(
        "exit".to_owned(),
        serde_json::json!(null),
    )));

    nvs_lsp::serve(&server).expect("the server served the exchange without a protocol error");
    drop(server);

    client
        .receiver
        .try_iter()
        .filter_map(|message| match message {
            Message::Response(response)
                if response.id != RequestId::from(1) && response.id != RequestId::from(1000) =>
            {
                Some(response)
            }
            _ => None,
        })
        .collect()
}

fn symbols(id: i32) -> Message {
    Message::Request(Request::new(
        RequestId::from(id),
        "textDocument/documentSymbol".to_owned(),
        serde_json::json!({ "textDocument": { "uri": "file:///never-opened.nvs" } }),
    ))
}

fn cancel(id: serde_json::Value) -> Message {
    Message::Notification(Notification::new(
        "$/cancelRequest".to_owned(),
        serde_json::json!({ "id": id }),
    ))
}

#[test]
fn a_cancelled_request_is_answered_with_request_cancelled() {
    let answered = serve_queued(vec![symbols(2), cancel(serde_json::json!(2)), symbols(3)]);

    assert_eq!(
        answered.len(),
        2,
        "every request gets exactly one response: {answered:?}"
    );
    assert_eq!(answered[0].id, RequestId::from(2));
    assert_eq!(
        error_code(&answered[0]),
        Some(ErrorCode::RequestCanceled as i32)
    );

    assert_eq!(answered[1].id, RequestId::from(3));
    assert_eq!(
        error_code(&answered[1]),
        None,
        "the request nobody cancelled is answered: {:?}",
        answered[1]
    );
}

/// The error code `response` carries, and `None` when it carries a result.
fn error_code(response: &Response) -> Option<i32> {
    response
        .response_result
        .as_ref()
        .err()
        .map(|error| error.code)
}

#[test]
fn a_string_id_is_cancelled_the_same_way() {
    let request = Message::Request(Request::new(
        RequestId::from("symbols".to_owned()),
        "textDocument/documentSymbol".to_owned(),
        serde_json::json!({ "textDocument": { "uri": "file:///never-opened.nvs" } }),
    ));
    let answered = serve_queued(vec![request, cancel(serde_json::json!("symbols"))]);

    assert_eq!(answered.len(), 1);
    assert_eq!(answered[0].id, RequestId::from("symbols".to_owned()));
    assert_eq!(
        error_code(&answered[0]),
        Some(ErrorCode::RequestCanceled as i32)
    );
}

#[test]
fn a_cancel_for_an_unknown_request_changes_nothing() {
    // Nothing has the id 99, so the cancel gets no response and stops nothing.
    let answered = serve_queued(vec![symbols(2), cancel(serde_json::json!(99)), symbols(3)]);

    let ids: Vec<RequestId> = answered
        .iter()
        .map(|response| response.id.clone())
        .collect();
    assert_eq!(ids, [2, 3].map(RequestId::from));
    assert!(
        answered
            .iter()
            .all(|response| error_code(response).is_none()),
        "no request was cancelled: {answered:?}"
    );
}

#[test]
fn a_cancel_for_an_answered_request_changes_nothing() {
    // The client waits for the answer to 2 before it cancels 2, so the cancel
    // arrives too late. It gets no response, and the next request is answered.
    let (server, client) = Connection::memory();
    let serving = std::thread::spawn(move || nvs_lsp::serve(&server));
    let send = |message: Message| client.sender.send(message).expect("the server is reading");
    let next_response = || loop {
        match client.receiver.recv().expect("the server is still writing") {
            Message::Response(response) => break response,
            _ => continue,
        }
    };

    send(Message::Request(Request::new(
        RequestId::from(1),
        "initialize".to_owned(),
        InitializeParams::default(),
    )));
    next_response();
    send(Message::Notification(Notification::new(
        "initialized".to_owned(),
        serde_json::json!({}),
    )));

    send(symbols(2));
    let first = next_response();
    assert_eq!(first.id, RequestId::from(2));
    assert_eq!(error_code(&first), None);

    send(cancel(serde_json::json!(2)));
    send(symbols(3));
    let second = next_response();
    assert_eq!(
        second.id,
        RequestId::from(3),
        "the late cancel got no response"
    );
    assert_eq!(error_code(&second), None);

    send(Message::Request(Request::new(
        RequestId::from(4),
        "shutdown".to_owned(),
        serde_json::json!(null),
    )));
    assert_eq!(next_response().id, RequestId::from(4));
    send(Message::Notification(Notification::new(
        "exit".to_owned(),
        serde_json::json!(null),
    )));
    serving
        .join()
        .expect("the server thread did not panic")
        .expect("the server served the exchange without a protocol error");
}

#[test]
fn an_exit_read_while_looking_for_a_cancel_still_ends_the_server() {
    // The last request reads `shutdown` and `exit` off the channel while it looks
    // for a cancel, so the shutdown has to find `exit` there.
    let answered = serve_queued(vec![symbols(2)]);

    assert_eq!(answered.len(), 1);
    assert_eq!(error_code(&answered[0]), None);
}
