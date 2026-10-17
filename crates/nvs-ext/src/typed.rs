//! A call to a built-in component through wasmtime's typed functions, so a `bytes` value crosses
//! as one copy each way (`rule:packaging/a-value-crosses-as-its-wit-type`).
//!
//! A built-in component's world is a file of this repository, so its Rust types are generated
//! here by `bindgen!`, one module per world, and its exports are called with
//! [`wasmtime::component::Func::typed`]. A typed call lowers a `Vec<u8>` into the guest's memory in
//! one copy and lifts one back the same way, where the dynamic path in `crate::convert` holds one
//! `Val` per byte. Every other extension stays on that dynamic path, because its types are only
//! known at run time and a typed call needs them when the host is compiled.
//!
//! The conversion from a [`Value`] to a generated type is directed by the manifest's
//! [`NovisType`], as `crate::convert`'s is, and takes the same values: an enum by its case or its
//! number, a `uint` from an `int` that is not negative, a list or a keyed array by its pairs in
//! order, a shape by its field names, and a closed union of shapes as the one case its keys fit.
//! A field, a case and a method are matched by name in kebab-case, so a generated `has_alpha` is
//! the manifest's `hasAlpha`. The conversion back writes what `crate::convert::from_wit` writes.
//!
//! **Handles cross as [`ResourceAny`].** A `mixed` value is lent and a resource the request keeps
//! is passed back by the call's [`Crossing`], and a resource the guest returns is kept by it, as on
//! the dynamic path. So the arguments are converted in the instance's store, once the call has
//! one, and the result is converted there too. wasmtime checks a [`ResourceAny`] against the
//! export's resource type when it lowers it.
//!
//! What it costs: a `bytes` value moves into its generated `Vec<u8>` and out of it again, so the
//! host copies no byte. wasmtime copies it once into the guest and once out of it. A call checks
//! the export's type against the generated one each time it runs.

use wasmtime::component::{Func, ResourceAny, Val};
use wasmtime::{AsContextMut, Store};

use crate::call::{Error, Guest};
use crate::convert::{self, Crossing, Key, Value};
use crate::types::{Field, NovisType};

/// A built-in world whose exports this module calls typed, by the interface its manifest maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum World {
    /// `nvs:image/codec`.
    Image,
    /// `nvs:pdf/engine`.
    Pdf,
    /// `nvs:intl/icu`.
    Intl,
    /// `nvs:spreadsheet/engine`.
    Spreadsheet,
}

impl World {
    /// The world `interface` is, where the component is a built-in one.
    pub(crate) fn of(interface: &str) -> Option<Self> {
        match interface {
            "nvs:image/codec" => Some(Self::Image),
            "nvs:pdf/engine" => Some(Self::Pdf),
            "nvs:intl/icu" => Some(Self::Intl),
            "nvs:spreadsheet/engine" => Some(Self::Spreadsheet),
            _ => None,
        }
    }

    /// Whether the world calls `method` typed. A method it does not takes the dynamic path.
    pub(crate) fn has(self, method: &str) -> bool {
        match self {
            Self::Image => image::Args::has(method),
            Self::Pdf => pdf::Args::has(method),
            Self::Intl => intl::Args::has(method),
            Self::Spreadsheet => spreadsheet::Args::has(method),
        }
    }

    /// The typed arguments of `method`, converted from `args` by `params`, with `call` lending
    /// and passing the handles among them.
    pub(crate) fn lower(
        self,
        method: &str,
        params: &[NovisType],
        args: Vec<Value>,
        call: &mut dyn Crossing,
    ) -> Result<Args, String> {
        Ok(match self {
            Self::Image => Args::Image(image::Args::lower(method, params, args, call)?),
            Self::Pdf => Args::Pdf(pdf::Args::lower(method, params, args, call)?),
            Self::Intl => Args::Intl(intl::Args::lower(method, params, args, call)?),
            Self::Spreadsheet => {
                Args::Spreadsheet(spreadsheet::Args::lower(method, params, args, call)?)
            }
        })
    }
}

/// One typed call's arguments.
pub(crate) enum Args {
    /// A call to the image world.
    Image(image::Args),
    /// A call to the PDF world.
    Pdf(pdf::Args),
    /// A call to the intl world.
    Intl(intl::Args),
    /// A call to the spreadsheet world.
    Spreadsheet(spreadsheet::Args),
}

/// What one typed call returned.
pub(crate) enum Out {
    /// A result of the image world.
    Image(image::Out),
    /// A result of the PDF world.
    Pdf(pdf::Out),
    /// A result of the intl world.
    Intl(intl::Out),
    /// A result of the spreadsheet world.
    Spreadsheet(spreadsheet::Out),
}

