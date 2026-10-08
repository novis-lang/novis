//! `Core\Ldap\Dn` for a Novis program: a DN built from parts with each value escaped, read from text an operator wrote, or taken from an entry the server sent
//!
//! ADR 0278 § 7, `rule:core-classes/ldap-dn-is-the-launderer`. The DN logic is
//! [`nvs_ldap::Dn`]'s, and every body here reads its arguments, calls it, and
//! builds what it returns.
//!
//! **A `Dn` is its RFC 4514 text**, one `string` slot written by
//! [`nvs_ldap::Dn::to_text`] and nothing else, so it always parses back. A
//! member that reads the parts parses the slot again. That costs a pass over a
//! short string per call, and keeps the value one slot wide.
//!
//! [`dn_arg`] reads every `Dn|string` parameter. The `string` arm is a sink,
//! so its text is sent as the program wrote it; the `Dn` arm sends the slot.

use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use super::{DN, DN_NAME, DN_TEXT_AT};

/// A `Dn` value over `dn`.
pub(super) fn dn_value(dn: &nvs_ldap::Dn) -> Value {
    crate::instance::build(&DN, [Value::str(NvsStr::new(dn.to_text().as_bytes()))])
}

/// The [`nvs_ldap::Dn`] a `Dn` argument carries.
fn dn_of(value: Value, member: &str) -> Result<nvs_ldap::Dn, Fault> {
    let receiver = crate::instance::receiver(value, &DN, member)?;
    let held = crate::instance::slot(receiver, DN_TEXT_AT);
    held.as_text()
        .and_then(|text| nvs_ldap::Dn::parse(text).ok())
        .ok_or_else(|| {
            // Unreachable from source: only [`dn_value`] writes the slot.
            Fault::fatal(format!(
                "{DN_NAME}::{member} found a slot that `Dn::to_text` did not write"
            ))
        })
}

/// The DN text a `Dn|string` argument names, or `None` for the `null` an
/// omitted option passes.
pub(super) fn dn_arg(value: Value, member: &str) -> Result<Option<String>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    if let Some(text) = value.as_text() {
        return Ok(Some(text.to_owned()));
    }
    let receiver = crate::instance::receiver(value, &DN, member)?;
    let held = crate::instance::slot(receiver, DN_TEXT_AT);
    Ok(held.as_text().map(str::to_owned))
}

/// The `Dn` for the DN text an entry carries. The server wrote that text, so
/// one that does not parse is a protocol error.
pub(super) fn from_server(text: &str, member: &str) -> Result<Value, Fault> {
    let dn = nvs_ldap::Dn::parse(text).map_err(|error| {
        super::fault_of(
            member,
            &nvs_ldap::Error::new(
                nvs_ldap::Kind::Protocol,
                format!("the server sent a DN that is not RFC 4514 text: {error}"),
            ),
        )
    })?;
    Ok(dn_value(&dn))
}

/// The error for an attribute or a value [`nvs_ldap::Dn::of`] refused.
fn bad_part(member: &str, attribute: &str, error: nvs_ldap::PartError) -> Fault {
    let message = match error {
        nvs_ldap::PartError::Attribute => format!(
            "{member}: `{attribute}` is not an attribute name. A name starts with a letter and \
             has only letters, digits and `-`, such as `CN` or `OU`"
        ),
        nvs_ldap::PartError::EmptyValue => format!("{member}: the value is empty"),
    };
    Fault::thrown_as(ThrownClass::Logic, message)
}

