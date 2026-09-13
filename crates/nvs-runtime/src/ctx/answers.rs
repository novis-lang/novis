//! `rule:testing/an-outbound-call-is-answered-from-a-table`'s table and
//! `rule:testing/an-outbound-socket-is-answered-by-a-scripted-peer`'s: what a
//! test says an outbound call answers with, what a scripted peer sends over a
//! socket, and what the program sent instead of connecting.
//!
//! **Plain data, and no HTTP in it.** A row is text and bytes — this crate has
//! no URL type, no header map and no client, exactly as it has no calendar
//! behind the fixed clock it holds a nanosecond count for. `nvs_stdlib::test`
//! writes the rows, `nvs_stdlib::http` reads them before it composes anything,
//! and both spellings a `$url` may take — an exact one and a prefix ending in
//! `*` — are [`selected`]'s, which both [`AnswerTable::answer_for`] and
//! [`AnswerTable::socket_for`] are a call to, so no two readers can come to
//! disagree about which row a URL selects.
//!
//! **It lives on the context because a test's isolate is what scopes it**
//! (`rule:testing/isolate-per-test`), which is the scripted answer queue's own
//! reason: a table in a `thread_local` would outlive the test that filled it
//! and answer the next test's call from a row nobody registered.

/// One registered answer — what a matching outbound call comes back with.
#[derive(Clone, Debug)]
pub struct HttpAnswer {
    /// The URL this row answers: matched exactly, or as a prefix where it ends
    /// in `*`.
    pub url: String,
    /// The status the call answers with.
    pub status: u16,
    /// The reply's headers, names lower-cased, as the reply's own members read
    /// them back.
    pub headers: Vec<(String, String)>,
    /// The reply's body, already encoded — a `json` answer is written to JSON
    /// where it is registered, so nothing here knows the difference.
    pub body: Vec<u8>,
}

/// One outbound call the program made while the table was armed.
#[derive(Clone, Debug)]
pub struct HttpSent {
    /// The verb, upper-cased as the wire spells it.
    pub verb: String,
    /// The URL as the program wrote it, before any matching.
    pub url: String,
    /// The headers the request would have carried, names lower-cased.
    pub headers: Vec<(String, String)>,
    /// The bytes the request would have carried.
    pub body: Vec<u8>,
}

/// One RFC 6455 payload, in either direction: a frame a scripted peer plays,
/// or one the program sent over a socket the table answered.
///
/// An enum and not two `Option` fields, because a frame is one kind or the
/// other and a shape admitting both at once would be a state
/// `Core\Socket\Message` has no reading for.
#[derive(Clone, Debug)]
pub enum SocketFrame {
    /// A text frame, already known to be UTF-8 by the type it was written as.
    Text(String),
    /// A binary frame.
    Bytes(Vec<u8>),
}

/// One scripted peer — what a socket opened to a matching URL receives, and
/// what it reports as the subprotocol the handshake chose.
#[derive(Clone, Debug)]
pub struct SocketAnswer {
    /// The URL this peer answers: matched exactly, or as a prefix where it
    /// ends in `*`.
    pub url: String,
    /// The subprotocol the `101` chose, or `None` for a handshake that chose
    /// none.
    pub protocol: Option<String>,
    /// The frames the peer sends, in order; the socket is closed by the peer
    /// after the last of them.
    pub frames: Vec<SocketFrame>,
}

/// `rule:testing/an-outbound-call-is-answered-from-a-table`'s two halves and
/// `rule:testing/an-outbound-socket-is-answered-by-a-scripted-peer`'s: the rows
/// a test registered, and the calls and frames it has taken since.
///
/// **What it spends:** eight words per request for the four empty vectors, and
/// nothing else anywhere a test did not register a row — every context outside
/// a test has all four empty for its whole life, and neither sent list grows
/// until [`Self::is_armed`] holds, which no request or `nvs run` can make true.
#[derive(Debug, Default)]
pub struct AnswerTable {
    /// The rows, in registration order.
    answers: Vec<HttpAnswer>,
    /// What the program sent, oldest first — the order
    /// `Core\Test::sentHttp` hands back.
    sent: Vec<HttpSent>,
    /// The scripted peers, in registration order.
    sockets: Vec<SocketAnswer>,
    /// Every frame the program sent over a scripted socket, oldest first — the
    /// order `Core\Test::sentSocket` hands back.
    sent_frames: Vec<SocketFrame>,
}

impl AnswerTable {
    /// Registers `answer`, taking this context off the network.
    pub fn answer(&mut self, answer: HttpAnswer) {
        self.answers.push(answer);
    }

