A test answers an outbound socket from `Core\Test`'s table, never from a listener. `Core\Test::answerSocket`
registers a URL with the frames a peer should send, beside the `answerHttp` that
`rule:testing/an-outbound-call-is-answered-from-a-table` gives an outbound call; a socket matching that URL
completes its handshake **without connecting**, receives those frames in order and then a close, and
`Core\Test::sentSocket` reads back every frame the program sent. A socket the table does not match throws
naming the URL, exactly as an unmatched call does, so a test that forgot to script a peer fails saying so
rather than reaching the network.

This is what makes a conformance case over a WebSocket writable at all: no port, no listener, no timing
assumption, and the same determinism every other test in the suite has
(`rule:testing/determinism-declared-on-the-test`). A scripted socket is still refused for anything the real
one would be refused for — a `tainted` URL, a scheme the row does not serve, a host outside the grant —
because the table answers a call that has already passed every question
(`rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`), and a received payload is
`tainted` there as it is on the wire.

What a scripted peer cannot reach is proved against a loopback peer in Rust, not approximated here:
masking, fragmentation, the close handshake and a ping are `tungstenite`'s behaviour rather than Novis's
surface, and a table that pretended to them would be asserting its own implementation.
