Only the **previous, finished week's** aggregate is ever sent — never a live stream — at most once a week,
at a randomized point inside the eligible window. An empty week is not sent, a week bucket in the future is
never sent, and a second upload inside the same week is refused locally.

The upload is carried by the next short-lived, user-initiated invocation that notices "telemetry is on and
a finished week is unsent". `nvs update-check` is the natural carrier — an ops team cronning it on
production boxes is exactly the short-lived process this rule wants — and `nvs telemetry upload` exists
for schedulers that want only that. There is no daemon, no timer and no background process.

**A process that has bound a socket records and never uploads.** The auditable claim is one sentence:
`nvs serve` makes zero outbound telemetry connections, under load, while still incrementing
`invocation.serve`. Every claim here is testable against a local listener, because the endpoint is
configuration (`rule:config/telemetry-and-update-endpoints-are-configuration`).

**The carriers are an allow-list.** `carries_upload` in `crates/nvs-cli/src/main.rs` is one `match`
that names every command, and only `update-check` and `telemetry upload` answer yes. A new command
does not compile until it answers, so an allow-list fails closed. The answer reaches the telemetry
module once, at startup, and a process it said no for sends nothing whatever it calls. A service's
stored line answers no. `crates/nvs-cli/tests/telemetry.rs` runs `nvs serve` with a finished week
waiting and the upload URL at a listener, and the listener accepts no connection.

**The window.** A finished week's point is the Monday 00:00 UTC that ends it plus a random offset
under 24 hours, drawn the first time a carrier sees the week and kept in the counter file as `due`
(`rule:tooling/telemetry-counters-are-a-closed-set`). A carrier that runs before the point sends
nothing and prints `telemetry: week <week> is sent from <time>`. The point is never inside the ISO
week of the last accepted upload. An accepted week leaves the file, so `nvs telemetry upload` with
nothing waiting prints `telemetry: no finished week is waiting to be sent`.

**Declined:**
- **A background daemon or timer that phones home.** It is the shape people distrust even when opted
  in, and a second long-lived process to secure. An upload is carried, never initiated by a resident
  process.
- **Uploading from the serving process, even once at startup.** It breaks the one-sentence claim above.
