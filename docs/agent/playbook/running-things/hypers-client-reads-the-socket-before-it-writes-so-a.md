- **`hyper`'s client reads the socket before it writes, so a blocking transport under it wedges
  before the request ever leaves.** What you get is a thirty-second hang and
  `hyper::Error(Canceled, …)`, which names the server's idle bound rather than the cause. Give a
  client the `WouldBlock` transport the server half has — `nvs_config::control`'s `Client`, adapted
  by `nvs_server::io::Nonblocking` — and read its connection future finishing as the answer having
  arrived, not as a truncation. [until: reviewed 2026-09-14]