impl Args {
    /// The host bytes the arguments hold on the heap.
    pub(crate) fn heap(&self) -> u64 {
        match self {
            Self::Image(args) => args.heap(),
            Self::Pdf(args) => args.heap(),
            Self::Intl(args) => args.heap(),
            Self::Spreadsheet(args) => args.heap(),
        }
    }

    /// Calls `func` with the arguments. The outer `Err` is wasmtime's, a trap or a type that does
    /// not match, and the inner one is the export's own `err`.
    pub(crate) async fn call(
        self,
        store: &mut Store<Guest>,
        func: Func,
    ) -> wasmtime::Result<Result<Out, Error>> {
        match self {
            Self::Image(args) => Ok(args.call(store, func).await?.map(Out::Image)),
            Self::Pdf(args) => Ok(args.call(store, func).await?.map(Out::Pdf)),
            Self::Intl(args) => Ok(args.call(store, func).await?.map(Out::Intl)),
            Self::Spreadsheet(args) => Ok(args.call(store, func).await?.map(Out::Spreadsheet)),
        }
    }
}

impl Out {
    /// The host bytes the result holds on the heap.
    pub(crate) fn heap(&self) -> u64 {
        match self {
            Self::Image(out) => out.heap(),
            Self::Pdf(out) => out.heap(),
            Self::Intl(out) => out.heap(),
            Self::Spreadsheet(out) => out.heap(),
        }
    }

    /// The Novis value the result is, where the manifest's return type is `ty`, with `call`
    /// keeping a resource the guest returned.
    pub(crate) fn into_value(
        self,
        ty: &NovisType,
        call: &mut dyn Crossing,
    ) -> Result<Value, String> {
        match self {
            Self::Image(out) => out.into_value(ty, call),
            Self::Pdf(out) => out.into_value(ty, call),
            Self::Intl(out) => out.into_value(ty, call),
            Self::Spreadsheet(out) => out.into_value(ty, call),
        }
    }
}

/// A generated type a Novis value converts to. `call` lends a `mixed` value and passes a resource
/// back, as `crate::convert::to_wit_with` does.
pub(crate) trait FromValue: Sized {
    /// The value `value` is, where the manifest's type is `ty`.
    fn from_value(ty: &NovisType, value: Value, call: &mut dyn Crossing) -> Result<Self, String>;

    /// What an optional shape field that is absent converts to. Only an `Option` has a value here.
    fn absent() -> Option<Self> {
        None
    }

    /// The entry of a keyed array, where the manifest's types are `key` and `value`. Only a pair
    /// is one.
    fn from_entry(
        _: &NovisType,
        _: &NovisType,
        key: Key,
        _: Value,
        _: &mut dyn Crossing,
    ) -> Result<Self, String> {
        Err(format!(
            "the array entry {key:?} is not a pair of a keyed array"
        ))
    }

    /// `bytes` as a list of this type. Only `u8` has it, so `Vec<u8>` takes a `bytes` value.
    fn list_from_bytes(_: Vec<u8>) -> Option<Vec<Self>> {
        None
    }
}

/// A generated type that converts back to a Novis value. `call` keeps a resource the guest
/// returned, as `crate::convert::from_wit_with` does.
pub(crate) trait IntoValue: Sized {
    /// The Novis value `self` is, where the manifest's type is `ty`.
    fn into_value(self, ty: &NovisType, call: &mut dyn Crossing) -> Result<Value, String>;

    /// Whether an optional shape field of this value is left out of the array: an `Option` that
    /// is `None`.
    fn omitted(&self) -> bool {
        false
    }

    /// The key and the value of a keyed array's entry. Only a pair is one.
    fn into_entry(
        self,
        _: &NovisType,
        _: &NovisType,
        _: &mut dyn Crossing,
    ) -> Result<(Key, Value), String> {
        Err("only a pair is an entry of a keyed array".to_owned())
    }

    /// A list of this type as `bytes`. Only `u8` has it.
    fn list_into_bytes(list: Vec<Self>) -> Result<Vec<u8>, Vec<Self>> {
        Err(list)
    }
}

/// The host bytes a generated value holds on the heap, which a call charges to its request.
pub(crate) trait Heap {
    /// The bytes `self` holds on the heap. A scalar holds none.
    fn heap(&self) -> u64;
}

/// A generated record, which is a Novis shape.
pub(crate) trait Shape: Sized {
    /// The record the array of `entries` is, where the shape is `fields`.
    fn from_shape(
        fields: &[Field],
        entries: Vec<(Key, Value)>,
        call: &mut dyn Crossing,
    ) -> Result<Self, String>;

    /// The array the record is, where the shape is `fields`.
    fn into_shape(self, fields: &[Field], call: &mut dyn Crossing) -> Result<Value, String>;
}

