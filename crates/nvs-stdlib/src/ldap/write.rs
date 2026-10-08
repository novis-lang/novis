//! `Core\Ldap`'s writing half: `Ldap\Connection`'s `add`, `modify`, `delete`, `rename`, `setPassword`, `changePassword` and `compare`, and the `Ldap\Change` values `modify` takes
//!
//! ADR 0278 §§ 1 and 9. [`modify`] sends one Modify request, so the server
//! applies every change in order or none of them. [`rename`] reads both DNs
//! and leaves [`nvs_ldap::Connection::rename`] to send the new first level,
//! and the new parent only where it differs. [`set_password`] and
//! [`change_password`] write AD's `unicodePwd` and are refused before
//! anything is sent on a connection without TLS, whatever the cleartext
//! grant says. [`compare`] lives here because it sends one value in the
//! form a write does.
//!
//! **An `Ldap\Change` is a value**: its kind, its attribute and the `mixed`
//! value it was given, unencoded. The value is encoded when `modify` sends it,
//! because an `Instant` takes one of two forms, and only the connection's
//! schema says which.
//!
//! **A value is written in the form it is read**
//! (`rule:core-classes/ldap-value-types`), so what a typed reader on
//! `Ldap\Entry` returns can be written back unchanged. [`encoded`] is the
//! table: a `string` or `bytes` as it is, an `int` in decimal, a `bool` as
//! `TRUE`/`FALSE`, a `Core\Uuid` in AD's GUID byte order, an `Ldap\Sid` in
//! its binary form, a `Dn` as its text, an `Instant` as a GeneralizedTime
//! where the schema gives the attribute that syntax and as a FILETIME
//! otherwise, a `Duration` as AD's negative interval, a flag object as its
//! 32 bits, and an `Ad\AccountType` case as its number. A list is one value
//! per element. Anything else is the program's own mistake, a `LogicError`.

use nvs_runtime::{Ctx, Fault, NvsStr, Tag, ThrownClass, Value};

use super::search::retained;
use super::{
    ACCOUNT_FLAGS, CHANGE, CHANGE_ATTRIBUTE_AT, CHANGE_KIND_AT, CHANGE_NAME, CHANGE_VALUE_AT,
    CONNECTION, CONNECTION_HANDLE_AT, DN, DN_TEXT_AT, FLAGS_BITS_AT, GROUP_TYPE, SID, SID_BYTES_AT,
    fault_of, held,
};

/// `Core\Ldap\Connection::add`, as its errors spell it.
pub const ADD: &str = r"Core\Ldap\Connection::add";
/// `Core\Ldap\Connection::modify`, as its errors spell it.
pub const MODIFY: &str = r"Core\Ldap\Connection::modify";
/// `Core\Ldap\Connection::delete`, as its errors spell it.
pub const DELETE: &str = r"Core\Ldap\Connection::delete";
/// `Core\Ldap\Connection::rename`, as its errors spell it.
pub const RENAME: &str = r"Core\Ldap\Connection::rename";
/// `Core\Ldap\Connection::setPassword`, as its errors spell it.
pub const SET_PASSWORD: &str = r"Core\Ldap\Connection::setPassword";
/// `Core\Ldap\Connection::changePassword`, as its errors spell it.
pub const CHANGE_PASSWORD: &str = r"Core\Ldap\Connection::changePassword";
/// `Core\Ldap\Connection::compare`, as its errors spell it.
pub const COMPARE: &str = r"Core\Ldap\Connection::compare";

/// `Core\Ldap\Connection::modify`'s body: `changes` applied to the entry at
/// `dn` in one request.
///
/// # Errors
///
/// [`held`]'s, and every failure the server reports, after which no change
/// was applied.
pub fn modify(
    ctx: &mut Ctx,
    key: u64,
    dn: &str,
    changes: &[nvs_ldap::Change],
) -> Result<(), Fault> {
    held(ctx, key, MODIFY)?
        .ready()
        .modify(dn, changes)
        .map_err(|error| fault_of(MODIFY, &error))
}

