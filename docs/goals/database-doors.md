---
looper: 1
title: "no caller text reaches DDL or error text unchecked, and a pooled connection carries nothing over"
summary: "Reached when every column default is written as a typed literal, a database error keeps the server's text out of its plain message, SQLite allows a PRAGMA by name only, and each driver's pool reset returns the session to its configured state, proved per driver against a live server."
context: [crates/nvs-db/src/conn.rs, docs/rules/security/db-pool-reset-is-a-boundary.md, docs/rules/core-classes/db-error.md]
stages:
  - id: 1
    title: a column default is a typed literal
    status: done
    context: [crates/nvs-db/src/schema.rs, crates/nvs-db/src/ddl.rs, crates/nvs-db/src/catalog.rs, crates/nvs-stdlib/src/db/schema.rs, docs/rules/core-classes/schema-vocabulary-is-closed.md]
  - id: 2
    title: "a database error keeps the server's text out of its message"
    status: done
    context: [crates/nvs-stdlib/src/db/bind.rs, crates/nvs-runtime/src/throwable.rs, crates/nvs-hir/src/errors.rs, crates/nvs-types/src/error_lib.rs, crates/nvs-stdlib/src/db/open.rs]
  - id: 3
    title: "the server's text rides the error only when no bound value is secret"
    status: done
    context: [crates/nvs-types/src/expr/quals.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/db/bind.rs, docs/rules/security/secret-sinks-refuse.md]
  - id: 4
    title: "SQLite allows a PRAGMA by name, and the reset sets back every one a request may write"
    status: done
    context: [crates/nvs-db/src/sqlite.rs]
  - id: 5
    title: "the PostgreSQL, MySQL and MariaDB resets return the session to its configured state"
    status: open
    context: [crates/nvs-db/src/mysql.rs, crates/nvs-db/src/maria.rs, crates/nvs-db/src/pg.rs, docs/rules/core-classes/db-safe-connection-defaults.md]
  - id: 6
    title: "one live test per driver reads the session state table, SQL Server included"
    status: open
    context: [crates/nvs-db/tests/pool_reuse.rs, crates/nvs-db/tests/handshake.rs, crates/nvs-db/src/tds/mod.rs, tools/nv/cmd/db-matrix.ts]
  - id: 7
    title: a MySQL text cell is copied once
    status: open
    context: [crates/nvs-db/src/mysql.rs, docs/rules/core-classes/db-crate-boundary.md, docs/rules/packaging/somebody-elses-specification-is-a-crate.md]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: defaults-are-literals
    stage: 1
    name: "every column default reaches DDL only as its dialect's literal, and a decimal or float default that is not a number is refused when the schema is read"
    run: [cargo, test, --lib, --, every_column_default_reaches_ddl_only_as_its_dialects_literal, a_decimal_default_that_is_not_a_number_is_refused_when_the_schema_is_read, a_float_default_that_is_not_finite_is_refused]
    timeout: 1800
    tests:
      - every_column_default_reaches_ddl_only_as_its_dialects_literal
      - a_decimal_default_that_is_not_a_number_is_refused_when_the_schema_is_read
      - a_float_default_that_is_not_finite_is_refused
  - id: message-holds-no-server-text
    stage: 2
    name: "a server error renders no server text, and a DbError carries it only as a property when the statement bound no value"
    run: [cargo, test, --lib, --, a_server_errors_rendering_holds_no_text_the_server_chose, a_db_error_message_holds_no_server_text, a_statement_with_a_bound_value_leaves_the_servers_text_out]
    timeout: 1800
    tests:
      - a_server_errors_rendering_holds_no_text_the_server_chose
      - a_db_error_message_holds_no_server_text
      - a_statement_with_a_bound_value_leaves_the_servers_text_out
  - id: server-text-is-tainted
    stage: 2
    name: "DbError's serverMessage and constraint are tainted strings, so neither is assignable to a plain string"
    run: [cargo, test, --test, db_error_text]
    timeout: 1800
    tests:
      - the_servers_text_on_a_db_error_is_a_tainted_string
  - id: secret-binds-are-known
    stage: 3
    name: "a statement call whose bound values hold no secret says so in its site word, and every other call leaves the server's text out"
    run: [cargo, test, --lib, --, a_statement_call_with_no_secret_bind_sets_the_plain_binds_bit, a_secret_anywhere_in_the_bound_values_clears_the_plain_binds_bit, a_secret_bound_value_leaves_the_servers_text_out_of_the_error, a_statement_reached_through_a_callable_leaves_the_servers_text_out]
    timeout: 1800
    tests:
      - a_statement_call_with_no_secret_bind_sets_the_plain_binds_bit
      - a_secret_anywhere_in_the_bound_values_clears_the_plain_binds_bit
      - a_secret_bound_value_leaves_the_servers_text_out_of_the_error
      - a_statement_reached_through_a_callable_leaves_the_servers_text_out
  - id: pragmas-by-name
    stage: 4
    name: "every PRAGMA the linked SQLite lists is allowed by name or refused, and the reset sets back every one a request may write"
    run: [cargo, test, --lib, --, every_pragma_sqlite_lists_is_allowed_by_name_or_refused, every_pragma_a_request_may_write_is_set_back_by_the_reset, a_process_wide_pragma_is_refused_on_every_connection, a_table_named_like_the_database_list_hides_nothing_from_the_reset, the_sqlite_reset_returns_the_last_inserted_rowid_to_zero]
    timeout: 1800
    tests:
      - every_pragma_sqlite_lists_is_allowed_by_name_or_refused
      - every_pragma_a_request_may_write_is_set_back_by_the_reset
      - a_process_wide_pragma_is_refused_on_every_connection
      - a_table_named_like_the_database_list_hides_nothing_from_the_reset
      - the_sqlite_reset_returns_the_last_inserted_rowid_to_zero
  - id: resets-restore-the-configured-session
    stage: 5
    name: "the MySQL and MariaDB reset selects the configured database and charset again, and PostgreSQL discards sequence state and pins its string syntax"
    run: [cargo, test, --lib, --, a_mysql_reset_selects_the_configured_database_and_charset_again, a_mysql_connection_with_no_database_is_closed_when_a_request_chose_one, the_postgres_reset_discards_sequence_state, the_postgres_startup_message_pins_standard_conforming_strings]
    timeout: 1800
    tests:
      - a_mysql_reset_selects_the_configured_database_and_charset_again
      - a_mysql_connection_with_no_database_is_closed_when_a_request_chose_one
      - the_postgres_reset_discards_sequence_state
      - the_postgres_startup_message_pins_standard_conforming_strings
  - id: every-driver-answers-for-session-state
    stage: 6
    name: "every driver names how each kind of session state is changed and read, or why it has none"
    run: [cargo, test, --lib, --, every_driver_answers_for_every_kind_of_session_state]
    timeout: 1800
    tests:
      - every_driver_answers_for_every_kind_of_session_state
  - id: one-copy-per-text-cell
    stage: 7
    name: "a MySQL text cell is read in place from its packet, and a row costs the same allocations at any width"
    run: [cargo, test, --lib, --, a_mysql_text_cell_is_read_in_place_from_its_packet, a_mysql_row_allocates_the_same_at_four_and_at_sixty_four_text_cells]
    timeout: 1800
    tests:
      - a_mysql_text_cell_is_read_in_place_from_its_packet
      - a_mysql_row_allocates_the_same_at_four_and_at_sixty_four_text_cells
