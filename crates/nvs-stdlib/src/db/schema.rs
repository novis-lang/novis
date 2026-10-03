//! `Core\Db\Schema` — `rule:core-classes/schema-is-a-value`'s canonical array form, as a value a program holds.
//!
//! [`nvs_db::schema`] already owns the vocabulary, the builders and the array
//! form's own ordered [`Node`] tree, and that crate is sans-io and holds no
//! Novis value. So this module is the one place a `Node` becomes an
//! [`nvs_runtime::NvsArray`] and back — its own module doc says so — and it
//! decides nothing about what a schema may say.
//!
//! # Decision: the instance holds its canonical array, and nothing else
//!
//! [`crate::instance`]'s first decision is that a `Core` instance is an
//! ordinary Novis object, so every slot must be a value Novis can already hold
//! and there is nowhere to keep an [`nvs_db::schema::Schema`] itself. That is
//! not a compromise here: § 1 makes the **array form canonical** — two schemas
//! are the same schema exactly when their array forms agree — so the array *is*
//! the value, and a `Schema` built from it is a derived thing any member that
//! wants one rebuilds in a few microseconds off a value already proved to
//! construct.
//!
//! The slot holds what [`nvs_db::schema::Schema::to_array`] emits rather than
//! what the caller wrote, which is what makes `toArray` canonical for every
//! spelling that reaches it: an input leaving `null` out, listing its tables in
//! any order, or leaving a table's `indexes` key off entirely comes back with
//! the same array an introspection of the applied database produces. That is
//! the property § 5's round trip is asserted over, and normalizing on the way
//! *in* means no member downstream has to remember to.
//!
//! **What it spends:** one array per schema value, holding one map per table
//! and one per column — a few kilobytes for a schema of any size a person
//! writes, charged to the request that built it and released with it.

use super::*;

/// How many arrays deep a schema array may nest before it is refused.
///
/// The form's own deepest nesting is six arrays — the root map, the `tables`
/// list, a table, its `columns` list, a column, and that column's one-key
/// `default` — so no schema anybody can write comes near this, and the margin
/// costs nothing. [`node_of`] and [`array_node`] call each other once per level,
/// and without a bound an array a program built in a loop spends the whole
/// stack: the process then ends with no diagnostic at all, which is the one
/// ending a refusal exists to replace. The names a refusal is reported at grow
/// with the depth too, so this caps what they can cost.
const DEPTH_LIMIT: usize = 16;

/// One Novis value as a node of the canonical form.
///
/// The vocabulary's own leaves and nothing else: a `decimal`, a `bytes`, an
/// object or a `null` inside a schema array is refused here rather than
/// silently rendered into something [`nvs_db::schema::Schema::from_array`]
/// would then reject under a worse message. `Tag::Decimal` is the one that
/// looks like an omission and is not — § 2's decimal default is *digits*, so
/// that a value written into a schema file and one written in source are the
/// same value.
fn node_of(value: Value, at: &str, depth: usize) -> Result<nvs_db::schema::Node, Fault> {
    use nvs_db::schema::Node;

    let refused = |holds: &str| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Db\\Schema::fromArray(): {at} holds {holds}. A schema array holds only \
                 strings, integers, floats, booleans and arrays of those — `rule:core-classes/schema-vocabulary-is-closed`'s \
                 vocabulary is closed and there is nowhere in it to put anything else"
            ),
        )
    };
    match value.tag() {
        Some(Tag::Bool) => Ok(Node::Bool(value.as_bool() == Some(true))),
        Some(Tag::Int | Tag::EnumInt) => Ok(Node::Int(value.as_int().unwrap_or(0))),
        Some(Tag::Uint | Tag::EnumUint) => Ok(Node::Uint(value.as_uint().unwrap_or(0))),
        Some(Tag::Float) => Ok(Node::Float(value.as_float().unwrap_or(f64::NAN))),
        // A name and a type spelling are both text, and both are compared byte
        // for byte further down — so a string that is not UTF-8 is refused here
        // rather than lossily rendered into a name nobody wrote.
        Some(Tag::Str) => value
            .as_text()
            .map(|text| Node::Text(text.to_owned()))
            .ok_or_else(|| refused("a string that is not UTF-8")),
        Some(Tag::Array) => array_node(value, at, depth),
        Some(Tag::Decimal) => Err(refused(
            "a `decimal`, where § 2's exact default is written as its digits in a string",
        )),
        Some(Tag::Bytes) => Err(refused("`bytes`")),
        Some(Tag::Object) => Err(refused("an object")),
        None | Some(Tag::Null | Tag::Unset) => Err(refused(
            "`null`, where a key a schema does not set is left out instead",
        )),
    }
}

