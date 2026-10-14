---
looper: 1
title: "A file response of any size is streamed from disk, and a resumed download is never spliced"
summary: "`Core\\Response::sendFile` and a static mount send a file of several gigabytes with memory bounded per connection, a `Range` whose `If-Range` names an old tag gets the whole file, and `Core\\Response::stream` with a declared length answers a range under the same policy."
context: [crates/nvs-server/src/statics.rs, crates/nvs-server/src/serve.rs]
stages:
  - id: 1
    title: a file body is read one chunk at a time
    status: done
    context: [crates/nvs-server/src/statics.rs, crates/nvs-server/src/serve.rs, crates/nvs-cli/src/serve.rs]
    extra:
      context:
        rules: [http-server/static-serving-is-one-policy, security/response-body-is-one-typed-member, programs/memory-priority]
        shapes: []
  - id: 2
    title: "`If-Range` is honoured"
    status: done
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
  - id: 4
    title: "a stream with a declared length answers a range"
    status: open
    context: [crates/nvs-stdlib/src/response.rs, crates/nvs-server/src/serve.rs, crates/nvs-server/src/statics.rs]
    extra:
      context:
        rules: [http-server/static-serving-is-one-policy, concurrency/a-stream-that-outlives-its-request-is-a-connection, security/response-body-is-one-typed-member, core-api/reference-card, testing/feature-proofs]
        shapes: [A `Core` member, A diagnostic]
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
  - id: stream-range
    stage: 4
    name: "nvs-server (a stream with a length answers a range with its window, a write past the window throws, a short stream cuts the connection, and a stream without a length ignores `Range`)"
    run: [cargo, test, --lib, "serve::"]
    tests: [a_stream_with_a_length_answers_a_range_with_its_window, a_stream_that_writes_past_its_window_throws, a_stream_that_ends_before_its_window_is_full_cuts_the_connection, a_stream_without_a_length_ignores_range]
  - id: rules-check-end
    stage: 4
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

Data that is not a file on this disk — an object in storage, a blob in a database, a generated export —
goes out through `Core\Response::stream`, which answers no `Range` at all: its headers go out when it
is called (`crates/nvs-stdlib/src/response.rs:585`) and its body is chunked. A video from storage
cannot be seeked in, and a broken download from it starts again from the first byte.

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
end would read as a whole file. The body's error type is one enum, `BodyCut`
(`crates/nvs-server/src/serve.rs:217`), carrying the drain cut and this one, and `hyper` aborts the
connection on either. A read error is the same case, and a short stream in stage 4 is too.

**`If-Range` is compared strongly against the current `ETag`.** It is read only when `Range` is
present and after `If-None-Match`. A match honours the range. Anything else is the whole file with a
`200`: an old tag, a `W/` tag, an HTTP date (this server sends no `Last-Modified` by rule, so no date
can match), or a value that does not parse. A range that would be a `416` but whose `If-Range` does
not match is that `200` as well, because the range is ignored. `statics::asked` copies `If-Range` too,
so a program's file body reads it (`crates/nvs-server/src/serve.rs:1606`).

**A stream answers a range when the program states the length, and the program writes the window.**
The user's call, over the server discarding bytes the program wrote from byte 0: a seek near the end
of a large stream would make the program produce everything before it.

- `Core\Response::stream(string $contentType, ?int $length = null, ?string $etag = null)`. With no
  `length` it is what it is now: chunked, no `Accept-Ranges`, and `Range` ignored, which RFC 9110
  allows.
- With `length`, the server applies the same decision a file gets — `If-None-Match`, `If-Range`,
  `Range`, `304`, `206` and `416` — states `Content-Length` and `Accept-Ranges: bytes`, and sends the
  head. `Core\Response\Stream::offset(): int` and `Core\Response\Stream::length(): int` give the window
  the program writes: the whole body, the asked-for range, or nothing for a `304` and a `416`.
- `etag` is the program's version of the bytes, sent as a strong tag. Without it, `If-None-Match` and
  `If-Range` never match, so a resumed download gets the whole body and a bare `Range` still seeks. It
  is a header, so a `tainted` value does not compile, and a value a header cannot carry, or one with a
  `"`, throws a `LogicError`, as `$contentType` does now. A negative `length` throws one too.
- The count is exact. A `write` that would pass the window's end throws a `LogicError` and sends
  nothing. A program that ends, or throws, before the window is full ends the body in the error the
  short file uses, so the connection is cut.
- A declared status wins only over the decision's `200`, as it does for a file body, and the
  program's own headers apply after the decision's.
- Off a request there is no `Range`: the window is the whole length, and the exact count still holds.

**One decision, two callers.** The rule says a second policy is a second security model, so the
conditional and range decision is one pure function over header text, in `nvs-runtime`, which both
`nvs-server` and `nvs-stdlib` reach. `statics::send` and `stream` call it. The media type, the
`Cache-Control` and the file's own tag stay the static policy's.