---

# no caller text reaches DDL or error text unchecked, and a pooled connection carries nothing over

## Why

Four doors of the database layer let text or state cross a line the rules draw. Each has a recent fix
beside it that closed one instance and left a sibling.

**A `decimal` column default is raw DDL text.** `Core\Db\Schema::fromArray` turns every string into
`Node::Text` (`crates/nvs-stdlib/src/db/schema.rs:75`). `ColumnDefault::from_node` stores a
`["decimal" => <text>]` unread (`crates/nvs-db/src/schema.rs:1038`). `Column::default` checks only
that the kind fits the type (`crates/nvs-db/src/schema.rs:508`). The emitter writes the text as it is
(`crates/nvs-db/src/ddl.rs:604`). The catalog reader does check the same field
(`crates/nvs-db/src/catalog.rs:744`), so the rule exists and is missing on the program's door. A
program that builds a schema from data adds columns, and on SQL Server a second statement, with one
default. `applySafe` runs it, because the grade comes from the change and never from the SQL. A float
default that is `NaN` or infinite is written the same raw way (`crates/nvs-db/src/ddl.rs:603`).

**A server's sentence is a plain string.** Each driver copies the server's message into
`ServerError.message` (`crates/nvs-db/src/pg.rs:1510`, `crates/nvs-db/src/mysql.rs:1391`,
`crates/nvs-db/src/tds/stream.rs:640`, `crates/nvs-db/src/sqlite.rs:2112`). Its `Display` prints it
(`crates/nvs-db/src/conn.rs:740`), and `statement_failure` formats that into `DbError->message`
(`crates/nvs-stdlib/src/db/bind.rs:192`). MySQL, SQL Server and PostgreSQL put the offending bound
value in that sentence. So a bound `secret string` reaches the log or the error page, and a remote
client's `tainted` value comes back inside a plain `string`. `rule:core-classes/db-error` promises
neither happens. The same `Display` feeds every connect failure
(`crates/nvs-stdlib/src/db/open.rs:342`) and every other caller that prints a driver error.
PostgreSQL's severity and `SQLSTATE` fields are also server text of any length.

**SQLite session state is a list somebody wrote.** The reset replays eight pragmas
(`crates/nvs-db/src/sqlite.rs:852`), and the authorizer allows every action but `ATTACH` and `DETACH`
(`crates/nvs-db/src/sqlite.rs:881`). Any other `PRAGMA` a request runs stays on the pooled
connection: `locking_mode`, `case_sensitive_like`, `synchronous` and more. `hard_heap_limit`,
`soft_heap_limit` and `temp_store_directory` are process-wide, so one statement by a package with one
`db.connect` grant breaks every SQLite database in the process. The attached-database check reads
`pragma_database_list` by name (`crates/nvs-db/src/sqlite.rs:1601`), which a table of that name may
hide. c35823e811 and 4299f2bfe9 made the reset a fixed list.

**A pooled server connection keeps what a request chose.** ed8c68601d made the PostgreSQL reset drop
the role and left the siblings. The MySQL and MariaDB reset sends `COM_RESET_CONNECTION` and the time
zone only (`crates/nvs-db/src/mysql.rs:2191`). That command keeps the database a request chose with
`USE`, so the next request reads another tenant's tables. On MySQL it also returns the charset to
the server's global one, which breaks `rule:core-classes/db-safe-connection-defaults`. The PostgreSQL
reset has no `DISCARD SEQUENCES` (`crates/nvs-db/src/pg.rs:3541`), so `lastval()` returns the previous
request's row id. No test asks a live MariaDB about its reset at all.