    /// Registers a scripted peer, taking this context off the network exactly
    /// as an answer does: the switch is one switch, so a test that scripts a
    /// socket and then makes a call still makes no connection.
    pub fn answer_socket(&mut self, peer: SocketAnswer) {
        self.sockets.push(peer);
    }

    /// Whether any row has been registered, which is the all-or-nothing switch
    /// the rule is named for: from the first one on, every outbound call this
    /// context makes is answered from here or throws.
    #[must_use]
    pub fn is_armed(&self) -> bool {
        !self.answers.is_empty() || !self.sockets.is_empty()
    }

    /// The row answering `url`, or `None` for a call the table does not name —
    /// see [`selected`] for which row that is.
    #[must_use]
    pub fn answer_for(&self, url: &str) -> Option<&HttpAnswer> {
        selected(&self.answers, url, |answer| &answer.url)
    }

    /// The scripted peer answering `url`, or `None` for a socket the table does
    /// not name — see [`selected`] for which row that is.
    #[must_use]
    pub fn socket_for(&self, url: &str) -> Option<&SocketAnswer> {
        selected(&self.sockets, url, |peer| &peer.url)
    }

    /// Writes down one call the program made.
    pub fn record(&mut self, sent: HttpSent) {
        self.sent.push(sent);
    }

    /// Writes down one frame the program sent over a scripted socket.
    pub fn record_frame(&mut self, frame: SocketFrame) {
        self.sent_frames.push(frame);
    }

    /// Every call the program has made, oldest first.
    #[must_use]
    pub fn sent(&self) -> &[HttpSent] {
        &self.sent
    }

    /// Every frame the program has sent over a scripted socket, oldest first.
    #[must_use]
    pub fn sent_frames(&self) -> &[SocketFrame] {
        &self.sent_frames
    }
}

/// The row of `rows` that answers `url`, under the one matching both readers
/// share.
///
/// An exact row wins over a prefix one, and the **longest** prefix wins among
/// prefixes, so a row written for one endpoint answers it even where a
/// `https://api.example.com/*` covering the whole origin was registered as
/// well. A later row wins a tie with an earlier one of the same length, which
/// is what makes registering a URL twice read as replacing its answer rather
/// than as shadowing it from behind.
fn selected<'a, T>(rows: &'a [T], url: &str, url_of: fn(&T) -> &str) -> Option<&'a T> {
    let mut best: Option<&T> = None;
    for row in rows {
        let registered = url_of(row);
        let Some(prefix) = registered.strip_suffix('*') else {
            if registered == url {
                return Some(row);
            }
            continue;
        };
        if url.starts_with(prefix) && best.is_none_or(|held| url_of(held).len() <= registered.len())
        {
            best = Some(row);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::{AnswerTable, HttpAnswer};

    /// A row with no body, at `url`.
    fn row(url: &str, status: u16) -> HttpAnswer {
        HttpAnswer {
            url: url.to_owned(),
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    #[test]
    fn an_empty_table_is_not_armed_and_answers_nothing() {
        let table = AnswerTable::default();
        assert!(!table.is_armed());
        assert!(table.answer_for("https://api.example.com/one").is_none());
    }

    #[test]
    fn an_exact_row_beats_a_prefix_one_however_they_were_registered() {
        let mut table = AnswerTable::default();
        table.answer(row("https://api.example.com/*", 500));
        table.answer(row("https://api.example.com/one", 200));
        assert!(table.is_armed());
        let answered = table.answer_for("https://api.example.com/one");
        assert_eq!(answered.map(|answer| answer.status), Some(200));

        let mut reversed = AnswerTable::default();
        reversed.answer(row("https://api.example.com/one", 200));
        reversed.answer(row("https://api.example.com/*", 500));
        let answered = reversed.answer_for("https://api.example.com/one");
        assert_eq!(answered.map(|answer| answer.status), Some(200));
    }

    #[test]
    fn the_longest_prefix_wins_and_a_later_row_wins_a_tie() {
        let mut table = AnswerTable::default();
        table.answer(row("https://api.example.com/*", 500));
        table.answer(row("https://api.example.com/users/*", 404));
        table.answer(row("https://api.example.com/users/*", 204));
        let answered = table.answer_for("https://api.example.com/users/7");
        assert_eq!(answered.map(|answer| answer.status), Some(204));
        let answered = table.answer_for("https://api.example.com/orders/7");
        assert_eq!(answered.map(|answer| answer.status), Some(500));
    }

    #[test]
    fn a_prefix_row_answers_nothing_outside_its_prefix() {
        let mut table = AnswerTable::default();
        table.answer(row("https://api.example.com/users/*", 200));
        assert!(
            table
                .answer_for("https://api.example.org/users/7")
                .is_none()
        );
        assert!(table.answer_for("https://api.example.com/users").is_none());
    }
}