/// A `string` argument.
fn text_arg<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        // Unreachable from source: the parameter is a `string`.
        Fault::fatal(format!("{member} expected a `string`"))
    })
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Dn::parse(string $text): Dn` — RFC 4514 text an operator wrote,
    /// read by [`nvs_ldap::Dn::parse`]. The parameter is a sink, so the text
    /// never came from a request.
    fn nvs_core_ldap_dn_parse(_ctx, args: [1]) {
        let member = r"Core\Ldap\Dn::parse";
        let dn = nvs_ldap::Dn::parse(text_arg(&args[0], member)?).map_err(|error| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{member}: the text stops being a DN at character {}: {}",
                    error.position, error.reason
                ),
            )
        })?;
        Ok(dn_value(&dn))
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Dn::of(string $attribute, tainted string $value): Dn` — the DN
    /// sink's launderer: the value is escaped as it is written, and the
    /// answer is a `Dn`, which only a DN parameter takes.
    fn nvs_core_ldap_dn_of(_ctx, args: [2]) {
        let member = r"Core\Ldap\Dn::of";
        let attribute = text_arg(&args[0], member)?;
        let dn = nvs_ldap::Dn::of(attribute, text_arg(&args[1], member)?)
            .map_err(|error| bad_part(member, attribute, error))?;
        Ok(dn_value(&dn))
    }
}

nvs_runtime::nvs_helper! {
    /// `$dn->child(string $attribute, tainted string $value): Dn` — the DN
    /// sink's launderer, as `of` is, one level below the receiver.
    fn nvs_core_ldap_dn_child(_ctx, args: [3]) {
        let member = "child";
        let dn = dn_of(args[0], member)?;
        let attribute = text_arg(&args[1], member)?;
        let child = dn
            .child(attribute, text_arg(&args[2], member)?)
            .map_err(|error| bad_part(r"Core\Ldap\Dn->child", attribute, error))?;
        Ok(dn_value(&child))
    }
}

nvs_runtime::nvs_helper! {
    /// `$dn->parent(): ?Dn` — the DN without its first level.
    fn nvs_core_ldap_dn_parent(_ctx, args: [1]) {
        let dn = dn_of(args[0], "parent")?;
        Ok(dn.parent().map_or_else(Value::null, |parent| dn_value(&parent)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$dn->rdnAttribute(): string` — the first level's first attribute.
    fn nvs_core_ldap_dn_rdn_attribute(_ctx, args: [1]) {
        let dn = dn_of(args[0], "rdnAttribute")?;
        Ok(Value::str(NvsStr::new(dn.rdn().first().attribute.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `$dn->rdnValue(): tainted string` — the first level's first value,
    /// unescaped.
    fn nvs_core_ldap_dn_rdn_value(_ctx, args: [1]) {
        let dn = dn_of(args[0], "rdnValue")?;
        Ok(Value::str(NvsStr::new(dn.rdn().first().value.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `$dn->isWithin(Dn $other): bool` — [`nvs_ldap::Dn::is_within`].
    fn nvs_core_ldap_dn_is_within(_ctx, args: [2]) {
        let dn = dn_of(args[0], "isWithin")?;
        let other = dn_of(args[1], "isWithin")?;
        Ok(Value::bool(dn.is_within(&other)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$dn->toString(): tainted string` — the slot, which is already the
    /// RFC 4514 text.
    fn nvs_core_ldap_dn_to_string(_ctx, args: [1]) {
        crate::instance::read_slot(args, &DN, DN_TEXT_AT, "toString")
    }
}

/// The address of one of this module's symbols, or `None` for a symbol that
/// belongs to another module. See [`crate::address`].
pub(super) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_ldap_dn_parse" => (nvs_core_ldap_dn_parse as *const ()).cast(),
        "nvs_core_ldap_dn_of" => (nvs_core_ldap_dn_of as *const ()).cast(),
        "nvs_core_ldap_dn_child" => (nvs_core_ldap_dn_child as *const ()).cast(),
        "nvs_core_ldap_dn_parent" => (nvs_core_ldap_dn_parent as *const ()).cast(),
        "nvs_core_ldap_dn_rdn_attribute" => (nvs_core_ldap_dn_rdn_attribute as *const ()).cast(),
        "nvs_core_ldap_dn_rdn_value" => (nvs_core_ldap_dn_rdn_value as *const ()).cast(),
        "nvs_core_ldap_dn_is_within" => (nvs_core_ldap_dn_is_within as *const ()).cast(),
        "nvs_core_ldap_dn_to_string" => (nvs_core_ldap_dn_to_string as *const ()).cast(),
        _ => return None,
    })
}