/// `Core\Ldap\Connection::add`'s body: a new entry at `dn` with `attributes`.
///
/// # Errors
///
/// [`held`]'s, and every failure the server reports.
pub fn add(
    ctx: &mut Ctx,
    key: u64,
    dn: &str,
    attributes: &[nvs_ldap::Attribute],
) -> Result<(), Fault> {
    held(ctx, key, ADD)?
        .ready()
        .add(dn, attributes)
        .map_err(|error| fault_of(ADD, &error))
}

/// `Core\Ldap\Connection::delete`'s body: the entry at `dn` deleted.
///
/// # Errors
///
/// [`held`]'s, and every failure the server reports.
pub fn delete(ctx: &mut Ctx, key: u64, dn: &str) -> Result<(), Fault> {
    held(ctx, key, DELETE)?
        .ready()
        .delete(dn)
        .map_err(|error| fault_of(DELETE, &error))
}

/// `Core\Ldap\Connection::rename`'s body: the entry at `from` renamed, and
/// moved where `to` has another parent.
///
/// # Errors
///
/// A `LogicError` for text that is not a DN, [`held`]'s, and every failure
/// the server reports.
pub fn rename(ctx: &mut Ctx, key: u64, from: &str, to: &str) -> Result<(), Fault> {
    let parsed = |text: &str| {
        nvs_ldap::Dn::parse(text).map_err(|error| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("{RENAME}: `{text}` is not a DN: {error}"),
            )
        })
    };
    let (from, to) = (parsed(from)?, parsed(to)?);
    held(ctx, key, RENAME)?
        .ready()
        .rename(&from, &to)
        .map_err(|error| fault_of(RENAME, &error))
}

/// `Core\Ldap\Connection::setPassword`'s body: the password of the account
/// at `dn` reset to `password`.
///
/// # Errors
///
/// [`held`]'s, `EncryptionRequired` on a connection without TLS before
/// anything is sent, `PasswordPolicy` for a password the domain does not
/// accept, and every other failure the server reports.
pub fn set_password(ctx: &mut Ctx, key: u64, dn: &str, password: &str) -> Result<(), Fault> {
    held(ctx, key, SET_PASSWORD)?
        .ready()
        .set_password(dn, password)
        .map_err(|error| fault_of(SET_PASSWORD, &error))
}

/// `Core\Ldap\Connection::changePassword`'s body: the password of the
/// account at `dn` changed from `old` to `new` in one request.
///
/// # Errors
///
/// As [`set_password`].
pub fn change_password(
    ctx: &mut Ctx,
    key: u64,
    dn: &str,
    old: &str,
    new: &str,
) -> Result<(), Fault> {
    held(ctx, key, CHANGE_PASSWORD)?
        .ready()
        .change_password(dn, old, new)
        .map_err(|error| fault_of(CHANGE_PASSWORD, &error))
}

/// `Core\Ldap\Connection::compare`'s body: whether the entry at `dn` has
/// `value` under `attribute`, as the server's matching rule for the
/// attribute decides.
///
/// # Errors
///
/// [`held`]'s, and every failure the server reports, such as
/// `NoSuchObject` for an entry that does not exist.
pub fn compare(
    ctx: &mut Ctx,
    key: u64,
    dn: &str,
    attribute: &str,
    value: &[u8],
) -> Result<bool, Fault> {
    held(ctx, key, COMPARE)?
        .ready()
        .compare(dn, attribute, value)
        .map_err(|error| fault_of(COMPARE, &error))
}

/// The error for a value [`encoded`] has no form for.
fn not_writable(member: &str, attribute: &str, what: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{member}: a value of `{attribute}` is {what}, which LDAP cannot store. Give a \
             `string`, `bytes`, `int`, `bool`, `Uuid`, `Sid`, `Dn`, `Instant`, `Duration` or \
             flag object, or a list of them"
        ),
    )
}

