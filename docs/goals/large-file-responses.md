---
looper: 1
title: "A file response of any size is streamed from disk, and a resumed download is never spliced"
summary: "`Core\\Response::sendFile` and a static mount send a file of several gigabytes with memory bounded per connection, and a `Range` whose `If-Range` names an old tag gets the whole file."
context: [crates/nvs-server/src/statics.rs, crates/nvs-server/src/serve.rs]
stages:
  - id: 1
    title: a file body is read one chunk at a time
    status: open
    context: [crates/nvs-server/src/statics.rs, crates/nvs-server/src/serve.rs, crates/nvs-cli/src/serve.rs]
    extra:
      context:
        rules: [http-server/static-serving-is-one-policy, security/response-body-is-one-typed-member, programs/memory-priority]
        shapes: []
  - id: 2
    title: "`If-Range` is honoured"
    status: open
    context: [crates/nvs-server/src/statics.rs, crates/nvs-server/src/serve.rs]
    extra:
      context:
        rules: [http-server/static-serving-is-one-policy, errors/ambiguous-input-refused]
        shapes: []
  - id: 3
    title: the rule, the reference card and the feature proofs
    status: open
    context: [docs/rules/http-server/static-serving-is-one-policy.md, crates/nvs-stdlib/src/response.rs, docs/examples/core/Response/sendFile/about.md]
    extra:
      context:
        rules: [http-server/static-serving-is-one-policy, core-api/reference-card, testing/feature-proofs]
        shapes: [A rule fragment, A `Core` member]
checks:
  - id: file-body-chunked
    stage: 1
    name: "nvs-server (a file on disk is sent one chunk at a time under an exact length, and a file that shrinks while it is sent ends the body in an error)"
    run: [cargo, test, --lib, "statics::"]
    tests: [a_file_on_disk_is_sent_one_chunk_at_a_time, a_file_that_shrinks_while_it_is_sent_ends_in_an_error]
  - id: sparse-file-live
    stage: 1
    name: "nvs-server (a sparse four-gigabyte file is answered at once, whole and from a range near its end, without holding it)"
    run: [cargo, test, --lib, "serve::"]
    tests: [a_sparse_file_of_four_gigabytes_is_sent_without_holding_it]
  - id: if-range
    stage: 2
    name: "nvs-server (`If-Range` with the current tag gets the range, and an old tag, a weak tag or a date gets the whole file)"
    run: [cargo, test, --lib, "statics::"]
    tests: [if_range_with_the_current_tag_gets_the_range_and_any_other_gets_the_whole_file]
  - id: if-range-send-file
    stage: 2
    name: "nvs-server (a program's file body reads `If-Range` too)"
    run: [cargo, test, --lib, "serve::"]
    tests: [a_declared_file_body_honours_if_range]
  - id: rules-check
    stage: 3
    name: "the rulebook is consistent"
    run: [bun, nv, rules, --check]
    reach: true
---

# A file response of any size is streamed from disk, and a resumed download is never spliced

## Why

A web application often serves a file behind an access check: the program checks the user, then calls
`Core\Response::sendFile`. The files can be videos, archives or data exports of several gigabytes, and
a browser fetches them with `Range`, both to seek in a video and to resume a broken download.

Today the server reads the whole asked-for range into one buffer before it sends a byte
(`crates/nvs-server/src/statics.rs:123-131`). A video opened with `Range: bytes=0-` costs its full size
in memory, and every seek costs the rest of the file again. The buffer is allocated at the connection
after the isolate has ended, so no request ceiling holds it. That breaks the priority on security and
request isolation and the one on latency at once: memory is unattributed and uncapped, and the first
byte waits for the last.

`If-Range` is not read either (`crates/nvs-server/src/statics.rs:153` copies only `If-None-Match` and
`Range`). A browser resuming a download sends `Range` with `If-Range: <etag>`. If the file changed
since, RFC 9110 § 13.1.5 requires the whole file with a `200`; this server sends a `206` slice of the new
file, and the browser joins it to the old bytes.

Static mounts use the same policy (`crates/nvs-cli/src/serve.rs:964`), so both are fixed at once.

## Standing decisions

**A file body is an open file and a count**, a third `Answer` arm beside `Whole` and `Streaming`
(`crates/nvs-server/src/serve.rs:128`). Each `poll_frame` reads at most one chunk and hands it to
`hyper`. `hyper` polls the body only when its write buffer has room, so what one response holds is one
chunk plus that buffer, whatever the file's size. The chunk is 64 KiB, the size `sendFile` already uses
off a request (`crates/nvs-stdlib/src/response.rs:250`). `size_hint` is exact, so `Content-Length` is
stated and nothing is chunk-encoded. A `HEAD` reads nothing, because `hyper` never polls its body.

**The reads are blocking reads on the connection's own thread.** A connection is one future driven
over a parking stream (`crates/nvs-server/Cargo.toml:22-26`), and its socket reads block the same
way. No thread hop per chunk, and no async file API is added.