Two smaller items ride along. PostgreSQL text defaults are quoted as if `standard_conforming_strings`
were on (`crates/nvs-db/src/ddl.rs:664`, `crates/nvs-db/src/sql.rs:141`), and the startup message
does not pin it (`crates/nvs-db/src/pg.rs:1302`). And `decode_row` asks `mysql_common` for an owned
value per cell (`crates/nvs-db/src/mysql.rs:3018`), which costs one allocation and one extra copy for
every text cell of every MySQL and MariaDB row.

## Standing decisions

**Every column default is written into DDL as a typed literal, and no text is written raw.**
`crate::ddl::literal` (`crates/nvs-db/src/ddl.rs:599`) is the one function that renders a default for
a dialect. `ColumnDefault::Decimal` holds a checked type whose only constructor accepts an optional
sign, digits and at most one point. `from_node` and `catalog::column_default` both build it, so the
two readers share one rule. `Column::default` refuses a float that is not finite. A refusal is a
`SchemaError` the program catches as `LogicError`. **What it spends:** one scan of the digits when a
schema is read, nothing on the request path. Safe default if the checked type proves awkward: one
`is_decimal_literal` function called from `Column::default`, which every reader passes.

**`tainted` text reaches DDL only inside a quoted literal or after a shape check.** `fromArray`
takes `array<mixed>`, and `mixed` carries no qualifier, so no checker rule can refuse a `tainted`
leaf there. The guard is the shape: a name passes `is_bare_identifier`, a type passes
`ScalarType::from_spelling`, a decimal passes its own check, and a text default is quoted.
**Declined:** a call-site refusal of a `tainted` argument. It would catch a typed string only, and
most schema data arrives as `mixed`.

**`ServerError`'s rendering holds no text the server chose.** `Display`
(`crates/nvs-db/src/conn.rs:739`) is the one place every caller's `{err}` passes. It prints the
backend, one of three severity words, the `SQLSTATE` when it is five ASCII letters or digits, the
driver code, and Novis's own sentence for the `DbErrorKind`. The server's sentence stays in
`ServerError.message`, and only `statement_failure` reads it. **What it spends:** two shape checks on
the error path. Usability: a message says less, and the kind's sentence replaces the server's.

**`Core\Db\DbError` gains `serverMessage`, a `?tainted string`, and `message` stays plain.** The
reviewer's design is used here in place of one message with two types. `message` is `Throwable`'s
property and has one type, so the server's text needs a property of its own. It is appended after
`sql`, so no compiled slot moves. `constraint` becomes `?tainted string` too, since PostgreSQL sends
it. **What it spends:** one more nullable slot on the boxed slice the error already carries, per
thrown error. Callers in the tree that assign `constraint` to a plain `string` are updated.

**The server's text is left out when a bound value may be `secret`.** `serverMessage` is `null`
then. `message` says the text was left out because the statement bound a `secret` value, and it
keeps the `SQLSTATE` and the driver code. A statement that bound no value at all always keeps the
text: DDL, `BEGIN` and `COMMIT` are such statements. The default is closed: the text rides only
when the call site proved that no bound value is `secret`.

**The proof travels in the query site word.** `secret` is erased before codegen, so the checker
decides. `reject_secret_into_container` (`crates/nvs-types/src/expr/quals.rs:1171`) already sees
every value written inside a call argument, with its own type. It notes a `secret` one there. A
`QUERY_SITE_MEMBERS` call (`crates/nvs-stdlib/src/registry.rs:3473`) whose arguments hold no note and
no `secret` type gets one bit set in its site constant, above the 32-bit id. A callable reference
passes the zero word, so its statements leave the text out. **What it spends:** one flag per call
while compiling, one bit in a constant that already exists, and one branch on the error path. No new
argument and no new helper. Safe default if the bit cannot share the constant: a flag column in
`QuerySites`, read by id, and the text is left out where a context has no table. **Declined:**
searching the server's text for the bound bytes, since servers cut and escape values. **Declined:**
typing the text `secret tainted string` always, since a syntax error would then need
`Core\Secret::reveal` to be read.

**An uncaught `DbError` still shows the server's text, and a failed connect logs it.** The uncaught
error report prints `serverMessage` after `message` when it is not `null`, through the renderer each
sink already uses for `tainted` text. A connect failure throws `IOError`, which has no such
property. It binds no value, so the server's sentence goes to one `Warn` log record, as
`trace_ungranted_record` writes one (`crates/nvs-stdlib/src/db/span.rs:178`). A log takes `tainted`
text freely (`rule:security/log-is-not-a-sink`). **What it spends:** one log record per failed
connect.

**SQLite allows a `PRAGMA` by name, and refuses every other one when the statement is prepared.**
This is chosen over closing a connection that wrote an unlisted pragma. Security and request
isolation: an allow list fails closed for a pragma a later SQLite adds, and a refused pragma never
runs, so no file lock or process limit is taken even inside one request. Latency and throughput: no
connection is ever reopened. Simplicity: one table. The table has three kinds of row. A *read* row
is allowed as it is: `table_info`, `table_xinfo`, `table_list`, `index_list`, `index_info`,
`index_xinfo`, `foreign_key_list`, `foreign_key_check`, `integrity_check`, `quick_check`,
`database_list`, `collation_list`, `function_list`, `module_list`, `pragma_list`, `compile_options`.
A *value* row is allowed with no argument only: `user_version`, `application_id`, `journal_mode`,
`page_size`, `page_count`, `freelist_count`, `data_version`, `encoding`. A *session* row may be
written, and it names the value the reset writes back: the eight of `SESSION_PRAGMAS`. The reset's
text is built from the session rows, so a pragma a request may write is always set back. A refusal is
a `DbError` of kind `Permission` that names the pragma. **What it spends:** one table lookup per
prepared `PRAGMA`. A cached statement pays nothing. Usability: a program cannot run `optimize`,
`wal_checkpoint`, `synchronous` or `cache_size`. A program that needs one gets a row, with its reset
value, in the same commit. **A process-wide pragma never gets a row:** `hard_heap_limit`,
`soft_heap_limit`, `temp_store_directory`, `data_store_directory`.