/// The arguments of one method: a tuple of generated types, converted from the call's values.
pub(crate) trait Params: Sized {
    /// The arguments `args` are, where the parameters' types are `params`.
    fn from_values(
        params: &[NovisType],
        args: Vec<Value>,
        call: &mut dyn Crossing,
    ) -> Result<Self, String>;
}

macro_rules! scalars {
    ($($rust:ty: $novis:ident $value:ident;)*) => {$(
        impl FromValue for $rust {
            fn from_value(ty: &NovisType, value: Value, _: &mut dyn Crossing) -> Result<Self, String> {
                match (ty, value) {
                    (NovisType::$novis, Value::$value(v)) => Ok(v),
                    (ty, value) => Err(mismatch(ty, &value)),
                }
            }
        }

        impl IntoValue for $rust {
            fn into_value(self, ty: &NovisType, _: &mut dyn Crossing) -> Result<Value, String> {
                match ty {
                    NovisType::$novis => Ok(Value::$value(self)),
                    ty => Err(back(ty, stringify!($rust))),
                }
            }
        }

        impl Heap for $rust {
            fn heap(&self) -> u64 {
                0
            }
        }
    )*};
}

scalars! {
    bool: Bool Bool;
    i64: Int Int;
    f64: Float Float;
}

impl FromValue for u64 {
    fn from_value(ty: &NovisType, value: Value, _: &mut dyn Crossing) -> Result<Self, String> {
        match (ty, value) {
            (NovisType::Uint, Value::Uint(n)) => Ok(n),
            (NovisType::Uint, Value::Int(n)) if let Ok(n) = u64::try_from(n) => Ok(n),
            (ty, value) => Err(mismatch(ty, &value)),
        }
    }
}

impl IntoValue for u64 {
    fn into_value(self, ty: &NovisType, _: &mut dyn Crossing) -> Result<Value, String> {
        match ty {
            NovisType::Uint => Ok(Value::Uint(self)),
            ty => Err(back(ty, "u64")),
        }
    }
}

impl Heap for u64 {
    fn heap(&self) -> u64 {
        0
    }
}

impl FromValue for String {
    fn from_value(ty: &NovisType, value: Value, _: &mut dyn Crossing) -> Result<Self, String> {
        match (ty, value) {
            (NovisType::String, Value::String(s)) => Ok(s),
            (ty, value) => Err(mismatch(ty, &value)),
        }
    }
}

impl IntoValue for String {
    fn into_value(self, ty: &NovisType, _: &mut dyn Crossing) -> Result<Value, String> {
        match ty {
            NovisType::String => Ok(Value::String(self)),
            ty => Err(back(ty, "String")),
        }
    }
}

impl Heap for String {
    fn heap(&self) -> u64 {
        self.len() as u64
    }
}

impl FromValue for u8 {
    fn from_value(ty: &NovisType, value: Value, _: &mut dyn Crossing) -> Result<Self, String> {
        Err(mismatch(ty, &value))
    }

    fn list_from_bytes(bytes: Vec<u8>) -> Option<Vec<Self>> {
        Some(bytes)
    }
}

impl IntoValue for u8 {
    fn into_value(self, ty: &NovisType, _: &mut dyn Crossing) -> Result<Value, String> {
        Err(back(ty, "u8"))
    }

    fn list_into_bytes(list: Vec<Self>) -> Result<Vec<u8>, Vec<Self>> {
        Ok(list)
    }
}

impl Heap for u8 {
    fn heap(&self) -> u64 {
        0
    }
}

impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(ty: &NovisType, value: Value, call: &mut dyn Crossing) -> Result<Self, String> {
        match (ty, value) {
            (NovisType::Bytes, Value::Bytes(bytes)) => {
                T::list_from_bytes(bytes).ok_or_else(|| back(ty, "a list that is not bytes"))
            }
            (NovisType::List(item), Value::Array(entries)) => entries
                .into_iter()
                .map(|(_, value)| T::from_value(item, value, call))
                .collect(),
            (NovisType::Keyed(key, value), Value::Array(entries)) => entries
                .into_iter()
                .map(|(k, v)| T::from_entry(key, value, k, v, call))
                .collect(),
            (ty, value) => Err(mismatch(ty, &value)),
        }
    }
}

