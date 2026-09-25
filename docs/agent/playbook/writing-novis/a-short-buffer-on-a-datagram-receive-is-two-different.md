- **A short buffer on a datagram receive is two different syscall answers.** `recv_from` on Windows
  refuses the call with `WSAEMSGSIZE` and hands back neither a count nor a sender, where the Unixes
  keep what fits and answer normally, so a case pinning truncation is green on CI's Linux legs and
  red locally. Hand the kernel a buffer no datagram can overflow and make the `$max` cut in the
  member — `nvs_stdlib::net`'s `DATAGRAM_CEILING` is that shape.
  [until: gone crates/nvs-stdlib/src/net.rs:DATAGRAM_CEILING]