**The driver's own pragmas pass the same table.** The open's `journal_mode = WAL` runs before the
authorizer is installed. Every later statement of the driver's, the reset's included, is held to the
table. Safe default if a driver statement needs an unlisted pragma: a flag on the connection the
driver sets around its own statement, never a wider row.

**The SQLite reset also clears what no pragma holds.** The attached-database check runs the `PRAGMA
database_list` statement, which no table can shadow. `last_insert_rowid()` returns to zero. The reset
uses `rusqlite`'s own setter when the installed version has one. Otherwise it inserts rowid `0` into a
`temp` table it drops, and only when `Connection::last_insert_rowid()` is not zero. `nvs-db` forbids
`unsafe`, so the FFI call is not used. **Declined:** clearing `changes()` and `total_changes()`.
SQLite has no call for it short of closing, and each holds a row count and no data.

**Each pool reset returns the session to its configured state, and one table names that state.**
The kinds are the database, the role, the charset, the generated id and sequence state, and session
variables. `crates/nvs-db/src/reset.rs` holds one function from a `Driver` and a kind to the SQL
that changes it and the SQL that reads it, or to the reason the driver has none. Its `match` has no
wildcard, so a new driver or kind does not compile until it answers. The live test walks that
function. `rule:security/db-pool-reset-is-a-boundary` gains the default database, the charset and
the generated id in its property.

**MySQL and MariaDB keep `COM_RESET_CONNECTION`, then select the database and charset again.**
`reset_session` (`crates/nvs-db/src/mysql.rs:2191`) is the one function both drivers call. After the
reset's `OK` it sends `COM_INIT_DB` with the configured database. Then one `SET` assigns
`character_set_client`, `character_set_connection`, `character_set_results` and the time zone. A
connection configured with no database asks `SELECT DATABASE()` and is closed when the answer is not
`NULL`. **What it spends:** one more command per pooled release, and the charset rides the statement
the time zone already costs. Written in one send where the wire layer allows, it adds no round trip.
The safe default is one round trip more per release. Safe default if the live test shows a role
survives the reset: that server family resets with `COM_CHANGE_USER`, which costs one authentication
per release. **Declined:** parsing statements to refuse `USE`. A statement classifier is a second SQL
reader to keep right.

**PostgreSQL adds `DISCARD SEQUENCES` and pins `standard_conforming_strings = on` at startup.** The
discard is a ninth command in the pipelined batch (`crates/nvs-db/src/pg.rs:3541`), so it adds no
round trip. It needs PostgreSQL 9.4. The pin is one more startup pair
(`crates/nvs-db/src/pg.rs:1321`), about forty bytes per connection. A startup parameter is the
session's default, so `RESET ALL` returns to it. `Dialect::backslash_escapes` is then true by
construction, for the DDL quoting and for the placeholder rewriter. **Declined:** writing defaults as
`E'...'` literals. The rewriter reads a program's own literals by the same judgement and cannot
rewrite them.

**SQL Server keeps `sp_reset_connection`, and the live test decides what it adds.** That reset
returns the session to the login's database, and the driver already restores the isolation level
(`crates/nvs-db/src/tds/mod.rs:481`). Where the test shows a kind survives, the driver restores it
after the reset. Where no statement can, the connection is closed and not pooled.

**A MySQL text cell is copied once, from the packet into the Novis string.** `decode_row` reads the
binary row by hand into ranges over the packet: the null bitmap, length-encoded strings, fixed-width
numbers and the date and time layouts. `MySqlRow` keeps the packet. `mysql_common` stays for the
handshake, authentication, column definitions and the packet codec, which are per connection or per
statement (`rule:core-classes/db-crate-boundary`). **What it spends:** about 150 owned lines of
parser. It saves one allocation and one copy per text, bytes, decimal and JSON cell.
`rule:packaging/somebody-elses-specification-is-a-crate` asks for the argument in writing, and the
stage writes it into that rule's fragment. The guard counts, and never times.

**A test that needs a server follows the matrix convention.** It asks `nvs_db::matrix::endpoint()`
for its driver and returns at once when `NVS_DB_MATRIX_DRIVER` is unset, as
`crates/nvs-db/tests/pool_reuse.rs:86` does. `bun nv db-matrix --driver <name>` starts the server
from `tests/db/compose.yaml` and runs the `nvs-db` suite, which `SUITES` in
`tools/nv/cmd/db-matrix.ts:128` already lists. A skipped case proves nothing, so each front-matter
check is a lib test that needs no server, and stage 6 names the matrix runs it requires.

**A hostile server that streams discardable messages is not this goal's.** Goal `waits-are-bounded`
owns it.

**Lanes:** stage 1 and stage 4 each touch files no other stage does, and may run as side goals.
Stages 2 and 3 share `crates/nvs-stdlib/src/db/bind.rs` and run in order. Stages 2 and 5 share
`crates/nvs-db/src/conn.rs`, and stages 5, 6 and 7 share `crates/nvs-db/src/mysql.rs`, so 2, 3, 5,
6 and 7 run in that order.

## Stages

### 1. a column default is a typed literal

**Does:** make a decimal default a checked value and refuse a float that is not finite, at the one
function every schema reader passes. One test then walks every column type and default kind.