impl<T: IntoValue> IntoValue for Vec<T> {
    fn into_value(self, ty: &NovisType, call: &mut dyn Crossing) -> Result<Value, String> {
        match ty {
            NovisType::Bytes => T::list_into_bytes(self)
                .map(Value::Bytes)
                .map_err(|_| back(ty, "a list that is not bytes")),
            NovisType::List(item) => Ok(Value::Array(
                (0_i64..)
                    .zip(self)
                    .map(|(at, value)| Ok((Key::Int(at), value.into_value(item, call)?)))
                    .collect::<Result<_, String>>()?,
            )),
            NovisType::Keyed(key, value) => Ok(Value::Array(
                self.into_iter()
                    .map(|entry| entry.into_entry(key, value, call))
                    .collect::<Result<_, String>>()?,
            )),
            ty => Err(back(ty, "a list")),
        }
    }
}

impl<T: Heap> Heap for Vec<T> {
    fn heap(&self) -> u64 {
        (self.len() * size_of::<T>()) as u64 + self.iter().map(Heap::heap).sum::<u64>()
    }
}

/// An `Option` is a `?T` or an optional shape field, so its type is the `?T`'s inner one or, for a
/// field, the field's own.
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(ty: &NovisType, value: Value, call: &mut dyn Crossing) -> Result<Self, String> {
        let inner = match ty {
            NovisType::Optional(inner) => inner,
            ty => ty,
        };
        match value {
            Value::Null => Ok(None),
            value => T::from_value(inner, value, call).map(Some),
        }
    }

    fn absent() -> Option<Self> {
        Some(None)
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self, ty: &NovisType, call: &mut dyn Crossing) -> Result<Value, String> {
        let inner = match ty {
            NovisType::Optional(inner) => inner,
            ty => ty,
        };
        match self {
            None => Ok(Value::Null),
            Some(value) => value.into_value(inner, call),
        }
    }

    fn omitted(&self) -> bool {
        self.is_none()
    }
}

impl<T: Heap> Heap for Option<T> {
    fn heap(&self) -> u64 {
        self.as_ref().map_or(0, Heap::heap)
    }
}

/// A pair is one entry of a keyed array. A key the runtime stored as an `int` is its decimal text
/// where the key type is `string`, as `crate::convert` crosses it.
impl<K: FromValue, V: FromValue> FromValue for (K, V) {
    fn from_value(ty: &NovisType, value: Value, _: &mut dyn Crossing) -> Result<Self, String> {
        Err(mismatch(ty, &value))
    }

    fn from_entry(
        key: &NovisType,
        value: &NovisType,
        k: Key,
        v: Value,
        call: &mut dyn Crossing,
    ) -> Result<Self, String> {
        let k = match (key, k) {
            (NovisType::String, Key::Int(n)) => Value::String(n.to_string()),
            (_, Key::Int(n)) => Value::Int(n),
            (_, Key::String(s)) => Value::String(s),
        };
        Ok((K::from_value(key, k, call)?, V::from_value(value, v, call)?))
    }
}

impl<K: IntoValue, V: IntoValue> IntoValue for (K, V) {
    fn into_value(self, ty: &NovisType, _: &mut dyn Crossing) -> Result<Value, String> {
        Err(back(ty, "a pair"))
    }

    fn into_entry(
        self,
        key: &NovisType,
        value: &NovisType,
        call: &mut dyn Crossing,
    ) -> Result<(Key, Value), String> {
        let k = match self.0.into_value(key, call)? {
            Value::Int(n) => Key::Int(n),
            Value::String(s) => Key::String(s),
            other => {
                return Err(format!(
                    "an array key is an `int` or a `string`, and the guest returned {other:?}"
                ));
            }
        };
        Ok((k, self.1.into_value(value, call)?))
    }
}

/// A handle: a `mixed` value the call lends, or a resource the request keeps, which the call
/// passes back. Either crosses as the `ResourceAny` the dynamic path would have sent, so wasmtime
/// checks it against the export's own resource type when it lowers it.
impl FromValue for ResourceAny {
    fn from_value(ty: &NovisType, value: Value, call: &mut dyn Crossing) -> Result<Self, String> {
        let val = match (ty, value) {
            (NovisType::Mixed, value) => call.lend(value)?,
            (NovisType::Resource(name), Value::Resource { name: of, id }) if *name == of => {
                call.pass(name, id)?
            }
            (ty, value) => return Err(mismatch(ty, &value)),
        };
        match val {
            Val::Resource(resource) => Ok(resource),
            val => Err(format!("the call crossed {val:?}, which is not a handle")),
        }
    }
}

/// A resource the guest returned as an `own`, which the request keeps until it ends.
impl IntoValue for ResourceAny {
    fn into_value(self, ty: &NovisType, call: &mut dyn Crossing) -> Result<Value, String> {
        match ty {
            NovisType::Resource(name) if self.owned() => call.keep(name, self),
            ty => Err(back(ty, "resource")),
        }
    }
}

impl Heap for ResourceAny {
    fn heap(&self) -> u64 {
        0
    }
}

