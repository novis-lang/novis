//! A value crossing into a guest and back: `rule:packaging/a-value-crosses-as-its-wit-type`'s table,
//! keyed by the [`NovisType`] the manifest writes.
//!
//! [`Value`] is the host's view of a Novis value, and it mirrors the runtime's own shapes rather than
//! the WIT ones: an array is one ordered list of key and value pairs, whatever type the manifest
//! gives it. So the host that wires a call into a request copies a runtime value into a [`Value`]
//! with no type in hand, and the type-directed walk is here, once. [`to_wit`] reads a list in its
//! order and ignores its keys, a keyed array as its pairs in order, and a shape by its field names;
//! [`from_wit`] writes a list with the keys `0..n`, a keyed array with the guest's keys in the
//! guest's order, and a shape with only the fields the guest returned, so a `None` optional field is
//! absent.
//!
//! **The named rows.** An enum case is [`Value::Case`], its Novis name, and crosses as the same
//! name in kebab-case. A value of a closed union of shapes is an array like any shape, and crosses
//! as the one case whose fields hold all its keys and whose required fields it has; the manifest has
//! already refused a union where two cases could both fit (`crate::manifest`). A `Core` value class
//! is [`Value::Core`], the class's name and its record's fields by their WIT names, so what reads an
//! instant's seconds out of the runtime's object is the host's, and this module only checks the
//! class and walks the fields.
//!
//! **A key is an `int` or a `string`**, the two a Novis array has. A key the runtime stored as an
//! `int` crosses as its decimal text where the manifest's key type is `string`, because the runtime
//! stores a decimal string key as an `int`; a `string` key from a guest comes back as it is, and the
//! runtime normalizes it on insert as it does any other.
//!
//! **What the conversion refuses** is a value the type does not describe: a `null` where the type is
//! not `?T`, a required shape field that is absent, a key that is neither `int` nor `string`, and a
//! `Val` of another shape than the type's. The checker has already typed the call, so a refusal is a
//! host bug or a guest that wasmtime let lift something the export did not declare, and the message
//! names the type that was expected.
//!
//! **Two rows cross as handles, and only a call has them.** [`to_wit_with`] and [`from_wit_with`]
//! take the call's [`Crossing`]; [`to_wit`] and [`from_wit`] have no call and refuse both rows.
//!
//! - `mixed` is lent, not converted: [`Crossing::lend`] puts the value in the call's handle table
//!   (`crate::handle`) and returns the `borrow<value>` the guest receives. A `mixed` value is any
//!   [`Value`], [`Value::Object`] included: an instance of a class with no row in the table, of
//!   which the guest reads only the class's name. `mixed` is never a return: a borrow cannot be one.
//! - A resource the guest returns is kept by the request, and the program holds
//!   [`Value::Resource`], its type's name and the number [`Crossing::keep`] gave it. Passed back to
//!   the extension, [`Crossing::pass`] finds the resource by that number and the guest receives a
//!   `borrow` of it. A number the request does not keep, or one kept for another resource type, is
//!   refused.
//!
//! What it costs: one copy of the value each way, on top of the canonical ABI's own. wasmtime's
//! dynamic [`Val`] holds one `Val` per element, a byte of `bytes` included, so a `bytes` value costs
//! one `Val` per byte here until the trampoline lowers it through a typed function.

use wasmtime::component::{ResourceAny, Val};

use crate::types::{Field, NovisType};

/// A Novis value as it crosses an extension call.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`, the empty side of `?T`.
    Null,
    /// A `bool`.
    Bool(bool),
    /// An `int`.
    Int(i64),
    /// A `uint`.
    Uint(u64),
    /// A `float`.
    Float(f64),
    /// A `string`.
    String(String),
    /// A `bytes` value.
    Bytes(Vec<u8>),
    /// An array: a list, a keyed array or a shape, its pairs in order.
    Array(Vec<(Key, Value)>),
    /// A case of an enum, by its Novis name.
    Case(String),
    /// An instance of a `Core` value class: the class's full name, and the fields of its record in
    /// `nvs:ext/types` by their WIT names.
    Core {
        /// The class, as `Core\Time\Instant`.
        class: String,
        /// Each field's name and value.
        fields: Vec<(String, Value)>,
    },
    /// An instance of a class that crosses only as a `mixed` value: its class's full name, which
    /// is all a guest can read of it.
    Object(String),
    /// A resource a guest returned, which the request keeps until it ends.
    Resource {
        /// The resource type's short name, as the manifest declares it.
        name: String,
        /// The request's number for it.
        id: u64,
    },
}

