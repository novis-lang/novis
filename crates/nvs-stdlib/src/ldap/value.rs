//! `Ldap\Entry`'s typed readers: an INTEGER as an `int`, `TRUE`/`FALSE` as a `bool`, a GUID as a `Core\Uuid`, a SID as an `Ldap\Sid`, a FILETIME or a GeneralizedTime as an `Instant`, an interval as a `Duration`, and AD's flag fields and account type
//!
//! `rule:core-classes/ldap-value-types`. Each reader finds the
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
    /// `$entry->int(string $name): ?int` — an INTEGER, read by
    /// [`nvs_ldap::value::int`].
    fn nvs_core_ldap_entry_int(_ctx, args: [2]) {
        let member = "int";
        let Some((name, value)) = one_value(args, member)? else {
            return Ok(Value::null());
        };
        nvs_ldap::value::int(value.as_bytes().unwrap_or_default())
            .map(Value::int)
            .map_err(|error| not_the_form(member, &name, error))
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->bool(string $name): ?bool` — `TRUE` or `FALSE`, read by
    /// [`nvs_ldap::value::boolean`].
    fn nvs_core_ldap_entry_bool(_ctx, args: [2]) {
        let member = "bool";
        let Some((name, value)) = one_value(args, member)? else {
            return Ok(Value::null());
        };
        nvs_ldap::value::boolean(value.as_bytes().unwrap_or_default())
            .map(Value::bool)
            .map_err(|error| not_the_form(member, &name, error))
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
        "nvs_core_ldap_entry_int" => (nvs_core_ldap_entry_int as *const ()).cast(),
        "nvs_core_ldap_entry_bool" => (nvs_core_ldap_entry_bool as *const ()).cast(),
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

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsFn, NvsStr, OutputSink, Tag, Value, call};

    use super::super::flags::{
        nvs_core_ldap_ad_account_flags_bits, nvs_core_ldap_ad_group_type_bits,
    };
    use super::super::search::{
        entry_value, nvs_core_ldap_entry_bytes, nvs_core_ldap_entry_dn, nvs_core_ldap_entry_has,
        nvs_core_ldap_entry_is_deleted, nvs_core_ldap_entry_string, nvs_core_ldap_entry_strings,
        nvs_core_ldap_entry_to_array,
    };
    use super::*;

    /// MS-DTYP § 2.3.4's GUID `{6F9619FF-8B86-D011-B42D-00C04FC964FF}`, as AD
    /// stores it, with its first three groups little-endian.
    const GUID: [u8; 16] = [
        0xff, 0x19, 0x96, 0x6f, 0x86, 0x8b, 0x11, 0xd0, 0xb4, 0x2d, 0x00, 0xc0, 0x4f, 0xc9, 0x64,
        0xff,
    ];
    /// `BUILTIN\Administrators`, MS-DTYP § 2.4.2.4.
    const SID: [u8; 16] = [1, 2, 0, 0, 0, 0, 0, 5, 0x20, 0, 0, 0, 0x20, 0x02, 0, 0];
    const DN: &str = "CN=Anna Berg,OU=Company,DC=example,DC=test";

    /// An entry the way a search returns a user, with one attribute per
    /// reader and `isDeleted` only when `deleted` is set.
    fn person(deleted: bool) -> Value {
        let text = |name: &str, values: &[&str]| nvs_ldap::Attribute {
            name: name.to_owned(),
            values: values
                .iter()
                .map(|value| value.as_bytes().to_vec())
                .collect(),
        };
        let mut attributes = vec![
            text("sAMAccountName", &["anna.berg"]),
            text(
                "memberOf",
                &[
                    "CN=Employees,OU=Company,DC=example,DC=test",
                    "CN=Managers,OU=Company,DC=example,DC=test",
                ],
            ),
            nvs_ldap::Attribute {
                name: "objectGUID".to_owned(),
                values: vec![GUID.to_vec()],
            },
            nvs_ldap::Attribute {
                name: "objectSid".to_owned(),
                values: vec![SID.to_vec(), SID.to_vec()],
            },
            text("logonCount", &["7"]),
            text("showInAdvancedViewOnly", &["FALSE"]),
            // 1970-01-01, as a FILETIME.
            text("pwdLastSet", &["116444736000000000"]),
            text("whenCreated", &["20240101120000.0Z"]),
            // Thirty minutes, written negative as AD writes an interval.
            text("lockoutDuration", &["-18000000000"]),
            text("userAccountControl", &["514"]),
            text("groupType", &["-2147483646"]),
            text("sAMAccountType", &["805306368"]),
        ];
        if deleted {
            attributes.push(text("isDeleted", &["TRUE"]));
        }
        entry_value(nvs_ldap::Entry {
            dn: DN.to_owned(),
            attributes,
        })
    }

    fn named(name: &str) -> Value {
        Value::str(NvsStr::new(name.as_bytes()))
    }

    fn read(ctx: &mut Ctx, member: NvsFn, entry: Value, name: &str) -> Result<Value, i32> {
        call(member, ctx, &[entry, named(name)])
    }

    fn text_of(value: Value) -> String {
        value.as_text().expect("text").to_owned()
    }

    fn is_null(value: Value) -> bool {
        matches!(value.tag(), Some(Tag::Null))
    }

    /// Every `Ldap\Entry` reader, end to end through `call`: each reads its
    /// own form, a name matches without case, a missing attribute reads as
    /// `null`, and a value in another form or a list where one value belongs
    /// throws.
    // covers: Core\Ldap\Entry::dn, Core\Ldap\Entry::has, Core\Ldap\Entry::isDeleted
    // covers: Core\Ldap\Entry::string, Core\Ldap\Entry::strings, Core\Ldap\Entry::bytes
    // covers: Core\Ldap\Entry::int, Core\Ldap\Entry::bool, Core\Ldap\Entry::uuid
    // covers: Core\Ldap\Entry::sid, Core\Ldap\Entry::sids, Core\Ldap\Entry::instant
    // covers: Core\Ldap\Entry::duration, Core\Ldap\Entry::accountFlags
    // covers: Core\Ldap\Entry::groupType, Core\Ldap\Entry::accountType, Core\Ldap\Entry::toArray
    #[test]
    fn every_entry_reader_reads_its_own_form() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let ctx = &mut ctx;
        let entry = person(false);
        let ok = |result: Result<Value, i32>| result.expect("the reader returns");

        let dn = ok(call(nvs_core_ldap_entry_dn, ctx, &[entry]));
        let dn = ok(call(
            super::super::dn::nvs_core_ldap_dn_to_string,
            ctx,
            &[dn],
        ));
        assert_eq!(text_of(dn), DN);

        let has =
            |ctx: &mut Ctx, name| ok(read(ctx, nvs_core_ldap_entry_has, entry, name)).as_bool();
        assert_eq!(
            has(ctx, "SAMACCOUNTNAME"),
            Some(true),
            "a name matches without case"
        );
        assert_eq!(has(ctx, "telephoneNumber"), Some(false));

        let string = ok(read(
            ctx,
            nvs_core_ldap_entry_string,
            entry,
            "samaccountname",
        ));
        assert_eq!(text_of(string), "anna.berg");
        assert!(is_null(ok(read(
            ctx,
            nvs_core_ldap_entry_string,
            entry,
            "telephoneNumber"
        ))));
        assert!(
            read(ctx, nvs_core_ldap_entry_string, entry, "memberOf").is_err(),
            "a list is not one value"
        );
        assert!(
            read(ctx, nvs_core_ldap_entry_string, entry, "objectGUID").is_err(),
            "a GUID is not text"
        );

        let groups = ok(read(ctx, nvs_core_ldap_entry_strings, entry, "memberOf"));
        let groups = groups.as_array().expect("a list");
        assert_eq!(groups.count(), 2);
        assert_eq!(
            text_of(groups.get_index(1).expect("the second group")),
            "CN=Managers,OU=Company,DC=example,DC=test",
            "the values keep the server's order"
        );

        let bytes = ok(read(ctx, nvs_core_ldap_entry_bytes, entry, "objectGUID"));
        assert_eq!(bytes.as_bytes(), Some(&GUID[..]));

        let int = ok(read(ctx, nvs_core_ldap_entry_int, entry, "logonCount"));
        assert_eq!(int.as_int(), Some(7));
        assert!(read(ctx, nvs_core_ldap_entry_int, entry, "sAMAccountName").is_err());
        let bool = ok(read(
            ctx,
            nvs_core_ldap_entry_bool,
            entry,
            "showInAdvancedViewOnly",
        ));
        assert_eq!(bool.as_bool(), Some(false));
        assert!(read(ctx, nvs_core_ldap_entry_bool, entry, "logonCount").is_err());

        let uuid = ok(read(ctx, nvs_core_ldap_entry_uuid, entry, "objectGUID"));
        let uuid = ok(call(crate::uuid::nvs_core_uuid_to_string, ctx, &[uuid]));
        assert_eq!(text_of(uuid), "6f9619ff-8b86-d011-b42d-00c04fc964ff");
        assert!(read(ctx, nvs_core_ldap_entry_uuid, entry, "logonCount").is_err());

        let to_text = super::super::sid::nvs_core_ldap_sid_to_string;
        assert!(
            read(ctx, nvs_core_ldap_entry_sid, entry, "objectSid").is_err(),
            "two SIDs are not one value"
        );
        let sids = ok(read(ctx, nvs_core_ldap_entry_sids, entry, "objectSid"));
        let sids = sids.as_array().expect("a list");
        assert_eq!(sids.count(), 2);
        let sid = ok(call(to_text, ctx, &[sids.get_index(0).expect("a SID")]));
        assert_eq!(text_of(sid), "S-1-5-32-544");

        let epoch_seconds = crate::time::nvs_core_time_instant_to_epoch_seconds;
        let set = ok(read(ctx, nvs_core_ldap_entry_instant, entry, "pwdLastSet"));
        assert_eq!(
            ok(call(epoch_seconds, ctx, &[set])).as_int(),
            Some(0),
            "a FILETIME"
        );
        let created = ok(read(ctx, nvs_core_ldap_entry_instant, entry, "whenCreated"));
        assert_eq!(
            ok(call(epoch_seconds, ctx, &[created])).as_int(),
            Some(1_704_110_400),
            "a GeneralizedTime"
        );
        let lockout = ok(read(
            ctx,
            nvs_core_ldap_entry_duration,
            entry,
            "lockoutDuration",
        ));
        let seconds = crate::time::nvs_core_time_duration_to_seconds;
        assert_eq!(ok(call(seconds, ctx, &[lockout])).as_int(), Some(1800));

        let flags = ok(call(nvs_core_ldap_entry_account_flags, ctx, &[entry]));
        let bits = ok(call(nvs_core_ldap_ad_account_flags_bits, ctx, &[flags]));
        assert_eq!(bits.as_int(), Some(514));
        let group = ok(call(nvs_core_ldap_entry_group_type, ctx, &[entry]));
        let bits = ok(call(nvs_core_ldap_ad_group_type_bits, ctx, &[group]));
        assert_eq!(
            bits.as_int(),
            Some(-2_147_483_646),
            "signed, as AD writes it"
        );
        let kind = ok(call(nvs_core_ldap_entry_account_type, ctx, &[entry]));
        let user = ACCOUNT_TYPE.cases.iter().find(|(name, _)| *name == "User");
        assert_eq!(kind.as_int(), user.map(|(_, ordinal)| *ordinal));

        let all = ok(call(nvs_core_ldap_entry_to_array, ctx, &[entry]));
        let all = all.as_array().expect("an array");
        assert_eq!(all.count(), 12);
        assert!(
            all.get(b"sAMAccountName").is_some(),
            "the key is the server's spelling"
        );
        assert!(all.get(b"samaccountname").is_none());

        let deleted =
            |ctx: &mut Ctx, entry| ok(call(nvs_core_ldap_entry_is_deleted, ctx, &[entry]));
        assert_eq!(deleted(ctx, entry).as_bool(), Some(false));
        assert_eq!(deleted(ctx, person(true)).as_bool(), Some(true));
    }
}
