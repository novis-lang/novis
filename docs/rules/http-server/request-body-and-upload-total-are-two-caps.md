| Directive | Default | Ceiling | Bounds |
|---|---|---|---|
| `request_body` | `"8M"` | `"64M"` | **Bytes parsed into memory** — `body()`, a JSON or urlencoded body, a multipart form's buffered fields, a bare `readAll()`. |
| `upload_total` | `"256M"` | `"2G"` | **Total bytes of a streamed multipart body**, consumed parts and drained ones alike. |

Both are rows in the `[limits]`/`[limits.hard]` pair that `rule:config/three-changeability-classes` established — a `Runtime` default beside a `System` ceiling per `rule:config/ceilings-are-their-own-directives` — not a third instance of that pattern. `nvs config dump` prints both where the tree writes them, and this table is the only place their meanings are defined.

The split follows from what the two measure. Content parsed into memory deserves the tighter number, because it is resident and O(in-flight); content streamed to disk deserves a ceiling shaped like what uploads are, because a phone photo is 5–12M and an 8M refusal is advice rather than protection. One number governing both would have to be the larger, and would then license a quarter-gigabyte JSON body parsed wholly into memory on the strength of an argument made about files. `rule:http-server/upload-total-is-enforced-on-the-wire` is where the second cap bites.

Both ceilings are numbers of their own, so a `[limits]` or `[app.limits]` value past one is refused
at boot whether or not `[limits.hard]` is written (`crates/nvs-config/src/app.rs`'s `body_caps`).

**Where each is read.** The door applies the global `[limits] upload_total` to a declared length,
because it refuses before a mount is selected (`nvs_config::app::upload_total`). A streamed body is
counted against that value until its first reader claims it, and the claim sets the request's own
value in force, `[app.limits]` and `Core\Config::set` included (`RequestBody::allow_total`).
`request_body` is read off the request by every reader that parses into memory
(`Ctx::request_body_limit`). Neither reads "no cap" from a missing or malformed value: both fall back
to the shipped default.