/// What a call does with the values that cross as handles: a `mixed` value it lends, a resource it
/// passes back, and a resource a guest returns, which it keeps.
pub trait Crossing {
    /// The `borrow<value>` the guest receives for `value`.
    ///
    /// # Errors
    ///
    /// Why the value could not be lent.
    fn lend(&mut self, value: Value) -> Result<Val, String>;

    /// The resource `name` the request keeps under `id`, as the guest receives it.
    ///
    /// # Errors
    ///
    /// When the request keeps no resource `name` under `id`.
    fn pass(&mut self, name: &str, id: u64) -> Result<Val, String>;

    /// Keeps `resource`, a resource `name` the guest returned, until the request ends, and gives
    /// the value the program holds for it.
    ///
    /// # Errors
    ///
    /// Why the resource could not be kept.
    fn keep(&mut self, name: &str, resource: ResourceAny) -> Result<Value, String>;
}

/// The crossing of a conversion with no call around it, which refuses every handle.
struct NoCall;

impl Crossing for NoCall {
    fn lend(&mut self, _: Value) -> Result<Val, String> {
        Err("a `mixed` value crosses as a handle, which only a call can lend".to_owned())
    }

    fn pass(&mut self, name: &str, _: u64) -> Result<Val, String> {
        Err(format!(
            "the resource `{name}` is kept by a request, and only a call can pass it"
        ))
    }

    fn keep(&mut self, name: &str, _: ResourceAny) -> Result<Value, String> {
        Err(format!(
            "the resource `{name}` is kept by a request, and only a call can keep it"
        ))
    }
}

/// The key of one array entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// An `int` key.
    Int(i64),
    /// A `string` key.
    String(String),
}

/// The WIT value `value` crosses into a guest as, where the manifest's type is `ty`.
///
/// # Errors
///
/// Why `value` is not a value of `ty`, naming the type.
pub fn to_wit(ty: &NovisType, value: Value) -> Result<Val, String> {
    to_wit_with(ty, value, &mut NoCall)
}

/// The WIT value `value` crosses into a guest as, where the manifest's type is `ty` and `call`
/// lends each `mixed` value and passes each resource.
///
/// # Errors
///
/// Why `value` is not a value of `ty`, naming the type, or why `call` could not cross one.
pub fn to_wit_with(ty: &NovisType, value: Value, call: &mut dyn Crossing) -> Result<Val, String> {
    Ok(match (ty, value) {
        (NovisType::Mixed, value) => call.lend(value)?,
        (NovisType::Resource(name), Value::Resource { name: of, id }) if *name == of => {
            call.pass(name, id)?
        }
        (NovisType::Bool, Value::Bool(b)) => Val::Bool(b),
        (NovisType::Int, Value::Int(n)) => Val::S64(n),
        (NovisType::Uint, Value::Uint(n)) => Val::U64(n),
        (NovisType::Float, Value::Float(x)) => Val::Float64(x),
        (NovisType::String, Value::String(s)) => Val::String(s),
        (NovisType::Bytes, Value::Bytes(bytes)) => {
            Val::List(bytes.into_iter().map(Val::U8).collect())
        }
        (NovisType::Optional(_), Value::Null) => Val::Option(None),
        (NovisType::Optional(inner), value) => {
            Val::Option(Some(Box::new(to_wit_with(inner, value, call)?)))
        }
        (NovisType::List(item), Value::Array(entries)) => Val::List(
            entries
                .into_iter()
                .map(|(_, value)| to_wit_with(item, value, call))
                .collect::<Result<_, _>>()?,
        ),
        (NovisType::Keyed(key, value), Value::Array(entries)) => Val::List(
            entries
                .into_iter()
                .map(|(k, v)| {
                    Ok(Val::Tuple(vec![
                        key_to_wit(key, k)?,
                        to_wit_with(value, v, call)?,
                    ]))
                })
                .collect::<Result<_, String>>()?,
        ),
        (NovisType::Shape(fields), Value::Array(entries)) => shape_to_wit(fields, entries, call)?,
        (NovisType::Enum { name, cases }, Value::Case(case)) => {
            if !cases.contains(&case) {
                return Err(format!("`{case}` is not a case of the enum `{name}`"));
            }
            Val::Enum(crate::kebab(&case))
        }
        (NovisType::Union { name, cases }, Value::Array(entries)) => {
            let Some((case, fields)) = cases.iter().find(|(_, fields)| fits(fields, &entries))
            else {
                return Err(format!("the array is a value of no case of `{name}`"));
            };
            Val::Variant(
                crate::kebab(case),
                Some(Box::new(shape_to_wit(fields, entries, call)?)),
            )
        }
        (NovisType::Core(core), Value::Core { class, mut fields }) if class == core.class => {
            let mut record = Vec::with_capacity(core.fields.len());
            for (name, ty) in core.fields {
                let Some(at) = fields.iter().position(|(n, _)| n == name) else {
                    return Err(format!("the `{class}` has no field `{name}`"));
                };
                let value = fields.swap_remove(at).1;
                record.push(((*name).to_owned(), to_wit_with(ty, value, call)?));
            }
            Val::Record(record)
        }
        (ty, value) => return Err(mismatch(ty, &value)),
    })
}