/// The values `value` writes under `attribute`, one per element of a list,
/// in the form a reader on `Ldap\Entry` reads back. `null` writes none.
///
/// # Errors
///
/// A `LogicError` for a value with no LDAP form, and [`super::schema`]'s for
/// an `Instant`, whose form the schema chooses.
pub(super) fn encoded(
    ctx: &mut Ctx,
    key: u64,
    attribute: &str,
    value: Value,
    member: &str,
) -> Result<Vec<Vec<u8>>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(Vec::new());
    }
    let Some(array) = value.array_ptr() else {
        return Ok(vec![one_encoded(ctx, key, attribute, value, member)?]);
    };
    let list = crate::arr::borrowed(array);
    let mut out = Vec::with_capacity(list.count());
    let mut at = list.next_slot(0);
    while let Some(slot) = at {
        let element = list.value_at(slot).unwrap_or_else(Value::null);
        if element.array_ptr().is_some() || matches!(element.tag(), Some(Tag::Null)) {
            let what = if element.array_ptr().is_some() {
                "a list inside a list"
            } else {
                "`null` inside a list"
            };
            return Err(not_writable(member, attribute, what));
        }
        out.push(one_encoded(ctx, key, attribute, element, member)?);
        at = list.next_slot(slot + 1);
    }
    Ok(out)
}

/// The one value `value` writes, which is not a list and not `null`.
fn one_encoded(
    ctx: &mut Ctx,
    key: u64,
    attribute: &str,
    value: Value,
    member: &str,
) -> Result<Vec<u8>, Fault> {
    if let Some(bytes) = value.as_str_bytes().or_else(|| value.as_bytes()) {
        return Ok(bytes.to_vec());
    }
    match value.tag() {
        Some(Tag::Bool) => {
            let on = value.as_bool().unwrap_or_default();
            return Ok(nvs_ldap::value::boolean_text(on).as_bytes().to_vec());
        }
        Some(Tag::Int) => {
            return Ok(value.as_int().unwrap_or_default().to_string().into_bytes());
        }
        Some(Tag::Uint) => {
            return Ok(value.as_uint().unwrap_or_default().to_string().into_bytes());
        }
        _ => {}
    }
    if crate::instance::is_instance(value, &DN) {
        return slot_bytes(value, &DN, DN_TEXT_AT, member);
    }
    if crate::instance::is_instance(value, &SID) {
        return slot_bytes(value, &SID, SID_BYTES_AT, member);
    }
    for class in [&ACCOUNT_FLAGS, &GROUP_TYPE] {
        if crate::instance::is_instance(value, class) {
            let receiver = crate::instance::receiver(value, class, member)?;
            let bits = crate::instance::slot(receiver, FLAGS_BITS_AT)
                .as_int()
                .unwrap_or_default();
            return Ok(nvs_ldap::value::flag_text(bits).into_bytes());
        }
    }
    if crate::instance::is_instance(value, &crate::uuid::CLASS) {
        let octets = crate::uuid::uuid_of(&[value], 0, member)?.into_bytes();
        // AD stores a GUID with its first three groups little-endian, and the
        // swap is its own inverse.
        let guid = nvs_ldap::value::uuid_from_guid(&octets)
            .map_err(|error| Fault::fatal(format!("{member}: {error}")))?;
        return Ok(guid.to_vec());
    }
    if crate::instance::is_instance(value, &crate::time::DURATION) {
        let nanos = crate::time::nanos_of(&[value], 0, member)?;
        return Ok(nvs_ldap::value::interval_text(nanos).into_bytes());
    }
    if crate::instance::is_instance(value, &crate::time::INSTANT) {
        let at = crate::time::instant_of(&[value], 0, member)?;
        let schema = super::schema(ctx, key, member)?;
        if schema.syntax(attribute) == nvs_ldap::Syntax::GeneralizedTime {
            return Ok(at.strftime("%Y%m%d%H%M%S%.fZ").to_string().into_bytes());
        }
        return nvs_ldap::value::filetime_text(at.as_nanosecond())
            .map(String::into_bytes)
            .ok_or_else(|| not_writable(member, attribute, "an `Instant` before the year 1601"));
    }
    Err(not_writable(
        member,
        attribute,
        match value.tag() {
            Some(Tag::Float) => "a `float`",
            Some(Tag::Decimal) => "a `decimal`",
            _ => "an object",
        },
    ))
}

