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
//! **A key is an `int` or a `string`**, the two a Novis array has. A key the runtime stored as an
//! `int` crosses as its decimal text where the manifest's key type is `string`, because the runtime
//! stores a decimal string key as an `int`; a `string` key from a guest comes back as it is, and the
//! runtime normalizes it on insert as it does any other.
//!
//! **What the conversion refuses** is a value the type does not describe: a `null` where the type is
//! not `?T`, a required shape field that is absent, a key that is neither `int` nor `string`, and a
//! `Val` of another shape than the type's. The checker has already typed the call, so a refusal is a
//! host bug or a guest that wasmtime let lift something the export did not declare, and the message
//! names the type that was expected. `mixed` is refused too: it crosses as a handle into the call's
//! handle table, which this module does not hold.
//!
//! What it costs: one copy of the value each way, on top of the canonical ABI's own. wasmtime's
//! dynamic [`Val`] holds one `Val` per element, a byte of `bytes` included, so a `bytes` value costs
//! one `Val` per byte here until the trampoline lowers it through a typed function.

use wasmtime::component::Val;

use crate::types::NovisType;

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
    Ok(match (ty, value) {
        (NovisType::Bool, Value::Bool(b)) => Val::Bool(b),
        (NovisType::Int, Value::Int(n)) => Val::S64(n),
        (NovisType::Uint, Value::Uint(n)) => Val::U64(n),
        (NovisType::Float, Value::Float(x)) => Val::Float64(x),
        (NovisType::String, Value::String(s)) => Val::String(s),
        (NovisType::Bytes, Value::Bytes(bytes)) => {
            Val::List(bytes.into_iter().map(Val::U8).collect())
        }
        (NovisType::Optional(_), Value::Null) => Val::Option(None),
        (NovisType::Optional(inner), value) => Val::Option(Some(Box::new(to_wit(inner, value)?))),
        (NovisType::List(item), Value::Array(entries)) => Val::List(
            entries
                .into_iter()
                .map(|(_, value)| to_wit(item, value))
                .collect::<Result<_, _>>()?,
        ),
        (NovisType::Keyed(key, value), Value::Array(entries)) => Val::List(
            entries
                .into_iter()
                .map(|(k, v)| Ok(Val::Tuple(vec![key_to_wit(key, k)?, to_wit(value, v)?])))
                .collect::<Result<_, String>>()?,
        ),
        (NovisType::Shape(fields), Value::Array(mut entries)) => {
            let mut record = Vec::with_capacity(fields.len());
            for (name, optional, ty) in fields {
                let found = entries
                    .iter()
                    .position(|(key, _)| matches!(key, Key::String(k) if k == name))
                    .map(|at| entries.swap_remove(at).1);
                let val = match (found, optional) {
                    (Some(value), false) => to_wit(ty, value)?,
                    (Some(value), true) => Val::Option(Some(Box::new(to_wit(ty, value)?))),
                    (None, true) => Val::Option(None),
                    (None, false) => {
                        return Err(format!(
                            "the shape has no field `{name}`, which is required"
                        ));
                    }
                };
                record.push((crate::kebab(name), val));
            }
            Val::Record(record)
        }
        (ty, value) => return Err(mismatch(ty, &value)),
    })
}

/// The Novis value the WIT value `val` a guest returned is, where the manifest's type is `ty`.
///
/// # Errors
///
/// Why `val` is not a WIT value of `ty`, naming the type.
pub fn from_wit(ty: &NovisType, val: Val) -> Result<Value, String> {
    Ok(match (ty, val) {
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
        (NovisType::Optional(inner), Val::Option(Some(val))) => from_wit(inner, *val)?,
        (NovisType::List(item), Val::List(items)) => Value::Array(
            (0_i64..)
                .zip(items)
                .map(|(at, val)| Ok((Key::Int(at), from_wit(item, val)?)))
                .collect::<Result<_, String>>()?,
        ),
        (NovisType::Keyed(key, value), Val::List(items)) => Value::Array(
            items
                .into_iter()
                .map(|item| match item {
                    Val::Tuple(pair) => match <[Val; 2]>::try_from(pair) {
                        Ok([k, v]) => Ok((key_from_wit(key, k)?, from_wit(value, v)?)),
                        Err(pair) => Err(wit_mismatch(ty, &Val::Tuple(pair))),
                    },
                    other => Err(wit_mismatch(ty, &other)),
                })
                .collect::<Result<_, String>>()?,
        ),
        (NovisType::Shape(fields), Val::Record(record)) if record.len() == fields.len() => {
            let mut entries = Vec::with_capacity(fields.len());
            for ((name, optional, ty), (_, val)) in fields.iter().zip(record) {
                let value = match (optional, val) {
                    (true, Val::Option(None)) => continue,
                    (true, Val::Option(Some(val))) => from_wit(ty, *val)?,
                    (true, other) => return Err(wit_mismatch(ty, &other)),
                    (false, val) => from_wit(ty, val)?,
                };
                entries.push((Key::String(name.clone()), value));
            }
            Value::Array(entries)
        }
        (ty, val) => return Err(wit_mismatch(ty, &val)),
    })
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
