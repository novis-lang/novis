Two requests open side by side with what differs marked, and a recorded request is sent again to the
development server only after the developer confirms it in a dialog that names what will be sent.

**Compare** sets the recorded input of each — method, path, query, headers and body — against the
other, then the response — its status, headers and body, a JSON body compared structurally — the
duration, the queries aligned by the call that sent them, one row per call site with each statement
and how many times it ran there, the dumps and the log lines. A second request is picked by marking it in any list, and a request's own
view offers the previous request to the same route. A redacted header shows as redacted on both
sides, never as a difference.

**Replay** sends the recorded method, path, query, headers and body to the development `nvs serve`
that started this service, and to nothing else: the target is the address that server gave its child,
never a value from the browser, so the action cannot be turned into a request to another host. A
service started by hand has no target, and the action says so. The dialog names the method, the full
path and the target, opens with the cancel button focused, and asks every time, because a replay runs
the handler's side effects again. A header the spool redacted is listed in the dialog with an empty
field: a value typed there is sent once and is never written anywhere. The replayed request gets its
own trace id, and the viewer opens it beside the original in the compare view.

Replay is a request that changes something, so it needs an allowed `Origin`
(`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).

- **Memory:** a replay holds one recorded request's input, under the spool's own body cap, while it is
  sent.

**Declined:**
- **Replaying to a URL the developer types.** A browser-facing listener that sends requests where it
  is told is a request forger for any page that reaches it.
- **A setting that stops the dialog asking.** The cost of one keystroke is smaller than the cost of a
  repeated payment or a repeated email.
