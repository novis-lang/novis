//! `Ldap\Entry`'s typed readers: a GUID as a `Core\Uuid`, a SID as an `Ldap\Sid`, a FILETIME or a GeneralizedTime as an `Instant`, an interval as a `Duration`, and AD's flag fields and account type
//!
//! ADR 0278 § 8, `rule:core-classes/ldap-value-types`. Each reader finds the
//! attribute's list as `string` does ([`super::search::values_named`]), reads
//! each value with [`nvs_ldap::value`], and builds the `Core` value from what
//! that returns. A value not in the form the reader needs throws `LogicError`
//! naming the attribute, as a text reader does for a value that is not text.
//!
//! **The value's form chooses, not yet the schema.** `instant` reads a
//! decimal integer as a FILETIME and anything else as a GeneralizedTime,
//! which is unambiguous because a GeneralizedTime always ends in `Z` or an
//! offset.

use std::time::{Duration, SystemTime};

use nvs_runtime::{Fault, NvsArray, ThrownClass, Value};

use super::flags::{account_flags_value, group_type_value};
use super::search::{only_value, values_named, values_of};
use super::sid::sid_value;
use super::{ACCOUNT_TYPE, ENTRY_NAME};

/// The error for a value of `name` that is not in the form `member` reads.
fn not_the_form(member: &str, name: &str, error: impl std::fmt::Display) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!("{ENTRY_NAME}::{member}: a value of `{name}` {error}"),
    )
}

/// The one value of the attribute a single-value reader was given, as
/// bytes, or `None` when the entry has none.
fn one_value(args: &[Value], member: &str) -> Result<Option<(String, Value)>, Fault> {
    let (name, values) = values_named(args, member)?;
    let Some(values) = values else {
        return Ok(None);
    };
    Ok(Some((name.to_owned(), only_value(&values, member, name)?)))
}

/// An `Instant` at `nanos` past 1970-01-01 UTC, or `None` for one no
/// `Instant` holds.
fn instant_at_unix_nanos(nanos: i128) -> Option<Value> {
    const NANOS_PER_SECOND: u128 = 1_000_000_000;
    let length = nanos.unsigned_abs();
    let length = Duration::new(
        u64::try_from(length / NANOS_PER_SECOND).ok()?,
        u32::try_from(length % NANOS_PER_SECOND).ok()?,
    );
    let at = if nanos < 0 {
        SystemTime::UNIX_EPOCH.checked_sub(length)
    } else {
        SystemTime::UNIX_EPOCH.checked_add(length)
    }?;
    crate::time::instant_at_system_time(at)
}

/// The `Instant` a FILETIME or a GeneralizedTime names, or `null` for a
/// FILETIME that means "never".
fn instant_of(value: &[u8], member: &str, name: &str) -> Result<Value, Fault> {
    if nvs_ldap::value::is_integer(value) {
        let nanos =
            nvs_ldap::value::filetime(value).map_err(|error| not_the_form(member, name, error))?;
        return match nanos {
            None => Ok(Value::null()),
            Some(nanos) => instant_at_unix_nanos(nanos)
                .ok_or_else(|| not_the_form(member, name, "is a time after the year 9999")),
        };
    }
    let read = nvs_ldap::value::generalized_time(value)
        .map_err(|error| not_the_form(member, name, error))?;
    let civil = crate::time::Civil {
        year: read.year,
        month: read.month,
        day: read.day,
        hour: read.hour,
        minute: read.minute,
        second: read.second,
        nanosecond: read.nanosecond,
    };
    crate::time::instant_at(&civil, read.offset)
        .ok_or_else(|| not_the_form(member, name, "names a day or a second that does not exist"))
}