/// The text or bytes in slot `at` of a `class` instance.
fn slot_bytes(
    value: Value,
    class: &crate::registry::CoreClass,
    at: usize,
    member: &str,
) -> Result<Vec<u8>, Fault> {
    let receiver = crate::instance::receiver(value, class, member)?;
    let held = crate::instance::slot(receiver, at);
    Ok(held
        .as_str_bytes()
        .or_else(|| held.as_bytes())
        .unwrap_or_default()
        .to_vec())
}

/// The key of the `Ldap\Connection` receiver in `args[0]`.
fn connection_key(args: &[Value], member: &str) -> Result<u64, Fault> {
    let receiver = crate::instance::receiver(args[0], &CONNECTION, member)?;
    crate::instance::slot(receiver, CONNECTION_HANDLE_AT)
        .as_uint()
        .ok_or_else(|| {
            // Unreachable from source: only this module's siblings build a connection.
            Fault::fatal(format!("{member} expected a `uint` in its `handle` slot"))
        })
}

/// The DN argument in `args[at]`, which may not be `null`.
fn dn_at(args: &[Value], at: usize, member: &str) -> Result<String, Fault> {
    super::dn::dn_arg(args[at], member)?.ok_or_else(|| {
        // Unreachable from source: the parameter is not nullable.
        Fault::fatal(format!("{member} expected a `Dn` or a `string`"))
    })
}

/// The attribute name a write was given, checked.
fn attribute_name(value: Value, member: &str) -> Result<String, Fault> {
    let name = value.as_text().ok_or_else(|| {
        // Unreachable from source: the parameter or the key is a `string`.
        Fault::fatal(format!("{member} expected a `string` attribute name"))
    })?;
    if !nvs_ldap::is_attribute_description(name) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{member}: `{name}` is not an attribute name. A name starts with a letter and \
                 has only letters, digits and `-`, such as `sAMAccountName`"
            ),
        ));
    }
    Ok(name.to_owned())
}

/// The `Ldap\Change` a static member builds.
fn change(args: &[Value], kind: i64, member: &str) -> Result<Value, Fault> {
    let attribute = attribute_name(args[0], &format!("{CHANGE_NAME}::{member}"))?;
    let value = if args.len() > 1 {
        retained(args[1])
    } else {
        Value::null()
    };
    Ok(crate::instance::build(
        &CHANGE,
        [
            Value::int(kind),
            Value::str(NvsStr::new(attribute.as_bytes())),
            value,
        ],
    ))
}

/// The kinds an `Ldap\Change` keeps in its `kind` slot.
const ADD_KIND: i64 = 0;
const REMOVE_KIND: i64 = 1;
const REPLACE_KIND: i64 = 2;

nvs_runtime::nvs_helper! {
    /// `Change::add(string $attribute, mixed $value): Change` — adds the values.
    fn nvs_core_ldap_change_add(_ctx, args: [2]) {
        change(args, ADD_KIND, "add")
    }
}

nvs_runtime::nvs_helper! {
    /// `Change::remove(string $attribute, mixed $value): Change` — removes the values.
    fn nvs_core_ldap_change_remove(_ctx, args: [2]) {
        change(args, REMOVE_KIND, "remove")
    }
}

nvs_runtime::nvs_helper! {
    /// `Change::removeAll(string $attribute): Change` — removes every value,
    /// which is a remove with no values.
    fn nvs_core_ldap_change_remove_all(_ctx, args: [1]) {
        change(args, REMOVE_KIND, "removeAll")
    }
}