- [x] `ColumnDefault::Decimal` holds a checked type with one constructor: optional sign, digits, at
  most one point. `from_node` and `catalog::column_default` build it
  (`rule:core-classes/schema-vocabulary-is-closed`, `crates/nvs-db/src/schema.rs:377`,
  `crates/nvs-db/src/schema.rs:1038`, `crates/nvs-db/src/catalog.rs:744`).
- [x] `Column::default` refuses a float default that is not finite, with a `SchemaError` that names
  the column (`rule:core-classes/schema-vocabulary-is-closed`, `crates/nvs-db/src/schema.rs:508`,
  `crates/nvs-db/src/ddl.rs:603`).
- [x] `literal` writes the checked decimal's digits, and the callers that build a decimal default
  from a string are updated (`crates/nvs-db/src/ddl.rs:604`, `crates/nvs-db/src/ddl.rs:1366`,
  `crates/nvs-db/src/catalog.rs:1684`, `crates/nvs-db/src/catalog.rs:1759`).
- [x] `fromArray` throws `LogicError` for both refusals, and the message names the column and what a
  decimal default looks like (`crates/nvs-stdlib/src/db/schema.rs:75`,
  `crates/nvs-stdlib/src/db/schema.rs:205`). The message follows AGENTS.md § *Text an end user
  reads*.
- [x] Test `a_decimal_default_that_is_not_a_number_is_refused_when_the_schema_is_read` beside
  `from_node`. It passes a default that holds a comma, a parenthesis and a second column, and one
  that holds two points (`crates/nvs-db/src/schema.rs:1023`).
- [x] Test `a_float_default_that_is_not_finite_is_refused`, for `NaN` and both infinities
  (`crates/nvs-db/src/schema.rs:508`).
- [x] Test `every_column_default_reaches_ddl_only_as_its_dialects_literal` beside the round-trip
  test (`crates/nvs-db/src/catalog.rs:1720`). It takes every type `ScalarType::from_spelling`
  accepts and every `ColumnDefault` case by a `match` with no wildcard. For each pair that `fits`,
  on each dialect, it emits `CREATE TABLE`, the added column and the altered column
  (`crates/nvs-db/src/ddl.rs:430`, `crates/nvs-db/src/ddl.rs:869`, `crates/nvs-db/src/ddl.rs:898`).
  It fails naming the pair whose statement holds the default in any form but `literal`'s output.
  Text values include a quote, a backslash and a statement end.
- [x] An attack under `tests/hostile/core/Db-Schema/fromArray/`, at the next free number: a schema
  built from request data gives a decimal default that carries a second column. Every attempt
  should throw before any statement is sent. Copy the directives of
  `tests/hostile/core/Db-Schema/applySafe/01-applying-the-same-schema-without-end.nvs:1`.
- [x] `rule:core-classes/schema-vocabulary-is-closed` says a default is a typed literal, names the
  decimal shape and the finite float, and declines the call-site `tainted` refusal
  (`docs/rules/core-classes/schema-vocabulary-is-closed.md:1`).

### 2. a database error keeps the server's text out of its message

**Does:** stop `ServerError`'s rendering from printing server text, and carry that text on
`DbError` as a `tainted` property. Until stage 3, any statement that bound a value leaves it out.

**Split:** an `Explore` subagent lists two sets before the first edit, as `file.rs:NN` and one line
each. First, every site that renders an uncaught `Throwable`: the log record, the error page and
the terminal. Second, every test and `.nvst` case that pins a message holding a server's sentence.

- [x] `Display for ServerError` prints the backend, a severity from `ERROR`, `FATAL` and `PANIC`,
  the `SQLSTATE` only when it is five ASCII letters or digits, the driver code, and a fixed sentence
  for each `DbErrorKind` (`rule:core-classes/db-error`, `crates/nvs-db/src/conn.rs:739`,
  `crates/nvs-db/src/conn.rs:606`). The four constructors keep filling `message`
  (`crates/nvs-db/src/pg.rs:1520`, `crates/nvs-db/src/mysql.rs:1387`,
  `crates/nvs-db/src/tds/stream.rs:636`, `crates/nvs-db/src/sqlite.rs:2106`).
- [x] `DbError` gains `serverMessage` after `sql`: the property list, the slot constant in both
  crates, and its type `?tainted string`. `constraint` becomes `?tainted string`
  (`crates/nvs-hir/src/errors.rs:194`, `crates/nvs-runtime/src/throwable.rs:244`,
  `crates/nvs-types/src/error_lib.rs:173`, `crates/nvs-codegen/tests/throwing.rs:127`,
  `crates/nvs-stdlib/src/db/transaction.rs:1059`).
- [x] `statement_failure` takes whether the statement bound a value. It writes `serverMessage` from
  `ServerError.message` only when none was bound. Otherwise the message says the server's text was
  left out and why (`crates/nvs-stdlib/src/db/bind.rs:180`, `crates/nvs-stdlib/src/db/bind.rs:192`).
  Its callers in `execute.rs`, `stream.rs`, `transaction.rs`, `column.rs` and `schema.rs` pass it.
- [x] A failed connect writes the server's sentence to one `Warn` log record and throws its
  `IOError` without it (`crates/nvs-stdlib/src/db/open.rs:342`, `crates/nvs-stdlib/src/db/open.rs:404`,
  `crates/nvs-stdlib/src/db/open.rs:871`, `crates/nvs-stdlib/src/db/open.rs:1095`).
- [x] Each uncaught-error renderer the list names prints `serverMessage` after `message` when it is
  not `null`, through that sink's own renderer for `tainted` text.
