**A file is resolved once per isolate, the first time execution reaches it, and never re-resolved
on a second reference within the same run.** The compiled unit a running isolate holds was cloned
out of the cache at the moment of first touch, and it is never mutated in place — only ever replaced
at the path-pointer layer above it. A request that resolved a file before an edit completes on the
pre-edit unit even if the edit and a successful recompile land before it finishes; the next request
to resolve the same path gets the new one.

The compiled code is the *only* thing shared across requests. A request's heap arena, its limits and
ceilings, its `Core\Request`/`Core\Server`/`Core\Session` state and its panic containment are
untouched by a swap: a hot-reload event changes what code a *future* request compiles to, never how
isolated any request's execution of that code is (`rule:security/isolate-shares-nothing`). A
connection isolate is the long-lived case of the same rule
(`rule:concurrency/a-connection-keeps-its-compiled-unit`).

Old generations are retained only while some in-flight request still holds one, bounded by that
request's own wall-clock and CPU limits — O(in-flight), never O(edits ever made).
