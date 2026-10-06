//! A `mixed` argument, held by the host and read by the guest through `nvs:ext/types`'s `value`
//! resource (`rule:packaging/values-cross-as-handles`).
//!
//! A `mixed` argument crosses as a `borrow<value>` whose rep is an index into the store's
//! [`Handles`], and the guest reads it through the accessor functions [`link`] defines: `kind`,
//! `as-bool` to `as-bytes`, `length`, `key`, `element`, `field` and `class-name`. `key`, `element`
//! and `field` return an owned `value` the guest drops when it is done with it. A rep is never a
//! pointer: a host function looks it up in the table, and a rep that is not there traps the call.
//!
//! **A handle lives for one call.** The call's arguments are the table's roots, and a handle is a
//! root and the path of positions to the entry under it, so reading an element copies nothing until
//! the guest asks for a scalar, a string or a byte string. When the call returns, every root and
//! every handle goes ([`Handles::end_call`]). Reps are never reused within an instance, so a handle
//! a guest kept in its memory past its call names nothing, and reading it traps rather than
//! reading a later call's argument.
//!
//! **Value kinds.** An array is `array`, a list or a keyed array alike. An instance of a class is
//! `object`, whose class `class-name` returns and whose properties the guest cannot read: a `Core`
//! value class, [`Value::Object`], and an enum case, whose class-name is `none` because
//! [`Value::Case`] carries its case and not its enum. A key is an `int` or a `string` value of its
//! own. `field("5")` finds the key `5` too, as `$a["5"]` does in Novis.
//!
//! What it spends: one table entry per handle the guest holds, each a root index and a path, and
//! at most [`MOST`] at once, so a guest looping over `element` cannot grow host memory past a
//! bound its request did not pay for. A key's handle copies the key.

use std::collections::HashMap;

use wasmtime::component::{Linker, Resource, ResourceAny, ResourceType, Val};
use wasmtime::{AsContextMut, StoreContextMut};

use crate::call::Guest;
use crate::convert::{Key, Value};

/// The most handles a guest may hold at once in one call. Past it, the accessor traps.
pub const MOST: usize = 1 << 16;

/// The host's side of `nvs:ext/types`'s `value` resource.
#[derive(Debug)]
pub struct Handle;

/// One instance's table of `value` handles.
#[derive(Debug, Default)]
pub struct Handles {
    roots: Vec<Value>,
    live: HashMap<u32, Entry>,
    next: u32,
    lent: Vec<ResourceAny>,
}

#[derive(Debug)]
enum Entry {
    /// The entry at the positions `path` under the argument `root`.
    At { root: usize, path: Vec<usize> },
    /// A key, copied out of its array.
    Key(Value),
}

impl Handles {
    /// Takes the borrows lent for this call's arguments, which the host drops once the call
    /// returns.
    pub(crate) fn take_lent(&mut self) -> Vec<ResourceAny> {
        std::mem::take(&mut self.lent)
    }

    /// Ends the call: every root and every handle it made stops being valid.
    pub(crate) fn end_call(&mut self) {
        self.roots.clear();
        self.live.clear();
    }

    fn insert(&mut self, entry: Entry) -> Result<u32, String> {
        if self.live.len() >= MOST {
            return Err(format!(
                "the extension holds more than {MOST} `value` handles at once"
            ));
        }
        let rep = self.next;
        self.next = rep
            .checked_add(1)
            .ok_or("the extension has used every `value` handle an instance may make")?;
        self.live.insert(rep, entry);
        Ok(rep)
    }

    fn get(&self, rep: u32) -> Result<&Value, String> {
        let stale = || "the `value` handle is not one this call received".to_owned();
        match self.live.get(&rep).ok_or_else(stale)? {
            Entry::Key(value) => Ok(value),
            Entry::At { root, path } => {
                let mut value = self.roots.get(*root).ok_or_else(stale)?;
                for &at in path {
                    value = match value {
                        Value::Array(entries) => &entries.get(at).ok_or_else(stale)?.1,
                        _ => return Err(stale()),
                    };
                }
                Ok(value)
            }
        }
    }

    /// A handle on the entry at `at` of the array `rep` is, or `None` where it is not an array or
    /// has no such entry.
    fn child(&mut self, rep: u32, at: Option<usize>) -> Result<Option<u32>, String> {
        let len = match self.get(rep)? {
            Value::Array(entries) => entries.len(),
            _ => return Ok(None),
        };
        let Some(at) = at.filter(|at| *at < len) else {
            return Ok(None);
        };
        let Some(Entry::At { root, path }) = self.live.get(&rep) else {
            return Ok(None);
        };
        let mut path = path.clone();
        path.push(at);
        let entry = Entry::At { root: *root, path };
        self.insert(entry).map(Some)
    }

    /// A handle on the key at `at` of the array `rep` is.
    fn key(&mut self, rep: u32, at: Option<usize>) -> Result<Option<u32>, String> {
        let key = match self.get(rep)? {
            Value::Array(entries) => match at.and_then(|at| entries.get(at)) {
                Some((Key::Int(n), _)) => Value::Int(*n),
                Some((Key::String(s), _)) => Value::String(s.clone()),
                None => return Ok(None),
            },
            _ => return Ok(None),
        };
        self.insert(Entry::Key(key)).map(Some)
    }

    /// A handle on the entry under the key `name` of the array `rep` is.
    fn field(&mut self, rep: u32, name: &str) -> Result<Option<u32>, String> {
        let at = match self.get(rep)? {
            Value::Array(entries) => entries.iter().position(|(key, _)| match key {
                Key::String(key) => key == name,
                Key::Int(n) => n.to_string() == name,
            }),
            _ => return Ok(None),
        };
        match at {
            Some(at) => self.child(rep, Some(at)),
            None => Ok(None),
        }
    }
}