/// The `ok` side of a `result<_, error>`, which a `void` method returns.
impl IntoValue for () {
    fn into_value(self, _: &NovisType, _: &mut dyn Crossing) -> Result<Value, String> {
        Ok(Value::Null)
    }
}

impl Heap for () {
    fn heap(&self) -> u64 {
        0
    }
}

macro_rules! params {
    ($($n:literal: $($t:ident),*;)*) => {$(
        impl<$($t: FromValue + Heap),*> Params for ($($t,)*) {
            fn from_values(
                params: &[NovisType],
                args: Vec<Value>,
                call: &mut dyn Crossing,
            ) -> Result<Self, String> {
                if params.len() != $n || args.len() != $n {
                    return Err(format!(
                        "the method takes {} arguments, and the call passed {}",
                        params.len(),
                        args.len()
                    ));
                }
                let mut params = params.iter();
                let mut args = args.into_iter();
                Ok(($(next::<$t>(&mut params, &mut args, call)?,)*))
            }
        }

        impl<$($t: Heap),*> Heap for ($($t,)*) {
            #[allow(non_snake_case, reason = "each element is named by its type parameter")]
            fn heap(&self) -> u64 {
                let ($($t,)*) = self;
                0 $(+ $t.heap())*
            }
        }
    )*};
}

params! {
    1: A;
    2: A, B;
    3: A, B, C;
    4: A, B, C, D;
}

/// The next argument, converted by the next parameter's type.
fn next<T: FromValue>(
    params: &mut std::slice::Iter<'_, NovisType>,
    args: &mut std::vec::IntoIter<Value>,
    call: &mut dyn Crossing,
) -> Result<T, String> {
    match (params.next(), args.next()) {
        (Some(ty), Some(value)) => T::from_value(ty, value, call),
        _ => Err("the call passed fewer arguments than the method takes".to_owned()),
    }
}

/// The value of the next field of a record, where `field` is the shape's field at the same
/// position and `rust` the generated field's name.
pub(crate) fn field<T: FromValue>(
    field: Option<&Field>,
    entries: &mut Vec<(Key, Value)>,
    rust: &str,
    call: &mut dyn Crossing,
) -> Result<T, String> {
    let (name, optional, ty) = shape_field(field, rust)?;
    let found = entries
        .iter()
        .position(|(key, _)| matches!(key, Key::String(k) if k == name))
        .map(|at| entries.swap_remove(at).1);
    match found {
        Some(value) => T::from_value(ty, value, call),
        None if *optional => {
            T::absent().ok_or_else(|| format!("the optional field `{name}` cannot be absent"))
        }
        None => Err(format!(
            "the shape has no field `{name}`, which is required"
        )),
    }
}

/// Writes the record field `value` into `entries`, where `field` is the shape's field at the same
/// position and `rust` the generated field's name. An optional field that is `None` is left out.
pub(crate) fn put<T: IntoValue>(
    field: Option<&Field>,
    entries: &mut Vec<(Key, Value)>,
    rust: &str,
    value: T,
    call: &mut dyn Crossing,
) -> Result<(), String> {
    let (name, optional, ty) = shape_field(field, rust)?;
    if *optional && value.omitted() {
        return Ok(());
    }
    entries.push((Key::String(name.clone()), value.into_value(ty, call)?));
    Ok(())
}

/// The shape's field `field`, checked to be the generated field `rust`. The loader has already
/// checked that a shape's fields are its record's, in the same order.
fn shape_field<'f>(field: Option<&'f Field>, rust: &str) -> Result<&'f Field, String> {
    let rust = rust.trim_end_matches('_');
    match field {
        Some(field) if same_name(&field.0, rust) => Ok(field),
        Some((name, ..)) => Err(format!(
            "the shape's field `{name}` is where the record has `{rust}`"
        )),
        None => Err(format!("the shape has no field for the record's `{rust}`")),
    }
}

/// The value of the next field of a `Core` value class's record, where `field` is the record's
/// field at the same position and `rust` the generated field's name.
pub(crate) fn core_field<T: FromValue>(
    field: Option<&(&'static str, NovisType)>,
    class: &str,
    fields: &mut Vec<(String, Value)>,
    rust: &str,
    call: &mut dyn Crossing,
) -> Result<T, String> {
    let (name, ty) = core_record_field(field, rust)?;
    match fields.iter().position(|(n, _)| n == name) {
        Some(at) => T::from_value(ty, fields.swap_remove(at).1, call),
        None => Err(format!("the `{class}` has no field `{name}`")),
    }
}

/// The named field of a `Core` value class the generated field `value` is, where `field` is the
/// record's field at the same position and `rust` the generated field's name.
pub(crate) fn core_put<T: IntoValue>(
    field: Option<&(&'static str, NovisType)>,
    rust: &str,
    value: T,
    call: &mut dyn Crossing,
) -> Result<(String, Value), String> {
    let (name, ty) = core_record_field(field, rust)?;
    Ok(((*name).to_owned(), value.into_value(ty, call)?))
}

/// The record's field `field`, checked to be the generated field `rust`.
fn core_record_field<'f>(
    field: Option<&'f (&'static str, NovisType)>,
    rust: &str,
) -> Result<&'f (&'static str, NovisType), String> {
    let rust = rust.trim_end_matches('_');
    match field {
        Some(field) if same_name(field.0, rust) => Ok(field),
        Some((name, _)) => Err(format!(
            "the class's field `{name}` is where the record has `{rust}`"
        )),
        None => Err(format!("the class has no field for the record's `{rust}`")),
    }
}