/// An array as either arm of the form: a list where every position from zero is
/// there, and an ordered map otherwise.
///
/// One `NvsArray` is both of Novis's shapes (`rule:types/arrays`), so the two are told apart exactly as [`crate::json`]'s decode tells
/// them apart — by asking for the positions. `["tables" => …]` has a count of
/// one and no index 0, and `[["name" => …]]` has both.
fn array_node(value: Value, at: &str, depth: usize) -> Result<nvs_db::schema::Node, Fault> {
    use nvs_db::schema::Node;

    if depth >= DEPTH_LIMIT {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Db\\Schema::fromArray(): {at} nests more than {DEPTH_LIMIT} arrays deep. A \
                 schema array nests six at the most, so nothing this deep describes a schema"
            ),
        ));
    }
    let Some(ptr) = value.array_ptr() else {
        return Err(Fault::fatal(format!(
            "Core\\Db\\Schema::fromArray() found tag {} behind {:?} at {at}",
            value.tag_byte(),
            Tag::Array
        )));
    };
    let source = crate::arr::borrowed(ptr);
    let positioned = (0..source.count())
        .all(|index| i64::try_from(index).is_ok_and(|at| source.get_index(at).is_some()));
    if positioned {
        let mut items = Vec::with_capacity(source.count());
        for index in 0..source.count() {
            let element = i64::try_from(index)
                .ok()
                .and_then(|position| source.get_index(position))
                .expect("every position is there, which is what `positioned` just asked");
            items.push(node_of(element, &format!("{at}[{index}]"), depth + 1)?);
        }
        return Ok(Node::List(items));
    }
    // Insertion order on both walks — `keys` is the array's own order and so is
    // the slot cursor — which is what makes zipping them the pairing rather
    // than a lookup per key. Key order is data here (§ 1), so nothing sorts.
    let keys = source.keys();
    let mut pairs = Vec::with_capacity(keys.len());
    let mut from = 0usize;
    for key in keys {
        let slot = source
            .next_slot(from)
            .expect("one live slot per key the array just listed");
        from = slot + 1;
        let element = source
            .value_at(slot)
            .expect("next_slot only names live entries");
        let key = String::from_utf8_lossy(&key).into_owned();
        let node = node_of(element, &format!("{at}[\"{key}\"]"), depth + 1)?;
        pairs.push((key, node));
    }
    Ok(Node::Map(pairs))
}

/// A node of the canonical form as the Novis value that spells it.
///
/// The total inverse of [`node_of`] over what a schema can hold: a map is a
/// string-keyed array in the order the form states, a list is a packed one, and
/// every leaf is the tag its arm names. Nothing here can fail — the form is
/// built by [`nvs_db::schema`] out of a value that already constructed.
fn value_of(node: &nvs_db::schema::Node) -> Value {
    use nvs_db::schema::Node;

    match node {
        Node::Text(text) => Value::str(NvsStr::new(text.as_bytes())),
        Node::Int(number) => Value::int(*number),
        Node::Uint(number) => Value::uint(*number),
        Node::Float(number) => Value::float(*number),
        Node::Bool(flag) => Value::bool(*flag),
        Node::List(items) => {
            let mut array = NvsArray::new();
            for item in items {
                array.append(value_of(item));
            }
            Value::array(array)
        }
        Node::Map(pairs) => {
            let mut array = NvsArray::new();
            for (key, item) in pairs {
                array.set(NvsStr::new(key.as_bytes()), value_of(item));
            }
            Value::array(array)
        }
    }
}