/// The WIT record of the shape `fields`, where `entries` are the array's.
fn shape_to_wit(
    fields: &[Field],
    mut entries: Vec<(Key, Value)>,
    call: &mut dyn Crossing,
) -> Result<Val, String> {
    let mut record = Vec::with_capacity(fields.len());
    for (name, optional, ty) in fields {
        let found = entries
            .iter()
            .position(|(key, _)| matches!(key, Key::String(k) if k == name))
            .map(|at| entries.swap_remove(at).1);
        let val = match (found, optional) {
            (Some(value), false) => to_wit_with(ty, value, call)?,
            (Some(value), true) => Val::Option(Some(Box::new(to_wit_with(ty, value, call)?))),
            (None, true) => Val::Option(None),
            (None, false) => {
                return Err(format!(
                    "the shape has no field `{name}`, which is required"
                ));
            }
        };
        record.push((crate::kebab(name), val));
    }
    Ok(Val::Record(record))
}

/// Whether the array of `entries` is a value of the shape `fields`: every key is one of its
/// fields, and every required field is a key.
fn fits(fields: &[Field], entries: &[(Key, Value)]) -> bool {
    let has = |name: &str| {
        entries
            .iter()
            .any(|(key, _)| matches!(key, Key::String(k) if k == name))
    };
    entries
        .iter()
        .all(|(key, _)| matches!(key, Key::String(k) if fields.iter().any(|(name, ..)| name == k)))
        && fields
            .iter()
            .all(|(name, optional, _)| *optional || has(name))
}

/// The Novis value the WIT value `val` a guest returned is, where the manifest's type is `ty`.
///
/// # Errors
///
/// Why `val` is not a WIT value of `ty`, naming the type.
pub fn from_wit(ty: &NovisType, val: Val) -> Result<Value, String> {
    from_wit_with(ty, val, &mut NoCall)
}