**The `ETag` and the bytes come from one open file.** `Source` becomes one `open` that returns the
`Stat` read from the opened handle together with the contents, so a file replaced between the stat
and the read cannot pair one file's tag with another file's bytes. The contents are a disk file, or
bytes already in memory: the case sources use the second (`crates/nvs-server/src/statics.rs:416`,
`crates/nvs-server/src/serve.rs:3123`), and a byte cache behind `Source` would too. `OnDisk::open`
records `exists` and `file` with `nvs_footprint`, the two lines `the_disk_records_each_stat_and_read`
expects.

**A file that ends early ends the body in an error.** `Content-Length` was already sent, so a short
end would read as a whole file. The body's error type becomes one enum carrying the drain cut that
exists now (`DrainPeriodEnded`, `crates/nvs-server/src/serve.rs:197`) and this one, and `hyper` aborts
the connection on either. A read error is the same case.

**`If-Range` is compared strongly against the current `ETag`.** It is read only when `Range` is
present and after `If-None-Match`. A match honours the range. Anything else is the whole file with a
`200`: an old tag, a `W/` tag, an HTTP date (this server sends no `Last-Modified` by rule, so no date
can match), or a value that does not parse. A range that would be a `416` but whose `If-Range` does
not match is that `200` as well, because the range is ignored. `statics::asked` copies `If-Range` too,
so a program's file body reads it (`crates/nvs-server/src/serve.rs:1606`).

**The trade-offs, which the user asked for.** Memory: from the whole range per response to about one
chunk plus `hyper`'s write buffer, O(in-flight) and independent of file size. Latency: the first byte
goes out after one chunk read rather than after the whole range. Simplicity: one more `Answer` arm and
a narrower `Source`. The language does not change: `sendFile` keeps its signature.

**Out of this goal:** `Range` over `Core\Response::stream()`, whose length a program may not know.
A program that wants it parses the header itself.

## Stages

### 1. a file body is read one chunk at a time

**Does:** `sendFile` and static mounts read a file one chunk at a time while `hyper` writes it.

- [ ] `Answer::File` with its `poll_frame`, `is_end_stream` and exact `size_hint`. `crates/nvs-server/src/serve.rs:128`
- [ ] The body's error type becomes one enum with the drain cut and the short file. `crates/nvs-server/src/serve.rs:197`
- [ ] `Source::open` replaces `stat` and `read`, and `send` uses it. `crates/nvs-server/src/statics.rs:98`
- [ ] The two case sources move to `open`. `crates/nvs-server/src/serve.rs:3123`
- [ ] The module doc's *What it spends* is rewritten for one chunk per response. `crates/nvs-server/src/statics.rs:49`
- [ ] The live test over a sparse file, and the unit tests the checks name. `crates/nvs-server/src/statics.rs:388`

### 2. `If-Range` is honoured

**Does:** a `Range` with an `If-Range` that does not match the current tag gets the whole file.

- [ ] `asked` copies `If-Range`. `crates/nvs-server/src/statics.rs:153`
- [ ] `send` evaluates it before the range. `crates/nvs-server/src/statics.rs:196`
- [ ] A program's file body reads it, through `named`. `crates/nvs-server/src/serve.rs:3222`

### 3. the rule, the reference card and the feature proofs

**Does:** the rule, the `sendFile` card and the examples say what is now true.

- [ ] The fragment is rewritten whole: `If-Range`, and a `**Memory:**` bullet. `docs/rules/http-server/static-serving-is-one-policy.md:3`
- [ ] The rule's `guardedBy` names the new tests. `data/rules/http-server/static-serving-is-one-policy.json:9`
- [ ] The `sendFile` card says that a browser can seek in a video and resume a download. `crates/nvs-stdlib/src/response.rs:788`
- [ ] A fourth example: a download behind an access check, with `Content-Disposition`. `docs/examples/core/Response/sendFile/about.md:1`

## Handoff

### State

The goal is written; nothing of it is built. The user asked for it after the question of how a web
application streams a large file behind an access check. § *Standing decisions* holds the design.
The anchors were read when the goal was written.

### Next

**Stage 1: a file body is read one chunk at a time**, one file set: `crates/nvs-server/src/statics.rs`, `crates/nvs-server/src/serve.rs`.

- [ ] `Answer::File` and the error enum (`rule:http-server/static-serving-is-one-policy`). `crates/nvs-server/src/serve.rs:128`
- [ ] `Source::open`, and `send` over it (`rule:http-server/static-serving-is-one-policy`). `crates/nvs-server/src/statics.rs:98`

**Stage 2: `If-Range` is honoured**, the same two files.

- [ ] `asked` and `send` read `If-Range` (`rule:http-server/static-serving-is-one-policy`). `crates/nvs-server/src/statics.rs:153`

### Backlog

- Stage 3 — this file's § *Stages*.