/// What a refused construction throws.
///
/// `LogicError`, not `RuntimeError`: every [`nvs_db::schema::SchemaError`] is a
/// schema no backend would accept or two would accept differently, and which of
/// those it is does not depend on anything outside the program — the array is
/// one the program wrote or one it read out of its own file. The facts are the
/// driver crate's, which holds no Novis-facing name (`rule:core-classes/db-crate-boundary`), and the
/// call is named here.
fn refused(member: &str, why: &nvs_db::schema::SchemaError) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!("Core\\Db\\Schema::{member}(): {why}"),
    )
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Schema::fromArray(array<mixed> $array): Db\Schema` — `rule:core-classes/schema-is-a-value`'s serialized spelling, and the door every schema value comes through.
    ///
    /// The array is validated by the builders themselves —
    /// [`nvs_db::schema::Schema::from_array`] reads the form and hands it to
    /// them — so a file cannot say anything a program could not have built, and
    /// this member has no rule of its own to disagree with them about.
    fn nvs_core_db_schema_from_array(_ctx, args: [1]) {
        let node = node_of(args[0], "the array", 0)?;
        let schema = nvs_db::schema::Schema::from_array(&node)
            .map_err(|why| refused("fromArray", &why))?;
        Ok(crate::instance::build(&SCHEMA, [value_of(&schema.to_array())]))
    }
}

nvs_runtime::nvs_helper! {
    /// `$schema->toArray(): array<mixed>` — the canonical form, which is what
    /// `Core\Json::encode` or a derived codec writes to a file.
    ///
    /// The slot already holds it, normalized when the value was built, so this
    /// is one retained reference rather than a second array: two calls answer
    /// arrays that compare equal because they are the same array.
    fn nvs_core_db_schema_to_array(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &SCHEMA, "toArray")?;
        Ok(owned(crate::instance::slot(receiver, SCHEMA_ARRAY_AT)))
    }
}

/// `Core\Db\Schema::planAgainst`, as a refusal names it.
const PLAN_AGAINST: &str = r"Core\Db\Schema::planAgainst";

/// `Core\Db\Schema::applySafe`, as a refusal names it.
const APPLY_SAFE: &str = r"Core\Db\Schema::applySafe";

/// `Core\Db\Schema::applyIncludingRisky`, as a refusal names it.
const APPLY_RISKY: &str = r"Core\Db\Schema::applyIncludingRisky";

/// The receiver's own schema, rebuilt out of the array slot.
///
/// The slot was normalized by [`nvs_db::schema::Schema::to_array`] when the
/// value was built, so this cannot fail on a value that exists — the module doc
/// above is where that decision lives, and rebuilding here is what it costs.
/// The refusal is kept rather than made a `fatal` because the alternative is a
/// panic-shaped answer to a condition a future `from_array` rule could create.
fn schema_of(receiver: Value, member: &str) -> Result<nvs_db::schema::Schema, Fault> {
    let node = node_of(
        crate::instance::slot(
            crate::instance::receiver(receiver, &SCHEMA, member)?,
            SCHEMA_ARRAY_AT,
        ),
        "the schema",
        0,
    )?;
    nvs_db::schema::Schema::from_array(&node).map_err(|why| refused(member, &why))
}

/// One catalog read, as the rows `Core\Db::query` would have answered.
///
/// **The whole of `rule:core-classes/schema-introspection`'s introspection is two ordinary statements**, so
/// this reuses the five arms every other statement in this crate goes through
/// rather than growing a sixth path: [`nvs_db::catalog::query`] is the text,
/// the driver decodes it, and what comes back is one string-keyed array per row
/// under the names [`nvs_db::catalog::Read::row`] fixes. There is no parameter
/// on either query — that module's own doc says why the catalog reads bind
/// nothing — so the binds are empty on every driver and [`Binds::sqlite`]
/// answers the empty vector for the one arm that reads them owned.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused, and the decode
/// refusals of whichever driver answered.
fn catalog_rows(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    block: Value,
    sql: &str,
    named: &str,
) -> Result<Value, Fault> {
    let statement = Statement {
        key,
        block,
        sql: sql.to_owned(),
        binds: Binds::Wire(Vec::new()),
    };
    let sending: Vec<Option<&[u8]>> = Vec::new();
    let watch = QueryWatch::of(ctx, &statement.block);
    let (answered, taken) = match bound_connection(ctx, key, named, None)? {
        nvs_db::Connection::Postgres(postgres) => {
            postgres_rows(postgres, &statement, &sending, Some(sql), watch, named)?
        }
        nvs_db::Connection::MySql(mysql) => mysql_rows(
            Framed::MySql(mysql),
            &statement,
            &sending,
            Some(sql),
            watch,
            named,
        )?,
        nvs_db::Connection::MariaDb(maria) => mysql_rows(
            Framed::MariaDb(maria),
            &statement,
            &sending,
            Some(sql),
            watch,
            named,
        )?,
        nvs_db::Connection::SqlServer(tds) => {
            tds_rows(tds, &statement, &sending, Some(sql), watch, named)?
        }
        nvs_db::Connection::Sqlite(sqlite) => {
            sqlite_rows(sqlite, &statement, Some(sql), watch, named)?
        }
    };
    watch.file(ctx, taken);
    Ok(Value::array(answered.rows))
}

/// One column of a catalog row as text, where the row has it.
///
/// **By the alias, not by the server's own name.**
/// [`nvs_db::catalog::Read::row`] owns why every catalog statement aliases its
/// select list: SQLite's column read selects `m.name` beside `p.name`, and the
/// row this crate builds is keyed by the described name, so the two would
/// collapse into one entry and shift every column after them — against an empty
/// database, where there are no rows, invisibly.
///
/// A server answers `ordinal` as an integer on one driver and as a decimal
/// string on another, and `default` as whichever type the column it defaults
/// has — so the three readers below are tolerant on purpose. Anything with no
/// textual reading at all is absence, which is what
/// [`nvs_db::catalog::column_default`] already answers `None` for.
fn text_at(row: &NvsArray, key: &[u8]) -> Option<String> {
    let value = row.get(key)?;
    match value.tag() {
        Some(Tag::Str) => value.as_text().map(str::to_owned),
        Some(Tag::Int) => value.as_int().map(|number| number.to_string()),
        Some(Tag::Uint) => value.as_uint().map(|number| number.to_string()),
        _ => None,
    }
}

/// One column of a catalog row as an integer — the two `ordinal` positions.
fn int_at(row: &NvsArray, key: &[u8]) -> Option<i64> {
    let value = row.get(key)?;
    match value.tag() {
        Some(Tag::Int) => value.as_int(),
        Some(Tag::Uint) => value
            .as_uint()
            .and_then(|number| i64::try_from(number).ok()),
        Some(Tag::Str) => value.as_text().and_then(|text| text.trim().parse().ok()),
        _ => None,
    }
}

/// One column of a catalog row as a flag — `nullable`, `identity`, `unique` and
/// `primary`.
///
/// The five servers spell a boolean five ways, and each query already narrows
/// it as far as its dialect can: PostgreSQL answers a `bool`, MySQL and SQL
/// Server a `0`/`1`, and SQLite whichever of those `pragma` produced. An
/// unreadable value is `false`, which **over**-reports a difference rather than
/// under-reporting one — the direction [`nvs_db::catalog::assemble`]'s own doc
/// argues for, since a plan gaining a step is visible and a plan losing one is
/// not.
fn flag_at(row: &NvsArray, key: &[u8]) -> bool {
    let Some(value) = row.get(key) else {
        return false;
    };
    match value.tag() {
        Some(Tag::Bool) => value.as_bool() == Some(true),
        Some(Tag::Int) => value.as_int().is_some_and(|number| number != 0),
        Some(Tag::Uint) => value.as_uint().is_some_and(|number| number != 0),
        Some(Tag::Str) => value
            .as_text()
            .is_some_and(|text| matches!(text.trim(), "1" | "t" | "true" | "TRUE" | "YES" | "y")),
        _ => false,
    }
}

/// Every row of one catalog read, converted by `of` and collected.
///
/// The rows are released here whatever `of` answered, because the array is this
/// function's own: [`catalog_rows`] hands over the one reference the driver
/// built, and nothing above holds a second.
fn read_rows<T>(
    rows: Value,
    named: &str,
    of: impl Fn(&NvsArray) -> Option<T>,
) -> Result<Vec<T>, Fault> {
    let collected = collect_rows(rows, named, of);
    #[expect(
        unsafe_code,
        reason = "the driver transferred the rows it built, and nothing above holds a second \
                  reference to them"
    )]
    unsafe {
        rows.release();
    }
    collected
}

/// [`read_rows`] without the release, so that one `?` cannot skip it.
fn collect_rows<T>(
    rows: Value,
    named: &str,
    of: impl Fn(&NvsArray) -> Option<T>,
) -> Result<Vec<T>, Fault> {
    // Unreachable from source: every arm of `catalog_rows` answers the array a
    // driver built its rows into, and no program can name this value at all.
    let Some(ptr) = rows.array_ptr() else {
        return Err(Fault::fatal(format!(
            "{named}: the catalog read answered tag {} where its rows should be",
            rows.tag_byte()
        )));
    };
    let outer = crate::arr::borrowed(ptr);
    let mut out = Vec::with_capacity(outer.count());
    for index in 0..outer.count() {
        let Some(row) = i64::try_from(index)
            .ok()
            .and_then(|position| outer.get_index(position))
        else {
            continue;
        };
        // Unreachable from source: a row is the string-keyed array
        // `Core\Db\Row` is built over, for the reason above.
        let Some(held) = row.array_ptr() else {
            return Err(Fault::fatal(format!(
                "{named}: the catalog read answered tag {} where row {index} should be",
                row.tag_byte()
            )));
        };
        let columns = crate::arr::borrowed(held);
        out.push(of(&columns).ok_or_else(|| {
            Fault::thrown(format!(
                "{named}: the server's catalog answered a row this read cannot use — row {index} \
                 is missing one of the columns the query asked it for, which means the catalog \
                 query and this reader have stopped agreeing about a driver"
            ))
        })?);
    }
    Ok(out)
}

/// `rule:core-classes/schema-introspection`'s live introspection: the database behind `key`, as the schema
/// value the diff compares against.
///
/// Two reads and no parser, which is § 4's whole rule — the catalog is
/// structured tables, and [`nvs_db::catalog::assemble`] is the one place rows
/// become a value, through the same builders a declared schema goes through.
///
/// # Errors
///
/// [`catalog_rows`]' refusals, and a thrown `LogicError` for a database holding
/// something the vocabulary cannot name.
fn introspected(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    block: Value,
    named: &str,
) -> Result<nvs_db::schema::Schema, Fault> {
    let dialect = nvs_db::Dialect::of(filed_connection(ctx, key, named)?.driver());
    let columns = catalog_rows(
        ctx,
        key,
        block,
        nvs_db::catalog::query(nvs_db::catalog::Read::Columns, dialect),
        named,
    )?;
    let columns = read_rows(columns, named, |row| {
        Some(nvs_db::catalog::ColumnRow {
            table: text_at(row, b"nvs_table")?,
            column: text_at(row, b"nvs_column")?,
            ordinal: int_at(row, b"nvs_ordinal")?,
            ty: text_at(row, b"nvs_type")?,
            nullable: flag_at(row, b"nvs_nullable"),
            default: text_at(row, b"nvs_default"),
            identity: flag_at(row, b"nvs_identity"),
        })
    })?;
    let indexes = catalog_rows(
        ctx,
        key,
        block,
        nvs_db::catalog::query(nvs_db::catalog::Read::Indexes, dialect),
        named,
    )?;
    let indexes = read_rows(indexes, named, |row| {
        Some(nvs_db::catalog::IndexRow {
            table: text_at(row, b"nvs_table")?,
            index: text_at(row, b"nvs_index")?,
            column: text_at(row, b"nvs_column")?,
            ordinal: int_at(row, b"nvs_ordinal")?,
            unique: flag_at(row, b"nvs_unique"),
            primary: flag_at(row, b"nvs_primary"),
            filter: text_at(row, b"nvs_filter"),
        })
    })?;
    nvs_db::catalog::assemble(&columns, &indexes, dialect).map_err(|why| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{named}: this database holds something `rule:core-classes/schema-vocabulary-is-closed`'s vocabulary cannot name, so \
                 no plan against it would be total: {why}"
            ),
        )
    })
}

/// § 9's plan: the receiver's schema against the database `$connection` reaches.
///
/// **An ordinary read.** The catalog queries go out under the `db.connect` the
/// program already holds, and nothing here asks for a grant — which is the
/// whole of that section's first paragraph and what
/// `plan_against_needs_only_the_db_connect_a_program_already_holds` asserts.
///
/// # Errors
///
/// [`schema_of`]'s and [`introspected`]'s.
fn planned(ctx: &mut nvs_runtime::Ctx, args: &[Value], named: &str) -> Result<nvs_db::Plan, Fault> {
    let want = schema_of(args[0], named)?;
    let (key, block) = handle_of(args[1], named)?;
    let conn = filed_connection(ctx, key, named)?;
    let dialect = nvs_db::Dialect::of(conn.driver());
    let server = nvs_db::ddl::Server::of(conn);
    let have = introspected(ctx, key, block, named)?;
    Ok(nvs_db::diff_on(&want, &have, dialect, server))
}

/// The `[db.<name>]` block a `db.schema` grant is asked about.
///
/// § 9 makes this capability name **connection blocks**, like `db.connect` and
/// unlike `db.open` — so a connection that has no block cannot be the subject
/// of a grant, and the honest answer is to say so rather than to invent a name
/// for it. `Core\Db::open`'s connections are the ones with no block: their
/// endpoint came out of the program rather than out of root-owned
/// configuration, which is exactly the difference § 3 of
/// `rule:core-classes/db-one-api` splits the two grants on.
///
/// # Errors
///
/// A thrown `LogicError` for a connection `Core\Db::open` produced.
fn granted_block<'a>(block: &'a Value, named: &str) -> Result<&'a str, Fault> {
    block.as_text().ok_or_else(|| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{named}: the `db.schema` capability names `[db.<name>]` blocks, and this \
                 connection was opened by `Core\\Db::open` and has no block for an operator to \
                 have granted. Name the connection in `nvs.toml` and open it with \
                 `Core\\Db::connect` to apply a schema to it"
            ),
        )
    })
}

/// Everything `applySafe` and `applyIncludingRisky` share: § 9's grant, the
/// plan, and running what the plan says may run.
///
/// **The grant is asked first, before a single catalog query goes out.** That
/// is `Core\Db::open`'s ordering and for its reason — a refusal a program is
/// going to get should not cost a round trip — and it is also what makes the
/// refusal assertable without a server, which
/// `applying_without_the_db_schema_capability_throws_naming_it` is.
///
/// `refuse` is the difference between the two members and the only one: § 9
/// names them for what they risk at the call site, so one of them declines a
/// plan the other runs and neither decides anything else differently.
///
/// # Errors
///
/// The `db.schema` refusal, [`granted_block`]'s throw, [`planned`]'s, whatever
/// `refuse` answers, and [`statement_failure`] for a statement the server would
/// not run.
fn applied(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    named: &str,
    refuse: impl Fn(&nvs_db::Plan) -> Option<Fault>,
) -> Result<Value, Fault> {
    let (key, block) = handle_of(args[1], named)?;
    let name = granted_block(&block, named)?.to_owned();
    nvs_runtime::capability::require(
        ctx,
        nvs_config::Cap::DbSchema,
        nvs_config::capability::Scope::Name(&name),
        named,
    )?;
    let plan = planned(ctx, args, named)?;
    if let Some(refused) = refuse(&plan) {
        return Err(refused);
    }
    // § 7: a report is carried and never run, whichever entry point was
    // written. `applyIncludingRisky` says the caller accepts a lock or a
    // rewrite, and says nothing at all about a drop.
    for step in plan.runnable() {
        for sql in step.sql() {
            run_ddl(ctx, key, block, sql, named)?;
        }
    }
    Ok(Value::null())
}

/// One statement of a step, sent as [`nvs_core_db_connection_execute`] sends a
/// write.
///
/// One statement per call rather than one step per call, because `rule:core-classes/db-statement-members`
/// has no multi-statement form on any driver and SQLite's rebuild is four
/// statements — [`mod@super::plan`]'s doc owns why `sql()` joins them for a
/// reader and this does not.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused.
fn run_ddl(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    block: Value,
    sql: &str,
    named: &str,
) -> Result<(), Fault> {
    let statement = Statement {
        key,
        block,
        sql: sql.to_owned(),
        binds: Binds::Wire(Vec::new()),
    };
    let sending: Vec<Option<&[u8]>> = Vec::new();
    let watch = QueryWatch::of(ctx, &statement.block);
    let (_, taken) = match bound_connection(ctx, key, named, None)? {
        nvs_db::Connection::Postgres(postgres) => {
            postgres_write(postgres, &statement, &sending, Some(sql), watch, named)?
        }
        nvs_db::Connection::MySql(mysql) => mysql_write(
            Framed::MySql(mysql),
            &statement,
            &sending,
            Some(sql),
            watch,
            named,
        )?,
        nvs_db::Connection::MariaDb(maria) => mysql_write(
            Framed::MariaDb(maria),
            &statement,
            &sending,
            Some(sql),
            watch,
            named,
        )?,
        nvs_db::Connection::SqlServer(tds) => {
            tds_write(tds, &statement, &sending, Some(sql), watch, named)?
        }
        nvs_db::Connection::Sqlite(sqlite) => {
            sqlite_write(sqlite, &statement, Some(sql), watch, named)?
        }
    };
    watch.file(ctx, taken);
    Ok(())
}

/// `applySafe`'s refusal — § 9's "throws naming the first step that is not
/// `Safe`".
///
/// Reads [`nvs_db::Plan::first_refused`], which is the grades of the steps that
/// would **run**: every plan against a shared database carries `Destructive`
/// reports it was never going to touch, and a rule reading those too would
/// refuse every plan ever computed. `crate::plan`'s module doc in `nvs-db` is
/// the home of that reasoning.
///
/// A free function rather than a closure written inline because it is the one
/// thing `applySafe` decides, and `apply_safe_refuses_a_plan_holding_a_step_that_is_not_safe`
/// asks it of a plan built with no server in front of it.
pub(super) fn unsafe_step(plan: &nvs_db::Plan) -> Option<Fault> {
    let step = plan.first_refused()?;
    Some(Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            // The reason is prose an operator reads and ends with its own full
            // stop, which `ddl`'s sweep over every dialect holds it to, so this
            // sentence adds none of its own.
            "{APPLY_SAFE}: the plan holds a step that is not `Safe`, and this entry point runs \
             none of it — `{}` is {} because {} `applyIncludingRisky` runs the same plan and \
             says so where it is written",
            step.change(),
            step.grade(),
            step.reason()
        ),
    ))
}

nvs_runtime::nvs_helper! {
    /// `$schema->planAgainst(Db\Connection $connection): Db\Plan` — `rule:core-classes/schema-apply-capability`'s read.
    ///
    /// **Planning is not privileged.** It issues § 4's two catalog queries
    /// through the connection a program already holds, under the `db.connect`
    /// it already has, and asks for nothing else — which is why this member has
    /// a `None` row in `crate::registry::CAPABILITIES` beside the two that ask
    /// for `db.schema`.
    ///
    /// **What it spends:** two statements and their rows, plus one object per
    /// step of the answer. The rows are released as they are converted, so the
    /// plan is what survives the call and the catalog is not.
    fn nvs_core_db_schema_plan_against(ctx, args: [2]) {
        let plan = planned(ctx, args, PLAN_AGAINST)?;
        Ok(plan_value(&plan))
    }
}

nvs_runtime::nvs_helper! {
    /// `$schema->applySafe(Db\Connection $connection): void` — `rule:core-classes/schema-apply-capability`'s
    /// first entry point.
    ///
    /// Refuses the **whole** plan if any step it would run is not `Safe`, and
    /// names the first one. All-or-nothing rather than "run the safe prefix":
    /// a partial convergence is a database in a shape nobody declared, and the
    /// caller who wanted the prefix can plan, read the grades and write it.
    fn nvs_core_db_schema_apply_safe(ctx, args: [2]) {
        applied(ctx, args, APPLY_SAFE, unsafe_step)
    }
}

nvs_runtime::nvs_helper! {
    /// `$schema->applyIncludingRisky(Db\Connection $connection): void` — ADR
    /// 0145 § 9's second entry point.
    ///
    /// **The name is the documentation.** § 9's whole argument for two members
    /// rather than one member with a flag is that a reviewer reading the call
    /// site sees the claim being made; a `{risky: true}` option is the same
    /// decision spelled where nobody greps.
    ///
    /// It still runs no report — § 7's absence-never-destroys is not what
    /// "risky" opts into, and [`applied`] says so at the loop.
    fn nvs_core_db_schema_apply_including_risky(ctx, args: [2]) {
        applied(ctx, args, APPLY_RISKY, |_| None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use nvs_runtime::{Ctx, OutputSink};

    /// What a refusal said, for a case that asserts a message rather than a
    /// class. [`Fault`] carries its own text and answers no accessor for it, so
    /// the `Debug` rendering is the one spelling that covers every variant.
    fn said(fault: &Fault) -> String {
        format!("{fault:?}")
    }

    /// A `Core\Db\Connection` over a key nothing is filed under.
    ///
    /// The playbook's standing note is that a `-p nvs-stdlib` test cannot build
    /// any `nvs_db::Connection` — the fields are `pub(crate)` one crate down and
    /// the wire is a TLS stream. It does not need to: both cases below are about
    /// what a member does **before** it reaches the driver, and the receiver a
    /// member reads is two slots that [`nvs_core_db_connect`] writes and nothing
    /// else does.
    fn connection(block: &str) -> Value {
        crate::instance::build(
            &CONNECTION,
            [Value::uint(0), Value::str(NvsStr::new(block.as_bytes()))],
        )
    }

    /// A `Core\Db\Schema` over the empty schema — a value that constructs and
    /// says nothing, which is all either capability case reads.
    fn empty_schema() -> Value {
        let schema = nvs_db::schema::Schema::new(Vec::new()).expect("the empty schema is a schema");
        crate::instance::build(&SCHEMA, [value_of(&schema.to_array())])
    }

    /// `rule:core-classes/schema-apply-capability`'s first sentence: planning is an ordinary read, under the
    /// `db.connect` the program already holds.
    ///
    /// **Asserted as the absence of a refusal, on a context that grants
    /// nothing.** A `Ctx` built here carries no configuration and therefore no
    /// capabilities at all, so a member that asked for one would refuse here
    /// first and could not reach anything else. `planAgainst` instead gets as
    /// far as the connection table and fails there — it has no connection to
    /// read a catalog through, which is the failure a member with no grant to
    /// ask for is supposed to reach.
    ///
    /// The sibling call is what makes that evidence rather than a coincidence:
    /// `applySafe` on the *same* context, the same receiver and the same
    /// connection stops at the grant, so the difference between the two
    /// messages is § 9's split and nothing about the fixture.
    // covers: Core\Db\Schema::planAgainst
    #[test]
    fn plan_against_needs_only_the_db_connect_a_program_already_holds() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let args = [empty_schema(), connection("main")];

        let planning = planned(&mut ctx, &args, PLAN_AGAINST)
            .expect_err("no connection is filed under this key, so the read cannot happen");
        let message = said(&planning);
        assert!(
            !message.contains("db.schema") && !message.to_lowercase().contains("capability"),
            "planning asked for a grant, which § 9 says it does not: {message}"
        );

        let applying = applied(&mut ctx, &args, APPLY_SAFE, unsafe_step)
            .expect_err("`db.schema` is deny-by-default and this context grants nothing");
        assert!(
            said(&applying).contains("db.schema"),
            "the same context refuses `applySafe` for a different reason than the grant: \
             {}",
            said(&applying)
        );
    }

    /// § 9's second half: applying takes `db.schema`, and an ungranted name
    /// throws **naming it**.
    ///
    /// Naming it is the assertion rather than a detail of the message: the
    /// operator reading this failure has to know which line of `nvs.toml` to
    /// write, and a refusal that said only "not permitted" would send them to
    /// the source of the member instead.
    ///
    /// Both entry points are asked, because § 9 gates them identically — the
    /// difference between them is which plans they will run, not which grant
    /// they need — and a check written into one body only would pass a case
    /// that asked one of them.
    // covers: Core\Db\Schema::applySafe
    // covers: Core\Db\Schema::applyIncludingRisky
    #[test]
    fn applying_without_the_db_schema_capability_throws_naming_it() {
        let args = [empty_schema(), connection("reports")];
        for (named, refuse) in [
            (
                APPLY_SAFE,
                &unsafe_step as &dyn Fn(&nvs_db::Plan) -> Option<Fault>,
            ),
            (APPLY_RISKY, &|_: &nvs_db::Plan| None),
        ] {
            let mut ctx = Ctx::new(OutputSink::Sink);
            let refused = applied(&mut ctx, &args, named, refuse)
                .err()
                .unwrap_or_else(|| panic!("{named} ran with no grant at all"));
            let message = said(&refused);
            assert!(
                message.contains("db.schema"),
                "{named} refused without naming the capability: {message}"
            );
            assert!(
                message.contains("reports"),
                "{named} refused without naming the block the grant is for: {message}"
            );
        }
    }

    /// § 9's `applySafe`: refuses a plan holding a step that is not `Safe`, and
    /// names the first one.
    ///
    /// **Over a plan built with no server in front of it**, which is what
    /// [`unsafe_step`] being a function of the plan alone buys: the decision is
    /// the whole of what `applySafe` adds to `applyIncludingRisky`, and a case
    /// needing a database to reach it would be a case this crate cannot run.
    ///
    /// The difference is a widened integer column, because on SQLite that is a
    /// create-copy-drop-rename rebuild and therefore `Destructive` — a step
    /// that **runs** rather than one of § 7's reports, which is the distinction
    /// [`nvs_db::Plan::first_refused`] exists for and the one a rule reading
    /// every step's grade would get wrong.
    // covers: Core\Db\Schema::applySafe
    #[test]
    fn apply_safe_refuses_a_plan_holding_a_step_that_is_not_safe() {
        use nvs_db::schema::{Column, Schema, Table};
        use nvs_db::{Dialect, IntWidth, ScalarType};

        let of = |width| {
            Schema::new(vec![
                Table::new(
                    "notes",
                    vec![Column::new("id", ScalarType::Int(width)).expect("`id` is an identifier")],
                )
                .expect("one column is a table"),
            ])
            .expect("one table is a schema")
        };
        let want = of(IntWidth::Big);
        let have = of(IntWidth::Normal);

        let plan = nvs_db::diff(&want, &have, Dialect::Sqlite);
        assert_eq!(
            plan.len(),
            1,
            "the fixture stopped producing one step:\n{plan}"
        );
        assert!(
            plan.runnable().count() == 1,
            "a widened column is a change and not one of § 7's reports:\n{plan}"
        );

        let refused = unsafe_step(&plan).expect("a `Destructive` step is not `Safe`");
        let message = said(&refused);
        assert!(
            message.contains("applySafe") && message.contains("notes.id"),
            "the refusal does not name the member and the step it refused: {message}"
        );
        assert!(
            message.contains("applyIncludingRisky"),
            "the refusal does not name the entry point that would run it: {message}"
        );
        // The grade's reason is a sentence of its own and ends with a full
        // stop, so the refusal that quotes it must not add a second one.
        assert!(
            !message.contains(".."),
            "the refusal doubles a full stop where it quotes the reason: {message}"
        );

        // The other half of the same rule, and the reason it is `first_refused`
        // rather than a scan of every grade: an empty plan refuses nothing, so
        // convergence is not a state `applySafe` declines to act on.
        assert!(
            unsafe_step(&nvs_db::diff(&want, &want, Dialect::Sqlite)).is_none(),
            "`applySafe` refused a plan with nothing in it"
        );
    }

    /// Drops the one reference a member handed this frame, which is what a
    /// compiled caller owes for every value it was given.
    fn released(value: Value) {
        #[expect(
            unsafe_code,
            reason = "the reference released here is the one this frame was \
                      handed, and nothing else holds it"
        )]
        unsafe {
            value.release();
        }
    }

    /// The smallest schema in its array form: one table, one column, and every
    /// optional key left out.
    fn smallest() -> Value {
        let mut column = NvsArray::new();
        column.set(NvsStr::new(b"name"), Value::str(NvsStr::new(b"id")));
        column.set(NvsStr::new(b"type"), Value::str(NvsStr::new(b"int64")));
        let mut columns = NvsArray::new();
        columns.append(Value::array(column));
        let mut table = NvsArray::new();
        table.set(NvsStr::new(b"name"), Value::str(NvsStr::new(b"notes")));
        table.set(NvsStr::new(b"columns"), Value::array(columns));
        let mut tables = NvsArray::new();
        tables.append(Value::array(table));
        let mut root = NvsArray::new();
        root.set(NvsStr::new(b"tables"), Value::array(tables));
        Value::array(root)
    }

    /// **`Core\Db\Schema::fromArray` keeps none of the array it was given**, and
    /// an array nested past [`DEPTH_LIMIT`] is refused rather than followed.
    ///
    /// The first half is the module doc's decision from the other side: the slot
    /// holds what [`nvs_db::schema::Schema::to_array`] emits, so the caller's
    /// array is read and let go. The reference count is the only place that is
    /// visible — a member that retained the input instead would answer an array
    /// comparing equal on every line a program can write, while a program
    /// writing to its own array afterwards would be editing the schema.
    ///
    /// The second half is the case [`DEPTH_LIMIT`] exists for, and one that
    /// could not be written at all before it: [`node_of`] and [`array_node`]
    /// descend once per level, so nesting like the two thousand below spent the
    /// whole stack and ended the process with no diagnostic. It is refused now,
    /// and the refusal names the nesting rather than the first schema rule the
    /// array happens to break.
    // covers: Core\Db\Schema::fromArray
    #[test]
    fn from_array_keeps_none_of_the_array_it_read_and_refuses_nesting_it_will_not_follow() {
        let mut ctx = Ctx::buffered();
        let written = smallest();
        let held = written
            .array_ptr()
            .expect("the array form was built as an array");

        #[expect(
            unsafe_code,
            reason = "this frame built the array and holds it until the release \
                      at the end"
        )]
        let before = unsafe { NvsArray::refcount_of(held) };

        let schema = nvs_runtime::call(nvs_core_db_schema_from_array, &mut ctx, &[written])
            .expect("one table with one column is a schema every backend takes");

        #[expect(
            unsafe_code,
            reason = "the array is still this frame's, and the schema built from \
                      it is released below as well"
        )]
        let after = unsafe { NvsArray::refcount_of(held) };
        assert_eq!(
            after, before,
            "`fromArray` took a reference to the array it read, so a program \
             writing to its own array afterwards would be editing the schema"
        );

        let answered = nvs_runtime::call(nvs_core_db_schema_to_array, &mut ctx, &[schema])
            .expect("a schema value answers its own array");
        assert_ne!(
            answered.array_ptr(),
            Some(held),
            "the schema holds the array it was given rather than the normalized \
             one, so two spellings of one schema would not compare equal"
        );
        released(answered);
        released(schema);
        released(written);

        // Two thousand arrays, one inside the next. The reader stops at
        // `DEPTH_LIMIT` and says so, rather than following the nesting down.
        let mut deep = Value::array(NvsArray::new());
        for _ in 0..2000 {
            let mut wrap = NvsArray::new();
            wrap.set(NvsStr::new(b"tables"), deep);
            deep = Value::array(wrap);
        }
        let refused = node_of(deep, "the array", 0)
            .expect_err("an array nested two thousand deep is not a schema");
        let message = said(&refused);
        assert!(
            message.contains("nests more than"),
            "the refusal does not say that the nesting is what it refused: \
             {message}"
        );
        assert!(
            message.contains("fromArray"),
            "the refusal does not name the member that was called: {message}"
        );
        released(deep);
    }

    /// **`Core\Db\Schema::toArray` hands on the array the value already holds**,
    /// and retains it once per call.
    ///
    /// The module doc's decision says the slot *is* the schema, so asking twice
    /// has to answer one array rather than two that compare equal. Only the
    /// address says which of those happened: a member that rebuilt the array per
    /// call would print the same JSON, cost a walk of the whole schema every
    /// time a program looked at it, and still pass every case written from
    /// Novis.
    ///
    /// The reference count is the other half, and it is what a compiled caller
    /// rests on: each answer is a reference of its own, so releasing one must
    /// leave the schema's own untouched. A member that handed the slot on
    /// without retaining it would free the schema's array under it the first
    /// time a caller let an answer go.
    // covers: Core\Db\Schema::toArray
    #[test]
    fn to_array_hands_on_the_one_array_the_schema_holds_and_retains_it_per_call() {
        let mut ctx = Ctx::buffered();
        let written = smallest();
        let schema = nvs_runtime::call(nvs_core_db_schema_from_array, &mut ctx, &[written])
            .expect("one table with one column is a schema every backend takes");
        released(written);

        let first = nvs_runtime::call(nvs_core_db_schema_to_array, &mut ctx, &[schema])
            .expect("a schema value answers its own array");
        let held = first
            .array_ptr()
            .expect("the canonical form is an array, whatever the schema says");

        #[expect(
            unsafe_code,
            reason = "the schema and the answer are both this frame's, and both \
                      are released at the end"
        )]
        let after_one = unsafe { NvsArray::refcount_of(held) };

        let second = nvs_runtime::call(nvs_core_db_schema_to_array, &mut ctx, &[schema])
            .expect("a schema value answers its own array as often as it is asked");
        assert_eq!(
            second.array_ptr(),
            Some(held),
            "the second call built a second array, so reading a schema costs a \
             walk of it every time"
        );

        #[expect(
            unsafe_code,
            reason = "both answers are still held by this frame, which releases \
                      them below"
        )]
        let after_two = unsafe { NvsArray::refcount_of(held) };
        assert_eq!(
            after_two,
            after_one + 1,
            "the second call handed on the array without retaining it, so the \
             first caller to let its answer go would free the schema's own"
        );

        released(second);

        #[expect(
            unsafe_code,
            reason = "the first answer and the schema are this frame's until the \
                      two releases below"
        )]
        let after_release = unsafe { NvsArray::refcount_of(held) };
        assert_eq!(
            after_release, after_one,
            "releasing one answer took more than that answer's own reference"
        );

        released(first);
        released(schema);
    }
}
