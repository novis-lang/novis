//! `Core\Db\Schema` — [ADR 0145](/docs/adr/0145-a-schema-is-a-value-core-db-schema-converges-a-closed.md)
//! § 1's canonical array form, as a value a program holds.
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

/// One Novis value as a node of the canonical form.
///
/// The vocabulary's own leaves and nothing else: a `decimal`, a `bytes`, an
/// object or a `null` inside a schema array is refused here rather than
/// silently rendered into something [`nvs_db::schema::Schema::from_array`]
/// would then reject under a worse message. `Tag::Decimal` is the one that
/// looks like an omission and is not — § 2's decimal default is *digits*, so
/// that a value written into a schema file and one written in source are the
/// same value.
fn node_of(value: Value, at: &str) -> Result<nvs_db::schema::Node, Fault> {
    use nvs_db::schema::Node;

    let refused = |holds: &str| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Db\\Schema::fromArray(): {at} holds {holds}. A schema array holds only \
                 strings, integers, floats, booleans and arrays of those — ADR 0145 § 2's \
                 vocabulary is closed and there is nowhere in it to put anything else"
            ),
        )
    };
    match value.tag() {
        Some(Tag::Bool) => Ok(Node::Bool(value.as_bool() == Some(true))),
        Some(Tag::Int) => Ok(Node::Int(value.as_int().unwrap_or(0))),
        Some(Tag::Uint) => Ok(Node::Uint(value.as_uint().unwrap_or(0))),
        Some(Tag::Float) => Ok(Node::Float(value.as_float().unwrap_or(f64::NAN))),
        // A name and a type spelling are both text, and both are compared byte
        // for byte further down — so a string that is not UTF-8 is refused here
        // rather than lossily rendered into a name nobody wrote.
        Some(Tag::Str) => value
            .as_text()
            .map(|text| Node::Text(text.to_owned()))
            .ok_or_else(|| refused("a string that is not UTF-8")),
        Some(Tag::Array) => array_node(value, at),
        Some(Tag::Decimal) => Err(refused(
            "a `decimal`, where § 2's exact default is written as its digits in a string",
        )),
        Some(Tag::Bytes) => Err(refused("`bytes`")),
        Some(Tag::Object) => Err(refused("an object")),
        None | Some(Tag::Null | Tag::Unset) => Err(refused(
            "`null`, where a key a schema does not set is left out instead",
        )),
        Some(Tag::Closure | Tag::Resource) => Err(refused("a value with no representation")),
    }
}

/// An array as either arm of the form: a list where every position from zero is
/// there, and an ordered map otherwise.
///
/// One `NvsArray` is both of Novis's shapes ([ADR 0007](/docs/adr/0007-explicit-type-system.md)
/// § 5), so the two are told apart exactly as [`crate::json`]'s decode tells
/// them apart — by asking for the positions. `["tables" => …]` has a count of
/// one and no index 0, and `[["name" => …]]` has both.
fn array_node(value: Value, at: &str) -> Result<nvs_db::schema::Node, Fault> {
    use nvs_db::schema::Node;

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
            items.push(node_of(element, &format!("{at}[{index}]"))?);
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
        let node = node_of(element, &format!("{at}[\"{key}\"]"))?;
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
/// driver crate's, which holds no Novis-facing name (ADR 0132 § 1), and the
/// call is named here.
fn refused(member: &str, why: &nvs_db::schema::SchemaError) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!("Core\\Db\\Schema::{member}(): {why}"),
    )
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Schema::fromArray(array<mixed> $array): Db\Schema` — ADR 0145
    /// § 1's serialized spelling, and the door every schema value comes through.
    ///
    /// The array is validated by the builders themselves —
    /// [`nvs_db::schema::Schema::from_array`] reads the form and hands it to
    /// them — so a file cannot say anything a program could not have built, and
    /// this member has no rule of its own to disagree with them about.
    fn nvs_core_db_schema_from_array(_ctx, args: [1]) {
        let node = node_of(args[0], "the array")?;
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
