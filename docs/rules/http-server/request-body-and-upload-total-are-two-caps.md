| Directive | Default | Ceiling | Bounds |
|---|---|---|---|
| `request_body` | `"8M"` | `"64M"` | **Bytes parsed into memory** — `body()`, a JSON or urlencoded body, a multipart form's buffered fields, a bare `readAll()`. |
| `upload_total` | `"256M"` | `"2G"` | **Total bytes of a streamed multipart body**, consumed parts and drained ones alike. |

Both are rows in the `[limits]`/`[limits.hard]` pair that `rule:config/three-changeability-classes` established — a `Runtime` default beside a `System` ceiling per `rule:config/ceilings-are-their-own-directives` — not a third instance of that pattern. `nvs info --config` prints both, and this table is the only place their meanings are defined.

The split follows from what the two measure. Content parsed into memory deserves the tighter number, because it is resident and O(in-flight); content streamed to disk deserves a ceiling shaped like what uploads are, because a phone photo is 5–12M and an 8M refusal is advice rather than protection. One number governing both would have to be the larger, and would then license a quarter-gigabyte JSON body parsed wholly into memory on the strength of an argument made about files. `rule:http-server/upload-total-is-enforced-on-the-wire` is where the second cap bites.

Both rows and both ceilings resolve, and a `[limits]` or `[app.limits]` value past its ceiling is
refused at boot whether or not `[limits.hard]` is written (`crates/nvs-config/src/app.rs`'s
`body_caps`).

**Not on disk.** Nothing reads the rows yet. The door still enforces a 256 MiB constant
(`crates/nvs-server/src/body.rs:60`), and the readers still hold a multipart field to an 8 MiB
constant (`crates/nvs-stdlib/src/request.rs:2832`).
