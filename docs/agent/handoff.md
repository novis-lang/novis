# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4, 5, 6, 7 and 8 are done; stage 9 (the record producers)
is next.** Both of stage 8's acceptance checks are green: the two named `nvs-stdlib` tests and the
`{through:}` case.

A `Core\Cli\Text` holds **runs** — `crates/nvs-stdlib/src/cli.rs:2033`'s `TEXT_RUNS` is the second
slot, two entries per run: the substituted body, and the `Cli\Style` it wears or `null`. Slot 0 still
holds a rendered `string`, because `nvs_runtime::value_to_string` renders that slot for `echo` and has
no stream to ask about — so it is **standard output's** rendering, and every other stream is rendered
from the runs by the member that knows which one it writes to (`Cli::write`). `depth_for` is the
per-stream answer: a depth that survived a redirected standard output was forced and reaches every
stream; otherwise a redirected stream gets none.

`Core\Cli\Text::text(): string` is the read half — the runs' bodies with the styling left out, and the
terminal carrier's alone, because that substitution is idempotent where HTML escaping is not. Both
`crates/nvs-stdlib/src/out.rs`'s gap 1 and `crates/nvs-stdlib/src/cli.rs`'s gap 2 are closed and
deleted. Nothing is blocked.

## Next group

**Stage 9: every record producer writes one shape** — one file set: `crates/nvs-runtime/src/floor.rs`,
`crates/nvs-runtime/src/throwable.rs` and `crates/nvs-render/src/`. `rule:errors/record-producers` is
the rule, and the goal's § *Standing decisions* fixes the rest: the JSON rendering writes an array of
`{function, file, line}` objects while the plaintext rendering keeps the `#0` form a person greps, and
two producers are not this stage's — the compiler diagnostic's (M10) and the `#[Test]` result's.

- [ ] **An uncaught throw's frames are a Sequence of Object nodes, and a `secret` in one is redacted**
      — `crates/nvs-runtime/src/floor.rs:94` is the producer, which carries them as one `backtrace`
      string today and says so in its own doc comment; `crates/nvs-runtime/src/throwable.rs:626` is
      where a frame is rendered. The named tests the acceptance asks for are
      `an_uncaught_throwables_frames_are_a_sequence_of_object_nodes` and
      `a_secret_property_on_an_object_in_a_frame_is_redacted_in_the_uncaught_record`, in `nvs-runtime`.
- [ ] **A third rendering of the record model, agreeing with the other two** —
      `crates/nvs-render/src/lib.rs:108` names the modules, and `crates/nvs-render/src/html.rs:61`
      holds `escape` and nothing else; `crates/nvs-render/src/plain.rs` and
      `crates/nvs-render/src/json.rs` are what it has to agree with on an elision and a redaction. The
      named tests are `the_html_rendering_elides_where_plain_and_json_elide` and
      `a_secret_renders_as_the_placeholder_in_all_three_renderings`, in `nvs-render`.

## Backlog
- `crates/nvs-stdlib/src/cli.rs` gap 1, a served request having no `arguments` — goal `unowned-closures`.
- `Core\Html\Markup` gets no read member: its escaping is not idempotent, and
  `crates/nvs-stdlib/src/cli.rs`'s `nvs_core_cli_text_text` is where that reasoning lives.