nvs_runtime::nvs_helper! {
    /// `Change::replace(string $attribute, mixed $value): Change` — replaces
    /// every value, and removes the attribute for `null` or an empty list.
    fn nvs_core_ldap_change_replace(_ctx, args: [2]) {
        change(args, REPLACE_KIND, "replace")
    }
}

nvs_runtime::nvs_helper! {
    /// `$connection->modify(Dn|string $dn, array<Change> $changes): void` —
    /// [`modify`], each change's value encoded by [`encoded`].
    fn nvs_core_ldap_connection_modify(ctx, args: [3]) {
        let key = connection_key(args, "modify")?;
        let dn = dn_at(args, 1, MODIFY)?;
        let Some(array) = args[2].array_ptr() else {
            // Unreachable from source: the parameter is an `array<Change>`.
            return Err(Fault::fatal(format!("{MODIFY} expected an `array<Change>`")));
        };
        let list = crate::arr::borrowed(array);
        let mut changes = Vec::with_capacity(list.count());
        let mut at = list.next_slot(0);
        while let Some(slot) = at {
            let element = list.value_at(slot).unwrap_or_else(Value::null);
            let receiver = crate::instance::receiver(element, &CHANGE, "modify")?;
            let kind = match crate::instance::slot(receiver, CHANGE_KIND_AT).as_int() {
                Some(ADD_KIND) => nvs_ldap::ChangeKind::Add,
                Some(REMOVE_KIND) => nvs_ldap::ChangeKind::Remove,
                _ => nvs_ldap::ChangeKind::Replace,
            };
            let attribute = crate::instance::slot(receiver, CHANGE_ATTRIBUTE_AT)
                .as_text()
                .unwrap_or_default()
                .to_owned();
            let value = crate::instance::slot(receiver, CHANGE_VALUE_AT);
            let values = encoded(ctx, key, &attribute, value, MODIFY)?;
            if values.is_empty() && kind == nvs_ldap::ChangeKind::Add {
                return Err(not_writable(MODIFY, &attribute, "`null` or an empty list in `Change::add`"));
            }
            changes.push(nvs_ldap::Change { kind, attribute, values });
            at = list.next_slot(slot + 1);
        }
        if changes.is_empty() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("{MODIFY}: the list of changes is empty"),
            ));
        }
        modify(ctx, key, &dn, &changes)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$connection->add(Dn|string $dn, array<string, mixed> $attributes): void`
    /// — [`add`], each value encoded by [`encoded`].
    fn nvs_core_ldap_connection_add(ctx, args: [3]) {
        let key = connection_key(args, "add")?;
        let dn = dn_at(args, 1, ADD)?;
        let Some(array) = args[2].array_ptr() else {
            // Unreachable from source: the parameter is an array.
            return Err(Fault::fatal(format!("{ADD} expected an array of attributes")));
        };
        let map = crate::arr::borrowed(array);
        let mut attributes = Vec::with_capacity(map.count());
        for name in map.keys() {
            let value = map.get(&name).unwrap_or_else(Value::null);
            let name = attribute_name(Value::str(NvsStr::new(&name)), ADD)?;
            let values = encoded(ctx, key, &name, value, ADD)?;
            if values.is_empty() {
                return Err(not_writable(ADD, &name, "`null` or an empty list"));
            }
            attributes.push(nvs_ldap::Attribute { name, values });
        }
        add(ctx, key, &dn, &attributes)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$connection->delete(Dn|string $dn): void` — [`delete`].
    fn nvs_core_ldap_connection_delete(ctx, args: [2]) {
        let key = connection_key(args, "delete")?;
        let dn = dn_at(args, 1, DELETE)?;
        delete(ctx, key, &dn)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$connection->rename(Dn|string $from, Dn|string $to): void` — [`rename`].
    fn nvs_core_ldap_connection_rename(ctx, args: [3]) {
        let key = connection_key(args, "rename")?;
        let from = dn_at(args, 1, RENAME)?;
        let to = dn_at(args, 2, RENAME)?;
        rename(ctx, key, &from, &to)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$connection->setPassword(Dn|string $dn, secret tainted string $password): void`
    /// — [`set_password`].
    fn nvs_core_ldap_connection_set_password(ctx, args: [3]) {
        let key = connection_key(args, "setPassword")?;
        let dn = dn_at(args, 1, SET_PASSWORD)?;
        let password = password_at(args, 2, SET_PASSWORD)?;
        set_password(ctx, key, &dn, &password)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$connection->changePassword(Dn|string $dn, secret tainted string $old,
    /// secret tainted string $new): void` — [`change_password`].
    fn nvs_core_ldap_connection_change_password(ctx, args: [4]) {
        let key = connection_key(args, "changePassword")?;
        let dn = dn_at(args, 1, CHANGE_PASSWORD)?;
        let old = password_at(args, 2, CHANGE_PASSWORD)?;
        let new = password_at(args, 3, CHANGE_PASSWORD)?;
        change_password(ctx, key, &dn, &old, &new)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$connection->compare(Dn|string $dn, string $attribute, mixed $value): bool`
    /// — [`compare`], the value encoded by [`encoded`] as a write would send
    /// it. A comparison takes exactly one value, so `null` and a list are the
    /// program's mistake.
    fn nvs_core_ldap_connection_compare(ctx, args: [4]) {
        let key = connection_key(args, "compare")?;
        let dn = dn_at(args, 1, COMPARE)?;
        let attribute = attribute_name(args[2], COMPARE)?;
        let what = if args[3].array_ptr().is_some() {
            Some("a list")
        } else if matches!(args[3].tag(), Some(Tag::Null)) {
            Some("`null`")
        } else {
            None
        };
        if let Some(what) = what {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("{COMPARE}: the value of `{attribute}` is {what}. Compare one value at a time"),
            ));
        }
        let value = one_encoded(ctx, key, &attribute, args[3], COMPARE)?;
        Ok(Value::bool(compare(ctx, key, &dn, &attribute, &value)?))
    }
}

/// The password argument in `args[at]`.
fn password_at(args: &[Value], at: usize, member: &str) -> Result<String, Fault> {
    args[at].as_text().map(str::to_owned).ok_or_else(|| {
        // Unreachable from source: the parameter is a `string`.
        Fault::fatal(format!("{member} expected a `string` password"))
    })
}

/// The address of one of this module's symbols, or `None` for a symbol that
/// belongs to another module. See [`crate::address`].
pub(super) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_ldap_change_add" => (nvs_core_ldap_change_add as *const ()).cast(),
        "nvs_core_ldap_change_remove" => (nvs_core_ldap_change_remove as *const ()).cast(),
        "nvs_core_ldap_change_remove_all" => (nvs_core_ldap_change_remove_all as *const ()).cast(),
        "nvs_core_ldap_change_replace" => (nvs_core_ldap_change_replace as *const ()).cast(),
        "nvs_core_ldap_connection_modify" => (nvs_core_ldap_connection_modify as *const ()).cast(),
        "nvs_core_ldap_connection_add" => (nvs_core_ldap_connection_add as *const ()).cast(),
        "nvs_core_ldap_connection_delete" => (nvs_core_ldap_connection_delete as *const ()).cast(),
        "nvs_core_ldap_connection_rename" => (nvs_core_ldap_connection_rename as *const ()).cast(),
        "nvs_core_ldap_connection_set_password" => {
            (nvs_core_ldap_connection_set_password as *const ()).cast()
        }
        "nvs_core_ldap_connection_change_password" => {
            (nvs_core_ldap_connection_change_password as *const ()).cast()
        }
        "nvs_core_ldap_connection_compare" => {
            (nvs_core_ldap_connection_compare as *const ()).cast()
        }
        _ => return None,
    })
}