/// Lends `value` to the call about to start: a `borrow<value>` on a new root of the store's table.
///
/// # Errors
///
/// When the table is full, or wasmtime cannot lower the borrow.
pub(crate) fn lend(mut store: StoreContextMut<'_, Guest>, value: Value) -> Result<Val, String> {
    let handles = &mut store.data_mut().handles;
    let root = handles.roots.len();
    handles.roots.push(value);
    let rep = handles.insert(Entry::At {
        root,
        path: Vec::new(),
    })?;
    // The host owns the resource and the export's `borrow` parameter lends it: wasmtime lowers a
    // borrow the host makes itself only inside a host call's scope, and there is none here.
    let any = Resource::<Handle>::new_own(rep)
        .try_into_resource_any(store.as_context_mut())
        .map_err(|err| err.to_string())?;
    store.data_mut().handles.lent.push(any);
    Ok(Val::Resource(any))
}

/// Defines `nvs:ext/types`'s `value` resource and its accessors in `linker`.
///
/// # Errors
///
/// When a definition is refused, which is a name defined twice.
pub(crate) fn link(linker: &mut Linker<Guest>) -> wasmtime::Result<()> {
    let mut types = linker.instance("nvs:ext/types@1.0.0")?;
    types.resource("value", ResourceType::host::<Handle>(), |mut store, rep| {
        store.data_mut().handles.live.remove(&rep);
        Ok(())
    })?;
    types.func_new("[method]value.kind", |mut store, _, params, results| {
        let [Val::Resource(this)] = params else {
            wasmtime::bail!("`kind` takes one `value`");
        };
        let this = this.try_into_resource::<Handle>(store.as_context_mut())?;
        let kind = match read(&store, &this)? {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Uint(_) => "uint",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Bytes(_) => "bytes",
            Value::Array(_) => "array",
            Value::Case(_) | Value::Core { .. } | Value::Object(_) | Value::Resource { .. } => {
                "object"
            }
        };
        if let Some(slot) = results.first_mut() {
            *slot = Val::Enum(kind.to_owned());
        }
        Ok(())
    })?;
    types.func_wrap(
        "[method]value.as-bool",
        |store, (this,): (Resource<Handle>,)| {
            Ok((match read(&store, &this)? {
                Value::Bool(b) => Some(*b),
                _ => None,
            },))
        },
    )?;
    types.func_wrap(
        "[method]value.as-int",
        |store, (this,): (Resource<Handle>,)| {
            Ok((match read(&store, &this)? {
                Value::Int(n) => Some(*n),
                _ => None,
            },))
        },
    )?;
    types.func_wrap(
        "[method]value.as-uint",
        |store, (this,): (Resource<Handle>,)| {
            Ok((match read(&store, &this)? {
                Value::Uint(n) => Some(*n),
                _ => None,
            },))
        },
    )?;
    types.func_wrap(
        "[method]value.as-float",
        |store, (this,): (Resource<Handle>,)| {
            Ok((match read(&store, &this)? {
                Value::Float(x) => Some(*x),
                _ => None,
            },))
        },
    )?;
    types.func_wrap(
        "[method]value.as-string",
        |store, (this,): (Resource<Handle>,)| {
            Ok((match read(&store, &this)? {
                Value::String(s) => Some(s.clone()),
                _ => None,
            },))
        },
    )?;
    types.func_wrap(
        "[method]value.as-bytes",
        |store, (this,): (Resource<Handle>,)| {
            Ok((match read(&store, &this)? {
                Value::Bytes(bytes) => Some(bytes.clone()),
                _ => None,
            },))
        },
    )?;
    types.func_wrap(
        "[method]value.length",
        |store, (this,): (Resource<Handle>,)| {
            Ok((match read(&store, &this)? {
                Value::Array(entries) => u64::try_from(entries.len()).ok(),
                _ => None,
            },))
        },
    )?;
    types.func_wrap(
        "[method]value.key",
        |mut store, (this, index): (Resource<Handle>, u64)| {
            let at = usize::try_from(index).ok();
            let rep = store.data_mut().handles.key(this.rep(), at);
            owned(rep)
        },
    )?;
    types.func_wrap(
        "[method]value.element",
        |mut store, (this, index): (Resource<Handle>, u64)| {
            let at = usize::try_from(index).ok();
            let rep = store.data_mut().handles.child(this.rep(), at);
            owned(rep)
        },
    )?;
    types.func_wrap(
        "[method]value.field",
        |mut store, (this, name): (Resource<Handle>, String)| {
            let rep = store.data_mut().handles.field(this.rep(), &name);
            owned(rep)
        },
    )?;
    types.func_wrap(
        "[method]value.class-name",
        |store, (this,): (Resource<Handle>,)| {
            Ok((match read(&store, &this)? {
                Value::Core { class, .. } | Value::Object(class) => Some(class.clone()),
                _ => None,
            },))
        },
    )?;
    Ok(())
}

/// The value the handle `this` is, or a trap where it is not one this call received.
fn read<'s>(
    store: &'s StoreContextMut<'_, Guest>,
    this: &Resource<Handle>,
) -> wasmtime::Result<&'s Value> {
    store
        .data()
        .handles
        .get(this.rep())
        .map_err(|err| wasmtime::format_err!("{err}"))
}

/// The owned handle a `key`, `element` or `field` returns, or a trap.
fn owned(rep: Result<Option<u32>, String>) -> wasmtime::Result<(Option<Resource<Handle>>,)> {
    match rep {
        Ok(rep) => Ok((rep.map(Resource::new_own),)),
        Err(err) => Err(wasmtime::format_err!("{err}")),
    }
}