- [x] Tests `a_server_errors_rendering_holds_no_text_the_server_chose` in `conn.rs`, and
  `a_db_error_message_holds_no_server_text` and
  `a_statement_with_a_bound_value_leaves_the_servers_text_out` in `bind.rs`. The first builds a
  `ServerError` whose message, severity and `SQLSTATE` each hold a marker, and finds none rendered.
- [x] Test `the_servers_text_on_a_db_error_is_a_tainted_string` in a new
  `crates/nvs-types/tests/db_error_text.rs:1`: `string $s = $e->serverMessage` and
  `string $c = $e->constraint` do not compile, and the `?tainted string` forms do.
- [x] Every test and case the list names is updated to the new message, never loosened to a prefix.
- [x] `rule:core-classes/db-error` lists `serverMessage`, says which text is plain and which is
  `tainted`, and says when the text is left out (`docs/rules/core-classes/db-error.md:1`).
- [x] `bun nv proofs --id 'Core\Db\DbError'` prints what the new property owes. Each `about.md`,
  example and reference card it names is written to AGENTS.md § *Text an end user reads*.

### 3. the server's text rides the error only when no bound value is secret

**Does:** let the checker prove that a statement call binds no `secret`, carry that proof in the
call's site word, and keep the server's text on the error for exactly those calls.

- [x] `reject_secret_into_container` notes a `secret` value written inside a call argument before it
  returns. The note is per call, and a nested call may only widen it
  (`rule:security/secret-sinks-refuse`, `crates/nvs-types/src/expr/quals.rs:1171`).
- [x] After a `QUERY_SITE_MEMBERS` call's arguments are checked, the checker records "plain binds"
  for the call's span when no note was made and no argument type contains `secret`
  (`crates/nvs-types/src/expr/calls.rs:786`, `crates/nvs-stdlib/src/registry.rs:3473`,
  `crates/nvs-types/src/expr_table.rs:2934`).
- [x] `QuerySiteConst` carries the recorded fact, and codegen sets one bit above the 32-bit id for
  it. The callable-reference thunk keeps the zero word (`crates/nvs-ir/src/lower/mod.rs:3100`,
  `crates/nvs-ir/src/ir.rs:1106`, `crates/nvs-codegen/src/emit.rs:925`,
  `crates/nvs-ir/src/lower/anon_fn.rs:863`).
- [x] Each statement helper masks the id for the trace and passes the bit to `statement_failure`,
  which keeps the server's text when the bit is set or no value was bound
  (`crates/nvs-stdlib/src/db/bind.rs:180`, `crates/nvs-runtime/src/ctx/trace.rs:387`).
- [x] Tests `a_statement_call_with_no_secret_bind_sets_the_plain_binds_bit` and
  `a_secret_anywhere_in_the_bound_values_clears_the_plain_binds_bit` in
  `crates/nvs-ir/src/lower/tests.rs:3999`. The second covers a variable, a property read, a
  concatenation, a nested literal for `executeMany`, and an `array<secret string>` argument.
- [x] Tests `a_secret_bound_value_leaves_the_servers_text_out_of_the_error` and
  `a_statement_reached_through_a_callable_leaves_the_servers_text_out` in
  `crates/nvs-stdlib/src/db/bind.rs:180`. Each asserts the message keeps the `SQLSTATE` and the
  driver code.
- [x] An attack under `tests/hostile/core/Db-Connection/query/`, at the next free number, on SQLite:
  a `secret` value is bound into a statement the database refuses. The error's `message` and
  `serverMessage` should not hold the value.
- [x] `rule:security/secret-sinks-refuse` and `rule:core-classes/db-error` say a bound `secret`
  removes the server's text, and name the cost (`docs/rules/security/secret-sinks-refuse.md:30`,
  `docs/rules/core-classes/db-error.md:11`).

### 4. SQLite allows a PRAGMA by name, and the reset sets back every one a request may write

**Does:** replace the fixed reset list with one table of pragma names that the authorizer and the
reset both read, and clear the two pieces of state no pragma holds.

**Split:** an `Explore` subagent lists every `PRAGMA` the tree's own Rust code sends outside tests,
as `file.rs:NN` and the pragma, before the first edit.

- [x] One table of pragma rows replaces `SESSION_PRAGMAS`: read rows, value rows and session rows
  with their reset value, as the standing decision lists them. The reset's text is built from the
  session rows (`rule:security/db-pool-reset-is-a-boundary`, `crates/nvs-db/src/sqlite.rs:852`,
  `crates/nvs-db/src/sqlite.rs:1617`).
- [x] `authorize` denies a `PRAGMA` action whose name has no row, a value row that carries an
  argument, and every process-wide pragma (`crates/nvs-db/src/sqlite.rs:881`). The refusal reaches a
  program as a `DbError` of kind `Permission` that names the pragma
  (`crates/nvs-db/src/sqlite.rs:2096`).
- [x] Each driver statement the list names passes the table, or runs before the authorizer is
  installed as the open's `journal_mode` does (`crates/nvs-db/src/sqlite.rs:816`).
- [x] `clean` asks `PRAGMA database_list` as a statement (`crates/nvs-db/src/sqlite.rs:1600`).
- [x] `clean` returns `last_insert_rowid()` to zero, as the standing decision says
  (`crates/nvs-db/src/sqlite.rs:1595`). Look the setter up in the installed `rusqlite`'s reference.
- [x] Test `every_pragma_sqlite_lists_is_allowed_by_name_or_refused`: it reads `PRAGMA pragma_list`
  from the linked SQLite, runs each name with and without an argument on a fresh connection, and
  fails naming one that ran without a row (`crates/nvs-db/src/sqlite.rs:881`).