**The trade-offs, which the user asked for.** Memory: from the whole range per file response to about
one chunk plus `hyper`'s write buffer, O(in-flight) and independent of size; a stream holds one part
in flight, as now. Latency: the first byte goes out after one chunk read rather than after the whole
range, and a seek in a stream reads only what the browser asked for. Simplicity: one more `Answer`
arm, a narrower `Source`, and one decision shared by files and streams. The language grows by two
named parameters on `stream` and two methods on `Core\Response\Stream`; `sendFile` keeps its signature.

## Stages

### 1. a file body is read one chunk at a time

**Does:** `sendFile` and static mounts read a file one chunk at a time while `hyper` writes it.

- [x] `Answer::File` with its `poll_frame`, `is_end_stream` and exact `size_hint`. `crates/nvs-server/src/serve.rs:128`
- [x] The body's error type becomes one enum with the drain cut and the short file. `crates/nvs-server/src/serve.rs:197`
- [x] `Source::open` replaces `stat` and `read`, and `send` uses it. `crates/nvs-server/src/statics.rs:98`
- [x] The two case sources move to `open`. `crates/nvs-server/src/serve.rs:3123`
- [x] The module doc's *What it spends* is rewritten for one chunk per response. `crates/nvs-server/src/statics.rs:49`
- [x] The live test over a sparse file, and the unit tests the checks name. `crates/nvs-server/src/statics.rs:388`

### 2. `If-Range` is honoured

**Does:** a `Range` with an `If-Range` that does not match the current tag gets the whole file.

- [x] `asked` copies `If-Range`. `crates/nvs-server/src/statics.rs:153`
- [x] `send` evaluates it before the range. `crates/nvs-server/src/statics.rs:196`
- [x] A program's file body reads it, through `named`. `crates/nvs-server/src/serve.rs:3222`

### 3. the rule, the reference card and the feature proofs

**Does:** the rule, the `sendFile` card and the examples say what is now true.

- [ ] The fragment is rewritten whole: `If-Range`, and a `**Memory:**` bullet. `docs/rules/http-server/static-serving-is-one-policy.md:3`
- [ ] The rule's `guardedBy` names the new tests. `data/rules/http-server/static-serving-is-one-policy.json:9`
- [ ] The `sendFile` card says that a browser can seek in a video and resume a download. `crates/nvs-stdlib/src/response.rs:788`
- [ ] A fourth example: a download behind an access check, with `Content-Disposition`. `docs/examples/core/Response/sendFile/about.md:1`

### 4. a stream with a declared length answers a range

**Does:** `Core\Response::stream` with `length` answers `Range`, `If-Range` and `If-None-Match`, and the program writes the window.

- [ ] The decision moves to `nvs-runtime` as one function, and `statics::send` calls it. `crates/nvs-server/src/statics.rs:196`
- [ ] `stream` takes `length` and `etag`, decides, and sends the head. `crates/nvs-stdlib/src/response.rs:1882`
- [ ] `Core\Response\Stream::offset` and `length`, and `write` holds the exact count. `crates/nvs-stdlib/src/response.rs:556`
- [ ] A streamed body with a length has an exact `size_hint`, and a short end is the body error. `crates/nvs-server/src/serve.rs:2392`
- [ ] The cards of `stream`, `offset` and `length`, and their feature proofs, including a video from storage that a browser can seek in. `crates/nvs-stdlib/src/response.rs:584`
- [ ] The fragment states that a stream with a length gets the same decision. `docs/rules/http-server/static-serving-is-one-policy.md:3`

## Handoff

### State

Stages 1 and 2 are built and their four checks pass. A file body is `Answer::File`, read one
`FILE_CHUNK` per poll (`crates/nvs-server/src/serve.rs:129`). `statics::Source` is one `open`
(`crates/nvs-server/src/statics.rs:137`), and the body error is `BodyCut`. `If-Range` is
`statics::still_the_same`, and `statics::asked` copies it. The rule fragment, the `sendFile` card and
the proofs do not mention any of this yet.

The acceptance sweep's nightly failures ran on `origin/main` at `cb7021a24`, which is behind local
`main`. At HEAD, the docs, lint, conformance, proofs and website jobs are already fixed. The macOS
warm-start budget passed and failed on that same commit across nights, so it is the runner. The
supply-chain job was real: `ruzstd` and `rustls` advisories, fixed in `265d4b7ea`.

### Next

**Stage 3: the rule, the reference card and the feature proofs**, one file set: the rule fragment and its record, `crates/nvs-stdlib/src/response.rs`, the `sendFile` examples.

- [ ] The fragment is rewritten whole, with `If-Range` and a `**Memory:**` bullet (`rule:http-server/static-serving-is-one-policy`). `docs/rules/http-server/static-serving-is-one-policy.md:3`
- [ ] `guardedBy` names the five new tests (`rule:http-server/static-serving-is-one-policy`). `data/rules/http-server/static-serving-is-one-policy.json:9`
- [ ] The `sendFile` card says a browser can seek in a video and resume a download (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/response.rs:692`
- [ ] A fourth example: a download behind an access check (`rule:testing/feature-proofs`). `docs/examples/core/Response/sendFile/about.md:1`

### Backlog

- Stage 4, a stream with a declared length answers a range — this file's § *Stages*.
