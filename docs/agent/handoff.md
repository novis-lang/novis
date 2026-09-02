# Handoff

## State

**Stage 9's literal-query diagnostics are in**, all three of ADR 0067 § 10's clauses this tree can
make. `Core\Db\Connection`'s and `Core\Db\Transaction`'s `query`/`queryAs`/`execute` are six new
rows on ADR 0057 § 1's closed list (`crates/nvs-types/src/intrinsics.rs:122`), reaching
`check_sql` (`crates/nvs-types/src/intrinsics.rs:316`). `executeMany` is deliberately absent — its
second argument is a list of parameter *sets*.

**The refusal is `nvs_db::sql`'s own rewriter, not a second reader of the grammar.**
`nvs_stdlib::db::check_literal_query` and `check_single_statement`
(`crates/nvs-stdlib/src/db.rs:378`) run `rewrite` and `holds_a_second_statement`
(`crates/nvs-db/src/sql.rs:542`) and hand back their messages, so ADR 0057 § 4's "an earlier answer,
never a different one" is a property of asking the runtime's question rather than of two scanners
kept in step. **A refusal must hold on all four dialects** — a `?` inside backticks is a placeholder
on PostgreSQL and text on MySQL, and a call site does not name its driver — so the first acceptance
ends it. `nvs-stdlib`'s `mod db` is `pub` for those two, `cap`'s reason exactly.

No new diagnostic code: both bands are full, and none was needed. A second statement is
`E_INTRINSIC_LITERAL_MALFORMED` (the literal alone), the pairing refusals are
`E_FORMAT_TEMPLATE_MISMATCH`, whose doc in `crates/nvs-diagnostics/src/lib.rs:2380` now names both
grammars.

**§ 10's fourth clause — an unterminated string literal — is not made, and it is a doc
disagreement rather than a gap**: `nvs-db/src/sql.rs`'s module doc declines it in the other
direction ("malformed SQL is the server's diagnosis"). Known gap 5 of
`crates/nvs-types/src/intrinsics.rs` owns it; it needs a decision, not a scan.

**Stage 2's `local_infile_is_refused_and_no_file_is_sent` still fails acceptance and always will**
until MySQL's driver exists — unchanged, permanent, backlog and not work.

## Next group

**Stage 9's remaining two are one file set — `crates/nvs-stdlib/src/db.rs`,
`crates/nvs-types/src/intrinsics.rs` and `crates/nvs-types/tests/intrinsics.rs` — and the third is
`crates/nvs-db/src/pg.rs` alone, so take it last or in its own session.**

- [ ] **A literal `Db::open` host against the `db.open` grants** — ADR 0067 § 10's second sentence,
      as `an_open_host_matching_no_grant_is_a_diagnostic`. `open` has **no registry row yet** (it
      waits on a shape *parameter*), so the first question is whether a host is reachable from a
      call site at all: read `crates/nvs-stdlib/src/db.rs:471`'s `CLASS` and the gap above it before
      anything else. The grant is `Cap::DbOpen`, `crates/nvs-config/src/capability.rs:52`, and
      `crates/nvs-types/src/intrinsics.rs:122` is the table if it turns out to be an intrinsic row.
- [ ] **A tainted `Settings` host names `assertTrusted`** — ADR 0058's laundering, as
      `a_tainted_settings_host_is_a_diagnostic_naming_assert_trusted`, and the same `open` question
      gates it. The taint idiom is `crates/nvs-types/tests/intrinsics.rs:259`'s `as tainted string`.
- [ ] **The query span carries no parameter value** — the `-p nvs-db` half of stage 9, as
      `a_query_span_contains_no_parameter_value_anywhere` beside `crates/nvs-db/src/pg.rs:3386`'s
      test module.

## Backlog

- Stage 7's `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` — a second and third
  driver (`docs/agent/loop-goal.toml`, stage 7).
- Stage 2's `local_infile_is_refused_and_no_file_is_sent` — MySQL's driver, permanently
  (`docs/agent/loop-goal.toml`, stage 2).
- § 10's unterminated string literal — `crates/nvs-types/src/intrinsics.rs` known gap 5 owns the
  disagreement to settle.
- Stage 5's remaining three of seven (`docs/agent/loop-goal.toml`).
- `[context]` gaps this session paid for: `adrs` wants `0067` §§ 5 and 10 and `0057` § 1;
  `modules` wants `nvs-types/src/intrinsics.rs` and `nvs-stdlib/src/db.rs`.