- [x] Test `every_pragma_a_request_may_write_is_set_back_by_the_reset`: for each session row it
  writes another value, resets, and reads the row's reset value.
- [x] Tests `a_process_wide_pragma_is_refused_on_every_connection`,
  `a_table_named_like_the_database_list_hides_nothing_from_the_reset` and
  `the_sqlite_reset_returns_the_last_inserted_rowid_to_zero`.
- [x] An attack under `tests/hostile/core/Db-Connection/query/`, at the next free number: a request
  tries a process-wide pragma, a file lock pragma and a pragma that changes what `LIKE` matches.
  Each should throw, and a second database should still answer.
- [x] `rule:security/db-pool-reset-is-a-boundary` replaces its SQLite sentence about pragmas with
  the allow list, and says the reset clears the generated id
  (`docs/rules/security/db-pool-reset-is-a-boundary.md:23`).

### 5. the PostgreSQL, MySQL and MariaDB resets return the session to its configured state

**Does:** make the shared MySQL reset select the configured database and charset again, add sequence
state to the PostgreSQL reset, and pin PostgreSQL's string syntax at startup.

- [ ] `MySqlConn` and `MariaConn` keep the database they logged in with, and both pass it to
  `reset_session` (`rule:security/db-pool-reset-is-a-boundary`, `crates/nvs-db/src/mysql.rs:2152`,
  `crates/nvs-db/src/maria.rs:679`, `crates/nvs-db/src/conn.rs:853`).
- [ ] `reset_session` sends `COM_INIT_DB` with that database after the reset's `OK`, then one `SET`
  for the three charset variables and the time zone (`crates/nvs-db/src/mysql.rs:2191`,
  `crates/nvs-db/src/mysql.rs:1758`, `rule:core-classes/db-safe-connection-defaults`).
- [ ] A connection configured with no database asks `SELECT DATABASE()` in the reset and returns an
  error when the answer is not `NULL`, so the pool closes it (`crates/nvs-db/src/mysql.rs:2191`).
- [ ] `RESET_COMMANDS` gains `DISCARD SEQUENCES` (`crates/nvs-db/src/pg.rs:3541`).
- [ ] The startup message sends `standard_conforming_strings = on`, and the comment on
  `backslash_escapes` says the driver pins it (`crates/nvs-db/src/pg.rs:1321`,
  `crates/nvs-db/src/sql.rs:132`).
- [ ] Tests `a_mysql_reset_selects_the_configured_database_and_charset_again` and
  `a_mysql_connection_with_no_database_is_closed_when_a_request_chose_one`, over a scripted peer as
  the module's other reset tests are (`crates/nvs-db/src/mysql.rs:2191`).
- [ ] Tests `the_postgres_reset_discards_sequence_state` and
  `the_postgres_startup_message_pins_standard_conforming_strings`, over a scripted peer
  (`crates/nvs-db/src/pg.rs:3541`, `crates/nvs-db/src/pg.rs:1302`).
- [ ] `rule:security/db-pool-reset-is-a-boundary` names the default database, the charset and the
  generated id in its property, and the per-backend commands
  (`docs/rules/security/db-pool-reset-is-a-boundary.md:5`). `rule:core-classes/db-safe-connection-defaults`
  says the charset is set again after every reset and the string syntax is pinned
  (`docs/rules/core-classes/db-safe-connection-defaults.md:12`).

### 6. one live test per driver reads the session state table, SQL Server included

**Does:** write the table of session state, walk it against a live server for each driver, and fix
what each run shows.

- [ ] New `crates/nvs-db/src/reset.rs:1`: the kinds of session state, and one function from a
  `Driver` and a kind to its change SQL and read SQL, or to the reason it has none. No `match` in it
  has a wildcard (`rule:security/db-pool-reset-is-a-boundary`).
- [ ] Test `every_driver_answers_for_every_kind_of_session_state` in that file: it walks every
  `Driver` and kind and fails naming a pair with empty SQL and no reason.
- [ ] Test `a_reset_returns_every_kind_of_session_state_to_the_configured_one` in
  `crates/nvs-db/tests/pool_reuse.rs:359`, once per driver. It opens the leg's driver, reads each
  kind, changes it, releases through `pool::release`, draws again and reads the first value. It
  follows the file's skip rule (`crates/nvs-db/tests/pool_reuse.rs:86`).
- [ ] This stage requires these runs, once each: `bun nv db-matrix --driver postgres`,
  `bun nv db-matrix --driver mysql`, `bun nv db-matrix --driver mariadb`,
  `bun nv db-matrix --driver mssql` and `bun nv db-matrix --driver sqlite`. Prove each case asserted
  and did not skip: break its assertion once, see the matrix name it as failed, and revert
  (`docs/agent/playbook/running-things/python-tools-db-matrix-py-reports-ok-for-a-case-that-never.md:1`).
- [ ] Each kind a run shows surviving is fixed in that driver's reset, by the standing decisions:
  MySQL and MariaDB roles (`crates/nvs-db/src/mysql.rs:2191`), and SQL Server's kinds
  (`crates/nvs-db/src/tds/mod.rs:481`).
- [ ] The existing per-driver reset cases stay, and each names the table where it repeats a kind
  (`crates/nvs-db/tests/handshake.rs:1018`, `crates/nvs-db/tests/handshake.rs:1383`).
- [ ] `docs/agent/testing.md` has no words on a test that needs a database server. It gains a short
  section: the matrix convention of the standing decision, and that a new driver answers `reset.rs`
  before it is pooled (`docs/agent/testing.md:1`).

