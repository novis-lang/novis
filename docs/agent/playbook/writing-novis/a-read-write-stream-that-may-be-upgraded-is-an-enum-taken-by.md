- **A `Read`/`Write` stream that may be upgraded is an enum taken by value, not a `&mut` swap.**
  `NvsTls::over` consumes its `NvsTcp`, so securing in place needs a placeholder variant nobody may
  observe; `nvs_stdlib::mail::Session::secure` takes `self` and returns a new one instead. Any
  in-band upgrade owes the second half too: refuse, rather than clear, a non-empty read buffer,
  since bytes held from before the handshake replayed after it is the *NO STARTTLS* injection class.
  [until: gone crates/nvs-stdlib/src/mail.rs:NvsTls::over]
