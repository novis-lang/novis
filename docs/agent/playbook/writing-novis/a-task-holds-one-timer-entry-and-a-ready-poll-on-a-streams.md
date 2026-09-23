- **A task holds one timer entry, and a `Ready` poll on a stream's other interest lifts it,
  cancelling an idle read timeout.** `hyper` polls the readable half (`Pending`, deadline filed)
  then the writable half in the same pass (`Ready`, deadline lifted), so the connection parks with
  no clock, and the tell is a test hanging for exactly the client's patience.
  `nvs_host::NvsStream::timed` records which interest filed the entry; anything else parking two
  interests of one stream under one task inherits the question. [until: reviewed 2026-09-06]
