//! A `Core` value class as the record `wit/nvs-ext/types.wit` defines for it, both ways: the seam the
//! extension host crosses one through (`rule:packaging/a-value-crosses-as-its-wit-type`).
//!
//! The instance is this crate's to read and to allocate, so the host never sees a slot. [`parts`]
//! reads an instance into its record's fields, in the record's order, and [`built`] makes a fresh
//! instance from fields a guest returned. Each class's half lives beside its layout, in the module
//! that owns it.
//!
//! **A guest's fields are checked as a program's arguments would be.** [`built`] goes through the
//! class's own validation: a date no calendar has, a zone the database does not know, a URI that
//! does not parse and a key that does not read are refused, so a guest cannot make an instance no
//! `Core` member could have made. A `Core\Crypto\PublicKey` crosses as its SubjectPublicKeyInfo
//! alone, and its kind is read from it again: the first of `P256`, `X25519`, `Ed25519`,
//! `RsaPkcs1` and `RsaPss` that reads it, so an RSA key comes back as `RsaPkcs1`.

use nvs_runtime::Value;

/// Every `Core` value class that crosses. `nvs_ext::types::CORE_CLASSES` is the list with each
/// record's fields, and `tests/ext_world.rs` holds the two equal.
pub const CLASSES: &[&str] = &[
    "Core\\BigInt",
    "Core\\Uuid",
    "Core\\Uri",
    "Core\\Crypto\\PublicKey",
    "Core\\Time\\Instant",
    "Core\\Time\\DateTime",
    "Core\\Time\\Date",
    "Core\\Time\\TimeOfDay",
    "Core\\Time\\Duration",
    "Core\\Time\\Zone",
];

/// One field of a record, in the types a record field may have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    /// A `bool`.
    Bool(bool),
    /// An `s64`.
    Int(i64),
    /// A `u64`.
    Uint(u64),
    /// A `string`.
    String(String),
    /// A `list<u8>`.
    Bytes(Vec<u8>),
}

/// The class of `value` and its record's fields in order, or `None` where `value` is not an
/// instance of a `Core` value class that crosses.
///
/// # Errors
///
/// Where an instance's slots hold what its class never writes.
pub fn parts(value: Value) -> Option<Result<(&'static str, Vec<Part>), String>> {
    let class = value.class_name()?;
    let class = *CLASSES.iter().find(|crossing| **crossing == class)?;
    let read = match class {
        "Core\\Uuid" => crate::uuid::ext_parts(value),
        "Core\\BigInt" => crate::bigint::ext_parts(value),
        "Core\\Uri" => crate::uri::ext_parts(value),
        "Core\\Crypto\\PublicKey" => crate::crypto::ext_parts(value),
        _ => crate::time::ext_parts(class, value),
    };
    Some(
        read.map(|parts| (class, parts))
            .map_err(|_| format!("a `{class}` whose slots hold what it never writes")),
    )
}

/// A fresh instance of the `Core` value class `class` holding `parts`, its record's fields in
/// order, which the caller owns.
///
/// # Errors
///
/// Where `class` does not cross, `parts` is not its record, or the fields make no value of it.
pub fn built(class: &str, parts: Vec<Part>) -> Result<Value, String> {
    let refused = || format!("a `{class}` record whose fields make no `{class}`");
    match class {
        "Core\\Uuid" => crate::uuid::ext_built(&parts),
        "Core\\BigInt" => crate::bigint::ext_built(&parts),
        "Core\\Uri" => crate::uri::ext_built(&parts),
        "Core\\Crypto\\PublicKey" => crate::crypto::ext_built(&parts),
        _ if CLASSES.contains(&class) => crate::time::ext_built(class, &parts),
        _ => return Err(format!("a `{class}` does not cross")),
    }
    .ok_or_else(refused)
}
