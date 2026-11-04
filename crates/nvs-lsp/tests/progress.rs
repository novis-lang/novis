//! The progress a client is shown while the server indexes the workspace.
//!
//! A client that declared `window.workDoneProgress` gets one token created and
//! one run on it: `begin`, any number of `report`s, then `end`. A client that
//! did not declare it gets no progress message at all. The runs here wait on
//! the messages themselves and never on time.

use std::fs;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{InitializeParams, WorkspaceFolder};

/// One progress message the server sent, with its token.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    Create(String),
    Begin(String),
    Report(String),
    End(String),
}

/// The progress message `message` is, if it is one.
fn seen(message: &Message) -> Option<Seen> {
    let token = |params: &serde_json::Value| params["token"].as_str().map(str::to_owned);
    match message {
        Message::Request(request) if request.method == "window/workDoneProgress/create" => {
            token(&request.params).map(Seen::Create)
        }
        Message::Notification(notification) if notification.method == "$/progress" => {
            let token = token(&notification.params)?;
            match notification.params["value"]["kind"].as_str()? {
                "begin" => Some(Seen::Begin(token)),
                "report" => Some(Seen::Report(token)),
                "end" => Some(Seen::End(token)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// A workspace of three small files, with the check scope set to walk it.
fn workspace(name: &str, progress: bool) -> (nvs_repo::Scratch, InitializeParams) {
    let dir = nvs_repo::scratch(&format!("lsp-progress-{name}"));
    for (file, class) in [("a.nvs", "A"), ("b.nvs", "B"), ("c.nvs", "C")] {
        fs::write(dir.join(file), format!("<?nvs\nclass {class} {{}}\n")).expect("a fixture file");
    }
    let mut capabilities = serde_json::json!({});
    if progress {
        capabilities = serde_json::json!({ "window": { "workDoneProgress": true } });
    }
    let params = InitializeParams {
        workspace_folders: Some(vec![WorkspaceFolder {
            uri: nvs_lsp::uri_of(&dir).expect("a scratch path is UTF-8"),
            name: "workspace".to_owned(),
        }]),
        initialization_options: Some(serde_json::json!({ "check": { "scope": "workspace" } })),
        capabilities: serde_json::from_value(capabilities).expect("the protocol's own shape"),
        ..InitializeParams::default()
    };
    (dir, params)
}

/// Asserts `seen` is one created token and one run on it, in order:
/// `begin`, any number of `report`s, then `end`.
fn assert_one_run(seen: &[Seen]) {
    let Some(Seen::Create(token)) = seen.first() else {
        panic!("the first progress message creates the token: {seen:?}");
    };
    assert_eq!(
        seen.get(1),
        Some(&Seen::Begin(token.clone())),
        "the run begins right after the token is created: {seen:?}"
    );
    assert_eq!(
        seen.last(),
        Some(&Seen::End(token.clone())),
        "the run ends, and nothing follows its end: {seen:?}"
    );
    let middle = &seen[2..seen.len() - 1];
    assert!(
        middle.iter().all(|one| *one == Seen::Report(token.clone())),
        "only reports on the same token come between the begin and the end: {seen:?}"
    );
}

/// Queues `messages` between the handshake and `shutdown`/`exit`, runs the
/// server over all of them, and returns every message it sent.
fn serve_queued(params: InitializeParams, messages: Vec<Message>) -> Vec<Message> {
    let (server, client) = Connection::memory();
    let send = |message: Message| client.sender.send(message).expect("the channel is open");
    send(Message::Request(Request::new(
        RequestId::from(1),
        "initialize".to_owned(),
        params,
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
    client.receiver.try_iter().collect()
}

/// `workspace/symbol`, which settles every waiting entry before it is answered.
fn workspace_symbols() -> Message {
    Message::Request(Request::new(
        RequestId::from(2),
        "workspace/symbol".to_owned(),
        serde_json::json!({ "query": "" }),
    ))
}

#[test]
fn a_client_that_shows_progress_sees_one_indexing_run_from_begin_to_end() {
    let (_dir, params) = workspace("idle", true);
    let (server, client) = Connection::memory();
    let serving = std::thread::spawn(move || nvs_lsp::serve(&server));
    let send = |message: Message| client.sender.send(message).expect("the server is reading");
    // A generous bound that only a hung server reaches: every wait below is
    // for a message the server sends without being asked again.
    let next = || {
        client
            .receiver
            .recv_timeout(Duration::from_secs(120))
            .expect("the server sends its next message")
    };

    send(Message::Request(Request::new(
        RequestId::from(1),
        "initialize".to_owned(),
        params,
    )));
    assert!(
        matches!(next(), Message::Response(_)),
        "the handshake answer"
    );
    send(Message::Notification(Notification::new(
        "initialized".to_owned(),
        serde_json::json!({}),
    )));

    // The idle turns settle the queue with no request asking for it, and the
    // run ends once they have.
    let mut progress = Vec::new();
    loop {
        let message = next();
        // A client answers the request that creates the token, as VS Code does.
        if let Message::Request(request) = &message {
            send(Message::Response(Response::new_ok(
                request.id.clone(),
                serde_json::json!(null),
            )));
        }
        if let Some(one) = seen(&message) {
            let ended = matches!(one, Seen::End(_));
            progress.push(one);
            if ended {
                break;
            }
        }
    }

    send(Message::Request(Request::new(
        RequestId::from(1000),
        "shutdown".to_owned(),
        serde_json::json!(null),
    )));
    loop {
        let message = next();
        if matches!(&message, Message::Response(response) if response.id == RequestId::from(1000)) {
            break;
        }
        progress.extend(seen(&message));
    }
    send(Message::Notification(Notification::new(
        "exit".to_owned(),
        serde_json::json!(null),
    )));
    serving
        .join()
        .expect("the server thread did not panic")
        .expect("the server served without a protocol error");

    assert_one_run(&progress);
}

#[test]
fn a_whole_tree_request_ends_the_indexing_run_it_settles() {
    let (_dir, params) = workspace("settled", true);
    let sent = serve_queued(params, vec![workspace_symbols()]);
    let progress: Vec<Seen> = sent.iter().filter_map(seen).collect();
    assert_one_run(&progress);

    // The run ends before the server answers `shutdown`, so a client that
    // leaves next has no run left open.
    let end = sent
        .iter()
        .position(|message| matches!(seen(message), Some(Seen::End(_))))
        .expect("the run ends");
    let shutdown = sent
        .iter()
        .position(|message| {
            matches!(message, Message::Response(response) if response.id == RequestId::from(1000))
        })
        .expect("`shutdown` is answered");
    assert!(end < shutdown, "the end comes before the `shutdown` answer");
}

#[test]
fn a_client_that_does_not_show_progress_is_sent_none() {
    let (_dir, params) = workspace("hidden", false);
    let sent = serve_queued(params, vec![workspace_symbols()]);
    let progress: Vec<Seen> = sent.iter().filter_map(seen).collect();
    assert_eq!(
        progress,
        Vec::new(),
        "no progress for a client that did not declare it"
    );
    assert!(
        !sent
            .iter()
            .any(|message| matches!(message, Message::Notification(n) if n.method == "$/progress")),
        "no `$/progress` at all: {sent:?}"
    );
}