/// The Novis case of the enum `ty` that `value` is: a case by its name, or by its number.
pub(crate) fn enum_case<'t>(ty: &'t NovisType, value: &Value) -> Result<&'t str, String> {
    let NovisType::Enum { name, cases } = ty else {
        return Err(mismatch(ty, value));
    };
    let found = match value {
        Value::Case(case) => cases.iter().find(|of| *of == case),
        Value::Int(number) => usize::try_from(*number).ok().and_then(|at| cases.get(at)),
        _ => None,
    };
    found
        .map(String::as_str)
        .ok_or_else(|| format!("{value:?} is not a case of the enum `{name}`"))
}

/// The Novis value of the generated enum case `rust`, where the manifest's type is `ty`.
pub(crate) fn enum_value(ty: &NovisType, rust: &str) -> Result<Value, String> {
    match ty {
        NovisType::Enum { cases, .. } => cases
            .iter()
            .find(|case| same_name(case, rust))
            .map(|case| Value::Case(case.clone()))
            .ok_or_else(|| back(ty, rust)),
        ty => Err(back(ty, rust)),
    }
}

/// The case of the closed union `ty` the array of `entries` is a value of, and its shape.
pub(crate) fn union_case<'t>(
    ty: &'t NovisType,
    entries: &[(Key, Value)],
) -> Result<(&'t str, &'t [Field]), String> {
    let NovisType::Union { name, cases } = ty else {
        return Err(format!("a shape where the type is {ty:?}"));
    };
    cases
        .iter()
        .find(|(_, fields)| convert::fits(fields, entries))
        .map(|(case, fields)| (case.as_str(), fields.as_slice()))
        .ok_or_else(|| format!("the array is a value of no case of `{name}`"))
}

/// The shape of the case `rust` of the closed union `ty`.
pub(crate) fn union_shape<'t>(ty: &'t NovisType, rust: &str) -> Result<&'t [Field], String> {
    match ty {
        NovisType::Union { cases, .. } => cases
            .iter()
            .find(|(case, _)| same_name(case, rust))
            .map(|(_, fields)| fields.as_slice())
            .ok_or_else(|| back(ty, rust)),
        ty => Err(back(ty, rust)),
    }
}

/// Whether the names `a` and `b` are one WIT name, as `crate::kebab` writes both.
pub(crate) fn same_name(a: &str, b: &str) -> bool {
    kebab(a).eq(kebab(b))
}

/// The characters of `crate::kebab(name)`, without allocating.
fn kebab(name: &str) -> impl Iterator<Item = char> + '_ {
    let mut dash = true;
    name.chars().flat_map(move |c| {
        let out = if c == '_' {
            [Some('-'), None]
        } else if c.is_ascii_uppercase() {
            [(!dash).then_some('-'), Some(c.to_ascii_lowercase())]
        } else {
            [Some(c), None]
        };
        dash = c == '_' || c == '-';
        out.into_iter().flatten()
    })
}

/// The error for a Novis value that is not one of the type `ty`.
pub(crate) fn mismatch(ty: &NovisType, value: &Value) -> String {
    format!("{value:?} is not a value of the type {ty:?}")
}

/// The error for a result the guest returned that the type `ty` does not describe.
pub(crate) fn back(ty: &NovisType, what: &str) -> String {
    format!("the guest's {what} is not one the type {ty:?} crosses as")
}

/// The typed function `func` is, for the parameters `P` and the result `R`, called with `args` in
/// `store`.
pub(crate) async fn call<P, R>(store: &mut Store<Guest>, func: Func, args: P) -> wasmtime::Result<R>
where
    P: wasmtime::component::ComponentNamedList + wasmtime::component::Lower + Send + Sync,
    R: wasmtime::component::ComponentNamedList + wasmtime::component::Lift + Send + Sync + 'static,
{
    let typed = func.typed::<P, R>(&*store)?;
    typed.call_async(store.as_context_mut(), args).await
}

