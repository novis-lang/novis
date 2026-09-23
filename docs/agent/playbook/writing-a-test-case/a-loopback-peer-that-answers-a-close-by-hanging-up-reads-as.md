- **A loopback peer that answers a close by hanging up reads as a protocol error, not as the end.**
  `tungstenite` writes its close echo only on the next read, write or flush, so a peer thread that
  breaks out of its loop and drops the socket sends a bare FIN and the client's codec answers
  `ResetWithoutClosingHandshake`. A case asserting `null` after a close rests on the read loop in
  `crates/nvs-stdlib/src/http/socket.rs` reading a failure after *this* end closed as the end; one
  about the *peer* closing has to make the peer flush first.
  [until: gone crates/nvs-stdlib/src/http/transport.rs:talking_origin]