nvs_runtime::nvs_helper! {
    /// `$entry->uuid(string $name): ?Core\Uuid` — AD's GUID bytes, the first
    /// three groups swapped by [`nvs_ldap::value::uuid_from_guid`].
    fn nvs_core_ldap_entry_uuid(_ctx, args: [2]) {
        let member = "uuid";
        let Some((name, value)) = one_value(args, member)? else {
            return Ok(Value::null());
        };
        let octets = nvs_ldap::value::uuid_from_guid(value.as_bytes().unwrap_or_default())
            .map_err(|error| not_the_form(member, &name, error))?;
        Ok(crate::uuid::of_octets(octets))
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->sid(string $name): ?Sid` — the SID's binary form, read by
    /// [`nvs_ldap::Sid::from_bytes`].
    fn nvs_core_ldap_entry_sid(_ctx, args: [2]) {
        let member = "sid";
        let Some((name, value)) = one_value(args, member)? else {
            return Ok(Value::null());
        };
        let sid = nvs_ldap::Sid::from_bytes(value.as_bytes().unwrap_or_default())
            .map_err(|error| not_the_form(member, &name, error))?;
        Ok(sid_value(&sid))
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->sids(string $name): ?array<Sid>` — every value of the
    /// attribute, as `sid` reads one, in the order the server sent them.
    fn nvs_core_ldap_entry_sids(_ctx, args: [2]) {
        let member = "sids";
        let (name, values) = values_named(args, member)?;
        let Some(values) = values else {
            return Ok(Value::null());
        };
        let mut out = NvsArray::new();
        let mut at = values.next_slot(0);
        while let Some(slot) = at {
            let value = values.value_at(slot).unwrap_or_else(Value::null);
            let sid = nvs_ldap::Sid::from_bytes(value.as_bytes().unwrap_or_default())
                .map_err(|error| not_the_form(member, name, error))?;
            out.append(sid_value(&sid));
            at = values.next_slot(slot + 1);
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->instant(string $name): ?Instant` — a FILETIME or a
    /// GeneralizedTime, by [`instant_of`].
    fn nvs_core_ldap_entry_instant(_ctx, args: [2]) {
        let member = "instant";
        let Some((name, value)) = one_value(args, member)? else {
            return Ok(Value::null());
        };
        instant_of(value.as_bytes().unwrap_or_default(), member, &name)
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->duration(string $name): ?Duration` — AD's negative interval,
    /// read by [`nvs_ldap::value::interval`].
    fn nvs_core_ldap_entry_duration(_ctx, args: [2]) {
        let member = "duration";
        let Some((name, value)) = one_value(args, member)? else {
            return Ok(Value::null());
        };
        let nanos = nvs_ldap::value::interval(value.as_bytes().unwrap_or_default())
            .map_err(|error| not_the_form(member, &name, error))?;
        Ok(nanos.map_or_else(Value::null, crate::time::duration_of))
    }
}

/// The integer the one value of `name` is, read by `read`, or `None` when the
/// entry has no value for it. For a reader with a fixed attribute.
fn fixed_value<T>(
    entry: Value,
    name: &str,
    member: &str,
    read: impl Fn(&[u8]) -> Result<T, nvs_ldap::value::ValueError>,
) -> Result<Option<T>, Fault> {
    let Some(values) = values_of(entry, name, member)? else {
        return Ok(None);
    };
    let value = only_value(&values, member, name)?;
    read(value.as_bytes().unwrap_or_default())
        .map(Some)
        .map_err(|error| not_the_form(member, name, error))
}

nvs_runtime::nvs_helper! {
    /// `$entry->accountFlags(): ?Ad\AccountFlags` — `userAccountControl`, with
    /// the computed attribute and `pwdLastSet` where the entry carries them.
    /// See [`super::flags`].
    fn nvs_core_ldap_entry_account_flags(_ctx, args: [1]) {
        let member = "accountFlags";
        let read = nvs_ldap::value::flag_field;
        let Some(bits) = fixed_value(args[0], "userAccountControl", member, read)? else {
            return Ok(Value::null());
        };
        let computed = nvs_ldap::value::COMPUTED_ACCOUNT_CONTROL;
        let computed = fixed_value(args[0], computed, member, read)?;
        let must_change = fixed_value(args[0], "pwdLastSet", member, |value| Ok(value == b"0"))?;
        Ok(account_flags_value(bits, computed, must_change.unwrap_or(false)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->groupType(): ?Ad\GroupType` — `groupType`, signed as AD
    /// writes it.
    fn nvs_core_ldap_entry_group_type(_ctx, args: [1]) {
        let read = nvs_ldap::value::flag_field;
        Ok(fixed_value(args[0], "groupType", "groupType", read)?
            .map_or_else(Value::null, group_type_value))
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->accountType(): ?Ad\AccountType` — `sAMAccountType`, as the
    /// [`ACCOUNT_TYPE`] case [`nvs_ldap::value::AccountType`] names.
    fn nvs_core_ldap_entry_account_type(_ctx, args: [1]) {
        let read = nvs_ldap::value::account_type;
        let Some(kind) = fixed_value(args[0], "sAMAccountType", "accountType", read)? else {
            return Ok(Value::null());
        };
        let (_, ordinal) = ACCOUNT_TYPE
            .cases
            .iter()
            .find(|(name, _)| *name == kind.name())
            .expect("every `nvs_ldap::value::AccountType` names a case `ACCOUNT_TYPE` registers");
        Ok(Value::int(*ordinal))
    }
}

/// The address of one of this module's symbols, or `None` for a symbol that
/// belongs to another module. See [`crate::address`].
pub(super) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_ldap_entry_uuid" => (nvs_core_ldap_entry_uuid as *const ()).cast(),
        "nvs_core_ldap_entry_sid" => (nvs_core_ldap_entry_sid as *const ()).cast(),
        "nvs_core_ldap_entry_sids" => (nvs_core_ldap_entry_sids as *const ()).cast(),
        "nvs_core_ldap_entry_instant" => (nvs_core_ldap_entry_instant as *const ()).cast(),
        "nvs_core_ldap_entry_duration" => (nvs_core_ldap_entry_duration as *const ()).cast(),
        "nvs_core_ldap_entry_account_flags" => {
            (nvs_core_ldap_entry_account_flags as *const ()).cast()
        }
        "nvs_core_ldap_entry_group_type" => (nvs_core_ldap_entry_group_type as *const ()).cast(),
        "nvs_core_ldap_entry_account_type" => {
            (nvs_core_ldap_entry_account_type as *const ()).cast()
        }
        _ => return None,
    })
}