/// Converts generated records, each written with its fields in the WIT's order, to and from Novis
/// shapes.
macro_rules! records {
    ($($ty:ident { $($field:ident),* $(,)? })*) => {$(
        impl $crate::typed::Shape for $ty {
            fn from_shape(
                fields: &[$crate::types::Field],
                mut entries: Vec<($crate::convert::Key, $crate::convert::Value)>,
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<Self, String> {
                let mut at = fields.iter();
                Ok(Self {
                    $($field: $crate::typed::field(
                        at.next(), &mut entries, stringify!($field), call,
                    )?,)*
                })
            }

            fn into_shape(
                self,
                fields: &[$crate::types::Field],
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<$crate::convert::Value, String> {
                let mut at = fields.iter();
                let mut entries = Vec::with_capacity(fields.len());
                $($crate::typed::put(at.next(), &mut entries, stringify!($field), self.$field, call)?;)*
                Ok($crate::convert::Value::Array(entries))
            }
        }

        impl $crate::typed::FromValue for $ty {
            fn from_value(
                ty: &$crate::types::NovisType,
                value: $crate::convert::Value,
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<Self, String> {
                match (ty, value) {
                    ($crate::types::NovisType::Shape(fields), $crate::convert::Value::Array(entries)) => {
                        $crate::typed::Shape::from_shape(fields, entries, call)
                    }
                    (ty, value) => Err($crate::typed::mismatch(ty, &value)),
                }
            }
        }

        impl $crate::typed::IntoValue for $ty {
            fn into_value(
                self,
                ty: &$crate::types::NovisType,
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<$crate::convert::Value, String> {
                match ty {
                    $crate::types::NovisType::Shape(fields) => {
                        $crate::typed::Shape::into_shape(self, fields, call)
                    }
                    ty => Err($crate::typed::back(ty, stringify!($ty))),
                }
            }
        }

        impl $crate::typed::Heap for $ty {
            fn heap(&self) -> u64 {
                0 $(+ $crate::typed::Heap::heap(&self.$field))*
            }
        }
    )*};
}

/// Converts generated enums to and from a Novis enum's cases.
macro_rules! enums {
    ($($ty:ident [$($case:ident),* $(,)?])*) => {$(
        impl $crate::typed::FromValue for $ty {
            fn from_value(
                ty: &$crate::types::NovisType,
                value: $crate::convert::Value,
                _: &mut dyn $crate::convert::Crossing,
            ) -> Result<Self, String> {
                let case = $crate::typed::enum_case(ty, &value)?;
                $(if $crate::typed::same_name(case, stringify!($case)) {
                    return Ok(Self::$case);
                })*
                Err($crate::typed::mismatch(ty, &value))
            }
        }

        impl $crate::typed::IntoValue for $ty {
            fn into_value(
                self,
                ty: &$crate::types::NovisType,
                _: &mut dyn $crate::convert::Crossing,
            ) -> Result<$crate::convert::Value, String> {
                $crate::typed::enum_value(ty, match self {
                    $(Self::$case => stringify!($case),)*
                })
            }
        }

        impl $crate::typed::Heap for $ty {
            fn heap(&self) -> u64 {
                0
            }
        }
    )*};
}

/// Converts generated variants, each of whose cases carries a record, to and from a Novis closed
/// union of shapes.
macro_rules! variants {
    ($($ty:ident [$($case:ident),* $(,)?])*) => {$(
        impl $crate::typed::FromValue for $ty {
            fn from_value(
                ty: &$crate::types::NovisType,
                value: $crate::convert::Value,
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<Self, String> {
                let entries = match value {
                    $crate::convert::Value::Array(entries) => entries,
                    value => return Err($crate::typed::mismatch(ty, &value)),
                };
                let (case, fields) = $crate::typed::union_case(ty, &entries)?;
                $(if $crate::typed::same_name(case, stringify!($case)) {
                    return Ok(Self::$case($crate::typed::Shape::from_shape(fields, entries, call)?));
                })*
                Err(format!("`{case}` is not a case of the record's variant"))
            }
        }

        impl $crate::typed::IntoValue for $ty {
            fn into_value(
                self,
                ty: &$crate::types::NovisType,
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<$crate::convert::Value, String> {
                match self {
                    $(Self::$case(payload) => $crate::typed::Shape::into_shape(
                        payload,
                        $crate::typed::union_shape(ty, stringify!($case))?,
                        call,
                    ),)*
                }
            }
        }

        impl $crate::typed::Heap for $ty {
            fn heap(&self) -> u64 {
                match self {
                    $(Self::$case(payload) => $crate::typed::Heap::heap(payload),)*
                }
            }
        }
    )*};
}

/// Converts the generated records of `nvs:ext/types`, each written with its fields in the WIT's
/// order, to and from the `Core` value class that crosses as it.
macro_rules! cores {
    ($($ty:ident { $($field:ident),* $(,)? })*) => {$(
        impl $crate::typed::FromValue for $ty {
            fn from_value(
                ty: &$crate::types::NovisType,
                value: $crate::convert::Value,
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<Self, String> {
                match (ty, value) {
                    (
                        $crate::types::NovisType::Core(core),
                        $crate::convert::Value::Core { class, mut fields },
                    ) if class == core.class => {
                        let mut at = core.fields.iter();
                        Ok(Self {
                            $($field: $crate::typed::core_field(
                                at.next(), &class, &mut fields, stringify!($field), call,
                            )?,)*
                        })
                    }
                    (ty, value) => Err($crate::typed::mismatch(ty, &value)),
                }
            }
        }

        impl $crate::typed::IntoValue for $ty {
            fn into_value(
                self,
                ty: &$crate::types::NovisType,
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<$crate::convert::Value, String> {
                let $crate::types::NovisType::Core(core) = ty else {
                    return Err($crate::typed::back(ty, stringify!($ty)));
                };
                let mut at = core.fields.iter();
                let mut fields = Vec::with_capacity(core.fields.len());
                $(fields.push($crate::typed::core_put(
                    at.next(), stringify!($field), self.$field, call,
                )?);)*
                Ok($crate::convert::Value::Core { class: core.class.to_owned(), fields })
            }
        }

        impl $crate::typed::Heap for $ty {
            fn heap(&self) -> u64 {
                0 $(+ $crate::typed::Heap::heap(&self.$field))*
            }
        }
    )*};
}

/// A world's typed calls: [`Args`] and [`Out`] with one case per method, named as the method is,
/// each with its generated parameter types and its `ok` type. `error` is the world's generated
/// `error` variant, which converts to [`Error`].
macro_rules! world {
    (error: $error:ty; $($method:ident($($param:ty),*) -> $ret:ty;)*) => {
        /// One call's typed arguments, by the method.
        #[allow(non_camel_case_types, reason = "each case is named as its method is")]
        pub(crate) enum Args {
            $($method(($($param,)*)),)*
        }

        /// What one call returned, by the method.
        #[allow(non_camel_case_types, reason = "each case is named as its method is")]
        pub(crate) enum Out {
            $($method($ret),)*
        }

        impl Args {
            /// Whether the world calls `method` typed.
            pub(crate) fn has(method: &str) -> bool {
                false $(|| $crate::typed::same_name(method, stringify!($method)))*
            }

            /// The typed arguments of `method`, converted from `args` by `params`, with `call`
            /// lending and passing the handles among them.
            pub(crate) fn lower(
                method: &str,
                params: &[$crate::types::NovisType],
                args: Vec<$crate::convert::Value>,
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<Self, String> {
                $(if $crate::typed::same_name(method, stringify!($method)) {
                    return <($($param,)*) as $crate::typed::Params>::from_values(params, args, call)
                        .map(Self::$method);
                })*
                Err(format!("the world calls no method `{method}` typed"))
            }

            /// The host bytes the arguments hold on the heap.
            pub(crate) fn heap(&self) -> u64 {
                match self {
                    $(Self::$method(args) => $crate::typed::Heap::heap(args),)*
                }
            }

            /// Calls `func`, the export of the method, with the arguments.
            pub(crate) async fn call(
                self,
                store: &mut wasmtime::Store<$crate::call::Guest>,
                func: wasmtime::component::Func,
            ) -> wasmtime::Result<Result<Out, $crate::call::Error>> {
                match self {
                    $(Self::$method(args) => {
                        let (result,) = $crate::typed::call::<($($param,)*), (Result<$ret, $error>,)>(
                            store, func, args,
                        )
                        .await?;
                        Ok(result.map(Out::$method).map_err($crate::call::Error::from))
                    })*
                }
            }
        }

        impl Out {
            /// The host bytes the result holds on the heap.
            pub(crate) fn heap(&self) -> u64 {
                match self {
                    $(Self::$method(out) => $crate::typed::Heap::heap(out),)*
                }
            }

            /// The Novis value the result is, where the manifest's return type is `ty`, with
            /// `call` keeping a resource the guest returned.
            pub(crate) fn into_value(
                self,
                ty: &$crate::types::NovisType,
                call: &mut dyn $crate::convert::Crossing,
            ) -> Result<$crate::convert::Value, String> {
                match self {
                    $(Self::$method(out) => $crate::typed::IntoValue::into_value(out, ty, call),)*
                }
            }
        }
    };
}

mod image;
mod intl;
mod pdf;
mod spreadsheet;