### 7. a MySQL text cell is copied once

**Does:** decode a binary row into ranges over its packet, so a text cell's bytes are copied once,
into the Novis string.

- [ ] `rule:packaging/somebody-elses-specification-is-a-crate` gains the argument for this one
  call: what is owned, why, and what stays borrowed
  (`docs/rules/packaging/somebody-elses-specification-is-a-crate.md:1`).
- [ ] `decode_row` reads the null bitmap, length-encoded strings, fixed-width integers and floats,
  and the binary date and time layouts by hand. `MySqlRow` keeps the packet and one range or number
  per cell (`rule:core-classes/db-crate-boundary`, `crates/nvs-db/src/mysql.rs:3005`,
  `crates/nvs-db/src/mysql.rs:2948`, `crates/nvs-db/src/mysql.rs:2982`).
- [ ] `scalar` borrows text and bytes from the packet, and the `uint` row keeps its unsigned read
  (`crates/nvs-db/src/mysql.rs:3339`, `crates/nvs-db/src/mysql.rs:3024`). MariaDB's row path and
  every caller of `MySqlRow::value` follow (`crates/nvs-db/src/direct.rs:257`).
- [ ] The existing `decode_row` tests pass unchanged in what they expect.
- [ ] Test `a_mysql_text_cell_is_read_in_place_from_its_packet`: every text and bytes scalar of a
  decoded row points inside that row's packet (`crates/nvs-db/src/mysql.rs:3005`).
- [ ] Test `a_mysql_row_allocates_the_same_at_four_and_at_sixty_four_text_cells`: it counts
  allocations while decoding each row and compares the two counts. It uses the tree's counter
  (`crates/nvs-runtime/src/budget.rs:1182`), or a test-only counter where that one is not live.
- [ ] `rule:core-classes/db-crate-boundary` says the row values are read here and why
  (`docs/rules/core-classes/db-crate-boundary.md:1`).

### 8. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 4 are done, and `secret-binds-are-known` and `pragmas-by-name` are green. Stage 5 is
next. The sweep items were stale a fifth time: no `env-secret-proofs.log` exists, and Miri run
`43201099245` ran on `578534cc7c`, which predates `7568aaa0fb`.

How stage 4 works. `PRAGMAS` (`crates/nvs-db/src/sqlite.rs`, beside `LOCK_WAIT`) is one table of
`Pragma::Read`, `Pragma::Value` and `Pragma::Session(reset value)` rows. `pragma_allowed` is what
`authorize` asks for an `AuthAction::Pragma`. `session_pragmas()` builds the text `open` and `clean`
run. A refusal writes the pragma's name to the thread-local `REFUSED_PRAGMA`, and `server_error`
takes it on an `SQLITE_AUTH` code, so the `Permission` error's server text names the pragma.
`clean` reads `PRAGMA database_list` as a statement, and when `last_insert_rowid()` is not zero it
inserts rowid 0 into a `temp` table it drops. `rusqlite` 0.41.1 has no setter.

**`SqliteTarget::setup` is new.** The session store (`crates/nvs-stdlib/src/session/sqlite.rs`)
sends `synchronous = NORMAL`, which has no row. Its `SETUP` runs before the authorizer is installed,
as the standing decision's safe default allows. A block sets `setup: &[]`.

The two new attacks are `tests/hostile/core/Db-Connection/query/05-...` and `06-...`, each with an
`[[app]]` block in `nvs.toml`. Both pass `bun nv proofs --id 'Core\Db\Connection::query' --run`.

### Next

Stage 5, in the order its items are written (`docs/goals/database-doors.md`, § 5).

- [ ] `crates/nvs-db/src/mysql.rs:2191` `reset_session`, `crates/nvs-db/src/mysql.rs:2152`,
  `crates/nvs-db/src/maria.rs:679`, `crates/nvs-db/src/conn.rs:853`: keep the login database on
  `MySqlConn` and `MariaConn`, then `COM_INIT_DB` and one `SET` for the charset and time zone after
  the reset's `OK`. With no database, ask `SELECT DATABASE()` and fail when it is not `NULL`.
- [ ] `crates/nvs-db/src/pg.rs:3541` `RESET_COMMANDS` gains `DISCARD SEQUENCES`, and
  `crates/nvs-db/src/pg.rs:1321` sends `standard_conforming_strings = on` at startup. Fix the comment
  on `backslash_escapes` (`crates/nvs-db/src/sql.rs:132`).
- [ ] `crates/nvs-db/src/mysql.rs:2191` and `crates/nvs-db/src/pg.rs:1302`: the four tests stage 5
  names, over the scripted peers the modules' reset tests already use.
- [ ] `docs/rules/security/db-pool-reset-is-a-boundary.md:5` and
  `docs/rules/core-classes/db-safe-connection-defaults.md:12`, then `bun nv rules --render`.

Expected red: any test that runs a `PRAGMA` with no row through `nvs_db::sqlite::open` now gets a
`Permission` error. The tree's own Rust and `.nvs` files send none outside the session store, but a
snapshot or fixture that pinned `pragma_database_list` in the reset's text would change. Snapshot
tests that print a query site may print ` plain` after the span (`crates/nvs-ir/src/print.rs`), and
any test pinning "bound a value." fails. `bun nv affected --run` was not run.

### Backlog

- `writable_schema` is a session row, as the standing decision lists it. A request can still turn it
  on and edit `sqlite_schema` inside its own request. Stage 8 should weigh dropping the row, which
  would refuse it outright and cost nothing the tree uses.
