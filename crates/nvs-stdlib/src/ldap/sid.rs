//! `Core\Ldap\Sid` for a Novis program: a Windows security identifier, read from its `S-1-…` text or from the bytes a directory sent
//!
//! ADR 0278 §§ 1 and 8. The SID logic is [`nvs_ldap::Sid`]'s, and every body
//! here reads its arguments, calls it, and builds what it returns.
//!
//! **A `Sid` is its binary form**, one `bytes` slot written by
//! [`nvs_ldap::Sid::to_bytes`] and nothing else, so it always reads back. The
//! binary form is the one a directory stores and a filter sends, so `bytes`
//! returns the slot, and a member that reads the parts reads the slot again.

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use super::{SID, SID_BYTES_AT, SID_NAME};

/// A `Sid` value over `sid`.
pub(super) fn sid_value(sid: &nvs_ldap::Sid) -> Value {
    crate::instance::build(&SID, [Value::bytes(NvsStr::new(&sid.to_bytes()))])
}

/// The [`nvs_ldap::Sid`] a `Sid` receiver carries.
fn sid_of(value: Value, member: &str) -> Result<nvs_ldap::Sid, Fault> {
    let receiver = crate::instance::receiver(value, &SID, member)?;
    let held = crate::instance::slot(receiver, SID_BYTES_AT);
    held.as_bytes()
        .and_then(|bytes| nvs_ldap::Sid::from_bytes(bytes).ok())
        .ok_or_else(|| {
            // Unreachable from source: only [`sid_value`] writes the slot.
            Fault::fatal(format!(
                "{SID_NAME}::{member} found a slot that `Sid::to_bytes` did not write"
            ))
        })
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Sid::parse(string $text): Sid` — [`nvs_ldap::Sid::parse`].
    fn nvs_core_ldap_sid_parse(_ctx, args: [1]) {
        let member = r"Core\Ldap\Sid::parse";
        let text = args[0].as_text().ok_or_else(|| {
            // Unreachable from source: the parameter is a `string`.
            Fault::fatal(format!("{member} expected a `string`"))
        })?;
        let sid = nvs_ldap::Sid::parse(text).map_err(|error| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("{member}: `{text}` is not a SID, because {error}"),
            )
        })?;
        Ok(sid_value(&sid))
    }
}

nvs_runtime::nvs_helper! {
    /// `$sid->toString(): string` — [`nvs_ldap::Sid::to_text`].
    fn nvs_core_ldap_sid_to_string(_ctx, args: [1]) {
        let sid = sid_of(args[0], "toString")?;
        Ok(Value::str(NvsStr::new(sid.to_text().as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `$sid->bytes(): bytes` — the slot, which is already the binary form.
    fn nvs_core_ldap_sid_bytes(_ctx, args: [1]) {
        crate::instance::read_slot(args, &SID, SID_BYTES_AT, "bytes")
    }
}

nvs_runtime::nvs_helper! {
    /// `$sid->domain(): ?Sid` — [`nvs_ldap::Sid::domain`].
    fn nvs_core_ldap_sid_domain(_ctx, args: [1]) {
        let sid = sid_of(args[0], "domain")?;
        Ok(sid.domain().map_or_else(Value::null, |domain| sid_value(&domain)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$sid->rid(): int` — [`nvs_ldap::Sid::rid`].
    fn nvs_core_ldap_sid_rid(_ctx, args: [1]) {
        let sid = sid_of(args[0], "rid")?;
        Ok(Value::int(i64::from(sid.rid())))
    }
}

/// The address of one of this module's symbols, or `None` for a symbol that
/// belongs to another module. See [`crate::address`].
pub(super) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_ldap_sid_parse" => (nvs_core_ldap_sid_parse as *const ()).cast(),
        "nvs_core_ldap_sid_to_string" => (nvs_core_ldap_sid_to_string as *const ()).cast(),
        "nvs_core_ldap_sid_bytes" => (nvs_core_ldap_sid_bytes as *const ()).cast(),
        "nvs_core_ldap_sid_domain" => (nvs_core_ldap_sid_domain as *const ()).cast(),
        "nvs_core_ldap_sid_rid" => (nvs_core_ldap_sid_rid as *const ()).cast(),
        _ => return None,
    })
}