/// The Novis value the WIT value `val` a guest returned is, where the manifest's type is `ty` and
/// `call` keeps each resource.
///
/// # Errors
///
/// Why `val` is not a WIT value of `ty`, naming the type, or why `call` could not keep a resource.
pub fn from_wit_with(ty: &NovisType, val: Val, call: &mut dyn Crossing) -> Result<Value, String> {
    Ok(match (ty, val) {
        (NovisType::Resource(name), Val::Resource(resource)) if resource.owned() => {
            call.keep(name, resource)?
        }
        (NovisType::Bool, Val::Bool(b)) => Value::Bool(b),
        (NovisType::Int, Val::S64(n)) => Value::Int(n),
        (NovisType::Uint, Val::U64(n)) => Value::Uint(n),
        (NovisType::Float, Val::Float64(x)) => Value::Float(x),
        (NovisType::String, Val::String(s)) => Value::String(s),
        (NovisType::Bytes, Val::List(items)) => Value::Bytes(
            items
                .into_iter()
                .map(|item| match item {
                    Val::U8(byte) => Ok(byte),
                    other => Err(wit_mismatch(ty, &other)),
                })
                .collect::<Result<_, _>>()?,
        ),
        (NovisType::Optional(_), Val::Option(None)) => Value::Null,
        (NovisType::Optional(inner), Val::Option(Some(val))) => from_wit_with(inner, *val, call)?,
        (NovisType::List(item), Val::List(items)) => Value::Array(
            (0_i64..)
                .zip(items)
                .map(|(at, val)| Ok((Key::Int(at), from_wit_with(item, val, call)?)))
                .collect::<Result<_, String>>()?,
        ),
        (NovisType::Keyed(key, value), Val::List(items)) => Value::Array(
            items
                .into_iter()
                .map(|item| match item {
                    Val::Tuple(pair) => match <[Val; 2]>::try_from(pair) {
                        Ok([k, v]) => Ok((key_from_wit(key, k)?, from_wit_with(value, v, call)?)),
                        Err(pair) => Err(wit_mismatch(ty, &Val::Tuple(pair))),
                    },
                    other => Err(wit_mismatch(ty, &other)),
                })
                .collect::<Result<_, String>>()?,
        ),
        (NovisType::Shape(fields), val) => shape_from_wit(ty, fields, val, call)?,
        (NovisType::Enum { cases, .. }, Val::Enum(wit)) => {
            match cases.iter().find(|case| crate::kebab(case) == wit) {
                Some(case) => Value::Case(case.clone()),
                None => return Err(wit_mismatch(ty, &Val::Enum(wit))),
            }
        }
        (NovisType::Union { cases, .. }, Val::Variant(wit, Some(payload))) => {
            match cases.iter().find(|(case, _)| crate::kebab(case) == wit) {
                Some((_, fields)) => shape_from_wit(ty, fields, *payload, call)?,
                None => return Err(wit_mismatch(ty, &Val::Variant(wit, Some(payload)))),
            }
        }
        (NovisType::Core(core), Val::Record(record)) if record.len() == core.fields.len() => {
            Value::Core {
                class: core.class.to_owned(),
                fields: core
                    .fields
                    .iter()
                    .zip(record)
                    .map(|((name, ty), (_, val))| {
                        Ok(((*name).to_owned(), from_wit_with(ty, val, call)?))
                    })
                    .collect::<Result<_, String>>()?,
            }
        }
        (ty, val) => return Err(wit_mismatch(ty, &val)),
    })
}

/// The array the WIT record `val` is, where its shape is `fields` and `ty` is the type the
/// manifest wrote.
fn shape_from_wit(
    ty: &NovisType,
    fields: &[Field],
    val: Val,
    call: &mut dyn Crossing,
) -> Result<Value, String> {
    let record = match val {
        Val::Record(record) if record.len() == fields.len() => record,
        other => return Err(wit_mismatch(ty, &other)),
    };
    let mut entries = Vec::with_capacity(fields.len());
    for ((name, optional, ty), (_, val)) in fields.iter().zip(record) {
        let value = match (optional, val) {
            (true, Val::Option(None)) => continue,
            (true, Val::Option(Some(val))) => from_wit_with(ty, *val, call)?,
            (true, other) => return Err(wit_mismatch(ty, &other)),
            (false, val) => from_wit_with(ty, val, call)?,
        };
        entries.push((Key::String(name.clone()), value));
    }
    Ok(Value::Array(entries))
}

/// The WIT value of the key `key`, where the manifest's key type is `ty`.
fn key_to_wit(ty: &NovisType, key: Key) -> Result<Val, String> {
    match (ty, key) {
        (NovisType::String, Key::Int(n)) => Ok(Val::String(n.to_string())),
        (_, Key::Int(n)) => to_wit(ty, Value::Int(n)),
        (_, Key::String(s)) => to_wit(ty, Value::String(s)),
    }
}

/// The key the WIT value `val` is, where the manifest's key type is `ty`.
fn key_from_wit(ty: &NovisType, val: Val) -> Result<Key, String> {
    match from_wit(ty, val)? {
        Value::Int(n) => Ok(Key::Int(n)),
        Value::String(s) => Ok(Key::String(s)),
        other => Err(format!(
            "an array key is an `int` or a `string`, and the guest returned {other:?}"
        )),
    }
}

fn mismatch(ty: &NovisType, value: &Value) -> String {
    format!("{value:?} is not a value of the type {ty:?}")
}

fn wit_mismatch(ty: &NovisType, val: &Val) -> String {
    format!("the WIT value {val:?} is not one the type {ty:?} crosses as")
}
