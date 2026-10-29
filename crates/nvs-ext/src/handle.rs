//! A `mixed` argument, held by the host and read by the guest through `nvs:ext/types`'s `value`
//! resource (`rule:packaging/values-cross-as-handles`).
//!
//! A `mixed` argument crosses as a `borrow<value>` whose rep is an index into the store's
//! [`Handles`], and the guest reads it through the accessor functions [`link`] defines: `kind`,
//! `as-bool` to `as-bytes`, `size`, `range`, `length`, `key`, `element`, `field` and `class-name`.
//! `key`, `element` and `field` return an owned `value` the guest drops when it is done with it. A
//! rep is never a pointer: a host function looks it up in the table, and a rep that is not there
//! traps the call. `range` copies at most [`RANGE_MOST`] bytes of a byte string, so a guest reads
//! a large one a block at a time (`rule:core-classes/spreadsheet-reads-as-a-stream`).
//!
//! **A lent source** ([`Value::Lent`]) is a `bytes` value or an open file the host keeps and lends
//! by number. It reads as `bytes`, and `size` is its length. `as-bytes` and `range` on it are async
//! imports: each asks [`crate::call::Budget::read`] for the bytes, and the guest's call waits,
//! pending, until the host serves the read where it holds the source.
//!
//! **A read past the request's memory is refused before it is made.** `as-bytes` on a lent source
//! reads all of it, and on a byte string copies all of it, so each first asks
//! [`crate::call::Budget::left`], and a size past it ends the call at the memory limit
//! (`rule:packaging/a-guest-runs-under-the-requests-budget`). `range` reads at most [`RANGE_MOST`]
//! and is asked the same.
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
//! **A `bytes` of a third-party export crosses as the `bytes` resource**
//! (`rule:packaging/a-value-crosses-as-its-wit-type`), a second table of [`Handles`] with its own
//! reps. An argument is lent as a `borrow<bytes>` on the program's own byte string, moved into the
//! table and not copied, or on a [`Value::Lent`] source. The guest reads it with `size`, `read` and
//! `range`, each a typed host function that copies once. A result is a `bytes` the guest made with
//! the constructor and filled with `append`, and [`take_bytes`] moves its buffer out as the
//! program's value. Only a `bytes` the guest made takes `append`, so a guest never changes an
//! argument. `read` asks [`crate::call::Budget::left`] first, as `as-bytes` does, and `append`
//! charges the request before it copies, so a result past the budget ends the call at the memory
//! limit. Every `bytes` handle goes with its call too, and its charge is given back then: the
//! result has moved to the program by that time, and the program's memory holds it from there.
//!
//! What it spends: one table entry per handle the guest holds, each a root index and a path, and
//! at most [`MOST`] at once, so a guest looping over `element` cannot grow host memory past a
//! bound its request did not pay for. A key's handle copies the key. A `bytes` handle is one entry
//! more, at most [`MOST`] of them as well, and a result's buffer is charged to its request while
//! the call runs.

use std::collections::HashMap;

use wasmtime::component::{Linker, Resource, ResourceAny, ResourceType, Val, WasmList};
use wasmtime::{AsContextMut, StoreContextMut};

use crate::call::Guest;
use crate::convert::{Key, Value};

/// The most handles a guest may hold at once in one call. Past it, the accessor traps.
pub const MOST: usize = 1 << 16;

/// The most bytes one `range` call gives, so a read of a large value copies one block at a time.
pub const RANGE_MOST: usize = 1 << 20;

/// The host's side of `nvs:ext/types`'s `value` resource.
#[derive(Debug)]
pub struct Handle;

/// The host's side of `nvs:ext/types`'s `bytes` resource.
#[derive(Debug)]
pub struct Bytes;

/// One instance's table of `value` and `bytes` handles.
#[derive(Debug, Default)]
pub struct Handles {
    roots: Vec<Value>,
    live: HashMap<u32, Entry>,
    next: u32,
    lent: Vec<ResourceAny>,
    blobs: HashMap<u32, Blob>,
    next_blob: u32,
    grown: u64,
}

/// One `bytes` handle: a [`Value::Bytes`] or a [`Value::Lent`] source, and whether the guest made
/// it, which only `append` reads.
#[derive(Debug)]
struct Blob {
    value: Value,
    made: bool,
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

    /// Ends the call: every root and every handle it made stops being valid. Returns the bytes
    /// `append` charged to the request during the call, which the caller gives back.
    pub(crate) fn end_call(&mut self) -> u64 {
        self.roots.clear();
        self.live.clear();
        self.blobs.clear();
        std::mem::take(&mut self.grown)
    }

