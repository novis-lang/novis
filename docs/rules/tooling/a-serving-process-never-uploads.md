Only the **previous, finished week's** aggregate is ever sent — never a live stream — at most once a week,
at a randomized point inside the eligible window. An empty week is not sent, a week bucket in the future is
never sent, and a second upload inside the same week is refused locally.

The upload is carried by the next short-lived, user-initiated invocation that notices "telemetry is on and
a finished week is unsent". `nvs update-check` is the natural carrier — an ops team cronning it on
production boxes is exactly the short-lived process this rule wants — and `nvs telemetry upload` exists
for schedulers that want only that. There is no daemon, no timer and no background process; that is the
shape people distrust even when opted in, and a second long-lived process to secure.

**A process that has bound a socket records and never uploads.** The auditable claim is one sentence:
`nvs serve` makes zero outbound telemetry connections, under load, while still incrementing
`invocation.serve`. Uploading from the serving process even once at startup was rejected because it breaks
that sentence. Every claim here is testable against a local listener, because the endpoint is
configuration (`rule:config/telemetry-and-update-endpoints-are-configuration`).