    fn insert_blob(&mut self, blob: Blob) -> Result<u32, String> {
        if self.blobs.len() >= MOST {
            return Err(format!(
                "the extension holds more than {MOST} `bytes` handles at once"
            ));
        }
        let rep = self.next_blob;
        self.next_blob = rep
            .checked_add(1)
            .ok_or("the extension has used every `bytes` handle an instance may make")?;
        self.blobs.insert(rep, blob);
        Ok(rep)
    }

    fn blob(&self, rep: u32) -> wasmtime::Result<&Blob> {
        self.blobs
            .get(&rep)
            .ok_or_else(|| wasmtime::format_err!("the `bytes` handle is not one this call holds"))
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

/// Lends the byte string `value`, a [`Value::Bytes`] or a [`Value::Lent`] source, to the call about
/// to start as a `borrow<bytes>`. A [`Value::Bytes`] moves into the table and is not copied.
///
/// # Errors
///
/// When `value` is no byte string, the table is full, or wasmtime cannot lower the borrow.
pub(crate) fn lend_bytes(
    mut store: StoreContextMut<'_, Guest>,
    value: Value,
) -> Result<Val, String> {
    if !matches!(value, Value::Bytes(_) | Value::Lent { .. }) {
        return Err(format!("{value:?} is not a value of the type Bytes"));
    }
    let rep = store
        .data_mut()
        .handles
        .insert_blob(Blob { value, made: false })?;
    // Lent from a host-owned resource, as `lend` does, and dropped when the call returns.
    let any = Resource::<Bytes>::new_own(rep)
        .try_into_resource_any(store.as_context_mut())
        .map_err(|err| err.to_string())?;
    store.data_mut().handles.lent.push(any);
    Ok(Val::Resource(any))
}

/// The byte string of the `own<bytes>` the guest returned as `resource`, moved out of the table
/// and not copied.
///
/// # Errors
///
/// When `resource` is no `bytes`, or no byte string this call holds.
pub(crate) fn take_bytes(
    mut store: StoreContextMut<'_, Guest>,
    resource: ResourceAny,
) -> Result<Vec<u8>, String> {
    let rep = resource
        .try_into_resource::<Bytes>(store.as_context_mut())
        .map_err(|err| format!("the extension did not return a `bytes`: {err}"))?
        .rep();
    match store.data_mut().handles.blobs.remove(&rep) {
        Some(Blob {
            value: Value::Bytes(bytes),
            ..
        }) => Ok(bytes),
        _ => Err("the extension returned a `bytes` handle this call does not hold".to_owned()),
    }
}

/// Defines `nvs:ext/types`'s `value` and `bytes` resources and their methods in `linker`.
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
            Value::Bytes(_) | Value::Lent { .. } => "bytes",
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
    types.func_wrap_async(
        "[method]value.as-bytes",
        |store, (this,): (Resource<Handle>,)| {
            let read = bytes_of(&store, &this, 0, None);
            Box::new(async move { Ok((read?.await?,)) })
        },
    )?;
    types.func_wrap(
        "[method]value.size",
        |store, (this,): (Resource<Handle>,)| {
            Ok((match read(&store, &this)? {
                Value::Bytes(bytes) => u64::try_from(bytes.len()).ok(),
                Value::Lent { size, .. } => Some(*size),
                _ => None,
            },))
        },
    )?;
    types.func_wrap_async(
        "[method]value.range",
        |store, (this, offset, length): (Resource<Handle>, u64, u64)| {
            let read = bytes_of(&store, &this, offset, Some(length));
            Box::new(async move { Ok((read?.await?,)) })
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
    link_bytes(&mut types)
}

/// Defines `nvs:ext/types`'s `bytes` resource and its five methods in `types`.
fn link_bytes(types: &mut wasmtime::component::LinkerInstance<'_, Guest>) -> wasmtime::Result<()> {
    types.resource("bytes", ResourceType::host::<Bytes>(), |mut store, rep| {
        store.data_mut().handles.blobs.remove(&rep);
        Ok(())
    })?;
    types.func_wrap("[constructor]bytes", |mut store, (): ()| {
        let blob = Blob {
            value: Value::Bytes(Vec::new()),
            made: true,
        };
        match store.data_mut().handles.insert_blob(blob) {
            Ok(rep) => Ok((Resource::<Bytes>::new_own(rep),)),
            Err(err) => Err(wasmtime::format_err!("{err}")),
        }
    })?;
    types.func_wrap(
        "[method]bytes.size",
        |store, (this,): (Resource<Bytes>,)| {
            Ok((match &store.data().handles.blob(this.rep())?.value {
                Value::Bytes(bytes) => u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                Value::Lent { size, .. } => *size,
                _ => 0,
            },))
        },
    )?;
    types.func_wrap_async(
        "[method]bytes.read",
        |store, (this,): (Resource<Bytes>,)| {
            let read = store
                .data()
                .handles
                .blob(this.rep())
                .and_then(|blob| bytes_from(&store, &blob.value, 0, None));
            Box::new(async move { Ok((read?.await?.unwrap_or_default(),)) })
        },
    )?;
    types.func_wrap_async(
        "[method]bytes.range",
        |store, (this, offset, length): (Resource<Bytes>, u64, u64)| {
            let read = store
                .data()
                .handles
                .blob(this.rep())
                .and_then(|blob| bytes_from(&store, &blob.value, offset, Some(length)));
            Box::new(async move { Ok((read?.await?.unwrap_or_default(),)) })
        },
    )?;
    types.func_wrap(
        "[method]bytes.append",
        |mut store, (this, data): (Resource<Bytes>, WasmList<u8>)| {
            if !store.data().handles.blob(this.rep())?.made {
                wasmtime::bail!("`append` takes only a `bytes` the extension made");
            }
            let more = u64::try_from(data.len()).unwrap_or(u64::MAX);
            crate::call::grow(&**store.data().budget(), "an `append`", more)?;
            let handles = &mut store.data_mut().handles;
            handles.grown = handles.grown.saturating_add(more);
            let Some(Blob {
                value: Value::Bytes(buffer),
                ..
            }) = handles.blobs.get_mut(&this.rep())
            else {
                wasmtime::bail!("a `bytes` the extension made holds a byte string");
            };
            // Taken out while the guest's memory is borrowed, so the bytes are copied once.
            let mut buffer = std::mem::take(buffer);
            buffer.extend_from_slice(data.as_le_slice(&store));
            if let Some(blob) = store.data_mut().handles.blobs.get_mut(&this.rep()) {
                blob.value = Value::Bytes(buffer);
            }
            Ok(())
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

/// What `as-bytes` gives, for `length` `None`, or `range` gives: bytes copied out of a `bytes`
/// value at once, or a read of a lent source that the host serves while the guest waits, and
/// `None` for any other value. A read the host fails traps the call with its reason.
fn bytes_of(
    store: &StoreContextMut<'_, Guest>,
    this: &Resource<Handle>,
    offset: u64,
    length: Option<u64>,
) -> wasmtime::Result<impl Future<Output = wasmtime::Result<Option<Vec<u8>>>> + Send + 'static> {
    bytes_from(store, read(store, this)?, offset, length)
}

/// What `bytes_of` gives for the value `value`, which a `bytes` handle's `read` and `range` give too.
fn bytes_from(
    store: &StoreContextMut<'_, Guest>,
    value: &Value,
    offset: u64,
    length: Option<u64>,
) -> wasmtime::Result<impl Future<Output = wasmtime::Result<Option<Vec<u8>>>> + Send + 'static> {
    let budget = store.data().budget();
    let (copied, lent) = match value {
        Value::Bytes(bytes) => {
            if length.is_none() {
                let whole = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                crate::call::afford(&**budget, "a copy of the bytes", whole)?;
            }
            (
                Some(length.map_or_else(
                    || bytes.clone(),
                    |length| range(bytes, offset, length).to_vec(),
                )),
                None,
            )
        }
        Value::Lent { id, size } => {
            let most = u64::try_from(RANGE_MOST).unwrap_or(u64::MAX);
            let length = length.map_or(*size, |length| length.min(most));
            crate::call::afford(&**budget, "a read of the source", length)?;
            (None, Some(budget.read(*id, offset, length)))
        }
        _ => (None, None),
    };
    Ok(async move {
        match lent {
            Some(lending) => lending
                .await
                .map(Some)
                .map_err(|err| wasmtime::format_err!("{err}")),
            None => Ok(copied),
        }
    })
}

/// The part of `bytes` that `range` gives: at most [`RANGE_MOST`] bytes from `offset`, clipped at
/// the end.
fn range(bytes: &[u8], offset: u64, length: u64) -> &[u8] {
    let start = usize::try_from(offset).map_or(bytes.len(), |at| at.min(bytes.len()));
    let take = usize::try_from(length).map_or(RANGE_MOST, |length| length.min(RANGE_MOST));
    &bytes[start..start + take.min(bytes.len() - start)]
}

/// The owned handle a `key`, `element` or `field` returns, or a trap.
fn owned(rep: Result<Option<u32>, String>) -> wasmtime::Result<(Option<Resource<Handle>>,)> {
    match rep {
        Ok(rep) => Ok((rep.map(Resource::new_own),)),
        Err(err) => Err(wasmtime::format_err!("{err}")),
    }
}
