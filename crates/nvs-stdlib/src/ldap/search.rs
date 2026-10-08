//! `Core\Ldap`'s reading half for a Novis program: `Ldap\Connection`'s `whoami`, `search` and `read`, the `Ldap\Entries` a search returns, the `Ldap\Entry` it yields, and the `Ldap\Filter` it takes
//!
//! ADR 0278 §§ 1, 5 and 6. Every body here reads its arguments, calls the
//! Rust half in [`super`], and builds what it returns.
//!
//! **An `Ldap\Entry` is a value, built once per entry.** Its two slots are the
//! DN and an array keyed by each attribute's name as the server spelled it,
//! whose values are lists of `bytes`. A reader finds a name without case, as
//! LDAP does, and `toArray`'s keys keep the server's spelling. The typed
//! readers ADR 0278 § 8 adds read the same lists.
//!
//! **An `Ldap\Filter` is its BER encoding**, one `bytes` slot written by the
//! static member that built it (`rule:core-classes/ldap-filter-is-a-value`).
//! A search sends it as [`nvs_ldap::Filter::Encoded`], so a value is never
//! rendered as text on the way to the wire, and an attribute name is checked
//! by [`nvs_ldap::is_attribute_description`] before it is encoded.
//!
//! **`Ldap\Entries` is its own iterator**, as `Core\Db\Stream` is, because the
//! next entry may be on a page the server has not sent yet. It carries the
//! connection's key, the id its search is parked under ([`super::step`]), the
//! entry the last `advance()` read, and the continuation references, which
//! the last `advance()` writes when the search ends. Until then the slot is
//! `null`, and `references()` reads the parked search instead.

use nvs_runtime::{Fault, NvsArray, NvsStr, ThrownClass, Value};

use super::{
    CONNECTION, CONNECTION_HANDLE_AT, ENTRIES, ENTRIES_ENTRY_AT, ENTRIES_HANDLE_AT,
    ENTRIES_REFERENCES_AT, ENTRIES_SEARCH_AT, ENTRY, ENTRY_ATTRIBUTES_AT, ENTRY_DN_AT, ENTRY_NAME,
    FILTER, FILTER_BER_AT, READ, SEARCH, Step,
};
use crate::registry::CoreClass;

/// `Ldap\Entries::iterate`'s symbol, which the dispatch roster names.
pub(crate) const ENTRIES_ITERATE_SYMBOL: &str = "nvs_core_ldap_entries_iterate";
/// `Ldap\Entries::advance`'s symbol.
pub(crate) const ENTRIES_ADVANCE_SYMBOL: &str = "nvs_core_ldap_entries_advance";
/// `Ldap\Entries::current`'s symbol.
pub(crate) const ENTRIES_CURRENT_SYMBOL: &str = "nvs_core_ldap_entries_current";

/// `value` with a reference of its own, for a value read out of a slot or an
/// array the receiver still holds.
fn retained(value: Value) -> Value {
    #[expect(
        unsafe_code,
        reason = "the slot or array the value was read from keeps its own \
                  reference, so the copy handed back to Novis code needs one"
    )]
    // SAFETY: the receiver is live for the length of the call, so what it holds is.
    unsafe {
        value.retain();
    }
    value
}

/// The `uint` key in slot `at` of a `class` receiver.
fn key_in(value: Value, class: &CoreClass, at: usize, member: &str) -> Result<u64, Fault> {
    let receiver = crate::instance::receiver(value, class, member)?;
    crate::instance::slot(receiver, at)
        .as_uint()
        .ok_or_else(|| {
            // Unreachable from source: only this module builds these classes.
            Fault::fatal(format!(
                "{}::{member} expected a `uint` in its `{}` slot",
                class.name, class.slots[at]
            ))
        })
}

/// A `string` argument, or `None` for the `null` an omitted option passes.
fn optional_text(value: &Value) -> Option<&str> {
    value.as_text()
}

/// The error for an attribute name RFC 4512 does not allow.
fn not_an_attribute(member: &str, name: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{member}: `{name}` is not an attribute name. A name starts with a letter and \
             has only letters, digits and `-`, such as `sAMAccountName`"
        ),
    )
}

/// The `select` option: the attribute names an entry carries, each checked.
/// `null` and an empty list both return every attribute a user can read.
fn selected(value: &Value, member: &str) -> Result<Vec<String>, Fault> {
    let Some(array) = value.array_ptr() else {
        return Ok(Vec::new());
    };
    let names = crate::arr::borrowed(array);
    let mut out = Vec::with_capacity(names.count());
    let mut at = names.next_slot(0);
    while let Some(slot) = at {
        let name = names
            .value_at(slot)
            .and_then(|value| value.as_text().map(str::to_owned))
            .ok_or_else(|| {
                // Unreachable from source: `select` is an `array<string>`.
                Fault::fatal(format!("{member} expected a `string` in `select`"))
            })?;
        if !nvs_ldap::is_attribute_description(&name) {
            return Err(not_an_attribute(member, &name));
        }
        out.push(name);
        at = names.next_slot(slot + 1);
    }
    Ok(out)
}

/// An `Ldap\Entry` over `entry`.
fn entry_value(entry: nvs_ldap::Entry) -> Value {
    let mut attributes = NvsArray::new();
    for attribute in entry.attributes {
        let mut values = NvsArray::new();
        for value in &attribute.values {
            values.append(Value::bytes(NvsStr::new(value)));
        }
        attributes.set(NvsStr::new(attribute.name.as_bytes()), Value::array(values));
    }
    crate::instance::build(
        &ENTRY,
        [
            Value::str(NvsStr::new(entry.dn.as_bytes())),
            Value::array(attributes),
        ],
    )
}

/// The name an `Ldap\Entry` reader was given, and the list of values the
/// entry has under it, found without case, or `None` when it has none.
fn values_named<'a>(
    args: &'a [Value],
    member: &str,
) -> Result<(&'a str, Option<std::mem::ManuallyDrop<NvsArray>>), Fault> {
    let receiver = crate::instance::receiver(args[0], &ENTRY, member)?;
    let name = args[1].as_text().ok_or_else(|| {
        // Unreachable from source: the parameter is a `string`.
        Fault::fatal(format!("{ENTRY_NAME}::{member} expected a `string` name"))
    })?;
    let held = crate::instance::slot(receiver, ENTRY_ATTRIBUTES_AT);
    let Some(array) = held.array_ptr() else {
        // Unreachable from source: only [`entry_value`] builds an entry.
        return Err(Fault::fatal(format!(
            "{ENTRY_NAME}::{member} found no array in its attributes slot"
        )));
    };
    let attributes = crate::arr::borrowed(array);
    let found = attributes
        .keys()
        .into_iter()
        .find(|key| key.eq_ignore_ascii_case(name.as_bytes()))
        .and_then(|key| attributes.get(&key))
        .and_then(Value::array_ptr)
        .map(crate::arr::borrowed);
    Ok((name, found))
}

/// The one value a single-value reader returns, or the error for a list
/// with more than one.
fn only_value(values: &NvsArray, member: &str, name: &str) -> Result<Value, Fault> {
    match (values.count(), values.get_index(0)) {
        (1, Some(value)) => Ok(value),
        (count, _) => Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{ENTRY_NAME}::{member}: `{name}` has {count} values. `strings` returns all \
                 of them"
            ),
        )),
    }
}

/// `value` as a `string`, or the error for a value that is not UTF-8.
fn text_value(value: Value, member: &str, name: &str) -> Result<Value, Fault> {
    let bytes = value.as_bytes().unwrap_or_default();
    if std::str::from_utf8(bytes).is_err() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{ENTRY_NAME}::{member}: a value of `{name}` is not text. `bytes` returns it \
                 as it is"
            ),
        ));
    }
    Ok(Value::str(NvsStr::new(bytes)))
}

/// `urls` as an `array<string>`.
fn url_list(urls: &[String]) -> Value {
    let mut out = NvsArray::new();
    for url in urls {
        out.append(Value::str(NvsStr::new(url.as_bytes())));
    }
    Value::array(out)
}

/// A `Ldap\Filter` over `filter`'s encoding.
fn filter_value(filter: &nvs_ldap::Filter) -> Value {
    crate::instance::build(&FILTER, [Value::bytes(NvsStr::new(&filter.to_ber()))])
}

/// The attribute name a `Ldap\Filter` constructor was given, checked.
fn attribute_arg<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    let name = value.as_text().ok_or_else(|| {
        // Unreachable from source: the parameter is a `string`.
        Fault::fatal(format!("{member} expected a `string` attribute"))
    })?;
    if !nvs_ldap::is_attribute_description(name) {
        return Err(not_an_attribute(member, name));
    }
    Ok(name)
}

/// The value a `Ldap\Filter` constructor was given, as the bytes it encodes.
fn value_arg<'a>(value: &'a Value, member: &str) -> Result<&'a [u8], Fault> {
    value.as_str_bytes().ok_or_else(|| {
        // Unreachable from source: the parameter is a `string`.
        Fault::fatal(format!("{member} expected a `string` value"))
    })
}

/// A part a substring filter is built from, which may not be empty: RFC 4511
/// has no empty part, and `present` is the filter for any value at all.
fn part_arg<'a>(value: &'a Value, member: &str) -> Result<&'a [u8], Fault> {
    let part = value_arg(value, member)?;
    if part.is_empty() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{member}: the value is empty. `present` matches every entry with a value"),
        ));
    }
    Ok(part)
}

/// The BER a `Ldap\Filter` argument carries.
fn ber_of(value: Value, member: &str) -> Result<Vec<u8>, Fault> {
    let receiver = crate::instance::receiver(value, &FILTER, member)?;
    let ber = crate::instance::slot(receiver, FILTER_BER_AT);
    Ok(ber.as_bytes().unwrap_or_default().to_vec())
}

/// The filters `all` or `any` was given, at least one.
fn filters_arg(value: &Value, member: &str) -> Result<Vec<nvs_ldap::Filter>, Fault> {
    let Some(array) = value.array_ptr() else {
        // Unreachable from source: a variadic tail arrives as an array.
        return Err(Fault::fatal(format!(
            "{member} expected an array of filters"
        )));
    };
    let filters = crate::arr::borrowed(array);
    let mut out = Vec::with_capacity(filters.count());
    let mut at = filters.next_slot(0);
    while let Some(slot) = at {
        let filter = filters.value_at(slot).unwrap_or_else(Value::null);
        out.push(nvs_ldap::Filter::Encoded(ber_of(filter, member)?));
        at = filters.next_slot(slot + 1);
    }
    if out.is_empty() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{member}: there is no filter. Give one filter or more"),
        ));
    }
    Ok(out)
}

/// The scope a `search` names, by the ordinal [`super::SCOPE`] gives it.
fn scope_of(value: &Value) -> Result<nvs_ldap::Scope, Fault> {
    match value.as_int() {
        Some(0) => Ok(nvs_ldap::Scope::Base),
        Some(1) => Ok(nvs_ldap::Scope::One),
        None | Some(2) => Ok(nvs_ldap::Scope::Subtree),
        // Unreachable from source: `scope` is a `Scope` case.
        Some(other) => Err(Fault::fatal(format!(
            "{SEARCH} expected a `Scope` case, got {other}"
        ))),
    }
}

/// A count option of `search`, which must fit a `u32` and be at least `least`.
fn count_option(value: &Value, option: &str, least: u32) -> Result<u32, Fault> {
    let given = value.as_int().unwrap_or_default();
    u32::try_from(given)
        .ok()
        .filter(|count| *count >= least)
        .ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{SEARCH}: `{option}` is {given}, and it must be between {least} and {}",
                    u32::MAX
                ),
            )
        })
}

nvs_runtime::nvs_helper! {
    /// `$connection->whoami(): string` — [`super::who_am_i`] for a Novis program.
    fn nvs_core_ldap_connection_whoami(ctx, args: [1]) {
        let key = key_in(args[0], &CONNECTION, CONNECTION_HANDLE_AT, "whoami")?;
        let who = super::who_am_i(ctx, key)?;
        Ok(Value::str(NvsStr::new(who.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `$connection->search(Filter $filter, {base?, scope?, select?, pageSize?,
    /// sizeLimit?}): Entries` — [`super::search`], with the search parked on
    /// the connection for the `Ldap\Entries` it returns.
    fn nvs_core_ldap_connection_search(ctx, args: [7]) {
        let key = key_in(args[0], &CONNECTION, CONNECTION_HANDLE_AT, "search")?;
        let filter = nvs_ldap::Filter::Encoded(ber_of(args[1], "search")?);
        let base = match optional_text(&args[2]) {
            Some(base) => base.to_owned(),
            None => super::base_of(ctx, key)?.ok_or_else(|| {
                Fault::thrown_as(
                    ThrownClass::Logic,
                    format!(
                        "{SEARCH}: the search has no `base`. Give one, such as \
                         `{{base: 'DC=example,DC=test'}}`, or set `base` in the `[ldap]` block"
                    ),
                )
            })?,
        };
        let scope = scope_of(&args[3])?;
        let select = selected(&args[4], SEARCH)?;
        let attributes: Vec<&str> = select.iter().map(String::as_str).collect();
        let request = nvs_ldap::SearchRequest {
            base: &base,
            scope,
            filter: &filter,
            attributes: &attributes,
            page_size: count_option(&args[5], "pageSize", 1)?,
            size_limit: count_option(&args[6], "sizeLimit", 0)?,
            time_limit: 0,
        };
        let id = super::search(ctx, key, &request)?.park(ctx)?;
        Ok(crate::instance::build(
            &ENTRIES,
            [Value::uint(key), Value::uint(id), Value::null(), Value::null()],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$connection->read(string $dn, {select?}): ?Entry` — [`super::read`]
    /// for a Novis program.
    fn nvs_core_ldap_connection_read(ctx, args: [3]) {
        let key = key_in(args[0], &CONNECTION, CONNECTION_HANDLE_AT, "read")?;
        let dn = args[1].as_text().ok_or_else(|| {
            // Unreachable from source: the parameter is a `string`.
            Fault::fatal(format!("{READ} expected a `string` DN"))
        })?.to_owned();
        let select = selected(&args[2], READ)?;
        let attributes: Vec<&str> = select.iter().map(String::as_str).collect();
        Ok(super::read(ctx, key, &dn, &attributes)?.map_or_else(Value::null, entry_value))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<Ldap\Entry>::iterate()` — the receiver itself, because the
    /// next entry may not have arrived yet.
    fn nvs_core_ldap_entries_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &ENTRIES, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Ldap\Entry>::advance(): bool` — reads the next entry into
    /// the receiver, asking the server for the next page when this one is
    /// used up, and returns `false` after the last one.
    fn nvs_core_ldap_entries_advance(ctx, args: [1]) {
        let member = nvs_runtime::sequence::ADVANCE;
        let stepped = (|| {
            let key = key_in(args[0], &ENTRIES, ENTRIES_HANDLE_AT, member)?;
            let id = key_in(args[0], &ENTRIES, ENTRIES_SEARCH_AT, member)?;
            let receiver = crate::instance::receiver(args[0], &ENTRIES, member)?;
            let next = super::step(ctx, key, id);
            // The entry the last step parked is freed here unless the loop
            // body still holds it, so a walk holds one entry at a time.
            let (parked, more) = match next {
                Ok(Step::Entry(entry)) => (entry_value(entry), true),
                Ok(Step::Ended(urls)) => {
                    crate::instance::set_slot(receiver, ENTRIES_REFERENCES_AT, url_list(&urls));
                    (Value::null(), false)
                }
                Ok(Step::Gone) => (Value::null(), false),
                Err(fault) => {
                    crate::instance::set_slot(receiver, ENTRIES_ENTRY_AT, Value::null());
                    return Err(fault);
                }
            };
            crate::instance::set_slot(receiver, ENTRIES_ENTRY_AT, parked);
            Ok(Value::bool(more))
        })();
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Ldap\Entry>::current(): Ldap\Entry` — the entry the last
    /// `advance()` read.
    fn nvs_core_ldap_entries_current(_ctx, args: [1]) {
        let read = crate::instance::read_slot(
            args,
            &ENTRIES,
            ENTRIES_ENTRY_AT,
            nvs_runtime::sequence::CURRENT,
        );
        crate::cursor::consume(args[0]);
        read
    }
}

nvs_runtime::nvs_helper! {
    /// `$entries->references(): array<tainted string>` — the continuation
    /// references the search returned, none of them followed: the list the
    /// last `advance()` kept, or what the parked search has so far.
    fn nvs_core_ldap_entries_references(ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &ENTRIES, "references")?;
        if crate::instance::slot(receiver, ENTRIES_REFERENCES_AT).array_ptr().is_some() {
            return crate::instance::read_slot(args, &ENTRIES, ENTRIES_REFERENCES_AT, "references");
        }
        let key = key_in(args[0], &ENTRIES, ENTRIES_HANDLE_AT, "references")?;
        let id = key_in(args[0], &ENTRIES, ENTRIES_SEARCH_AT, "references")?;
        Ok(url_list(&super::references(ctx, key, id)?.unwrap_or_default()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->toArray(): array<array<tainted bytes>>` — the attributes
    /// slot itself, keyed by each name as the server spelled it.
    fn nvs_core_ldap_entry_to_array(_ctx, args: [1]) {
        crate::instance::read_slot(args, &ENTRY, ENTRY_ATTRIBUTES_AT, "toArray")
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->dn(): string` — the entry's DN as the server sent it.
    fn nvs_core_ldap_entry_dn(_ctx, args: [1]) {
        crate::instance::read_slot(args, &ENTRY, ENTRY_DN_AT, "dn")
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->has(string $name): bool` — whether the entry has a value for
    /// the attribute.
    fn nvs_core_ldap_entry_has(_ctx, args: [2]) {
        let (_, values) = values_named(args, "has")?;
        Ok(Value::bool(values.is_some()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->string(string $name): ?tainted string` — the attribute's one
    /// value as text.
    fn nvs_core_ldap_entry_string(_ctx, args: [2]) {
        let (name, values) = values_named(args, "string")?;
        let Some(values) = values else {
            return Ok(Value::null());
        };
        text_value(only_value(&values, "string", name)?, "string", name)
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->strings(string $name): ?array<tainted string>` — every value
    /// of the attribute as text, in the order the server sent them.
    fn nvs_core_ldap_entry_strings(_ctx, args: [2]) {
        let (name, values) = values_named(args, "strings")?;
        let Some(values) = values else {
            return Ok(Value::null());
        };
        let mut out = NvsArray::new();
        let mut at = values.next_slot(0);
        while let Some(slot) = at {
            let value = values.value_at(slot).unwrap_or_else(Value::null);
            out.append(text_value(value, "strings", name)?);
            at = values.next_slot(slot + 1);
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `$entry->bytes(string $name): ?tainted bytes` — the attribute's one
    /// value as the server sent it.
    fn nvs_core_ldap_entry_bytes(_ctx, args: [2]) {
        let (name, values) = values_named(args, "bytes")?;
        let Some(values) = values else {
            return Ok(Value::null());
        };
        Ok(retained(only_value(&values, "bytes", name)?))
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::equals(string $attribute, tainted string $value): Filter`
    /// — RFC 4511's equality match, encoded now.
    fn nvs_core_ldap_filter_equals(_ctx, args: [2]) {
        compared(args,r"Core\Ldap\Filter::equals", nvs_ldap::Filter::Equal)
    }
}

/// A filter comparing the attribute in `args[0]` with the value in `args[1]`.
fn compared(
    args: &[Value],
    member: &str,
    make: fn(String, Vec<u8>) -> nvs_ldap::Filter,
) -> Result<Value, Fault> {
    let attribute = attribute_arg(&args[0], member)?;
    let value = value_arg(&args[1], member)?;
    Ok(filter_value(&make(attribute.to_owned(), value.to_vec())))
}

/// A substring filter on the attribute in `args[0]` whose one part, the value
/// in `args[1]`, `place` puts at the start, in the middle or at the end.
fn substring(args: &[Value], member: &str, place: Place) -> Result<Value, Fault> {
    let attribute = attribute_arg(&args[0], member)?.to_owned();
    let part = part_arg(&args[1], member)?.to_vec();
    let filter = match place {
        Place::Start => nvs_ldap::Filter::Substrings {
            attribute,
            initial: Some(part),
            any: Vec::new(),
            last: None,
        },
        Place::Middle => nvs_ldap::Filter::Substrings {
            attribute,
            initial: None,
            any: vec![part],
            last: None,
        },
        Place::End => nvs_ldap::Filter::Substrings {
            attribute,
            initial: None,
            any: Vec::new(),
            last: Some(part),
        },
    };
    Ok(filter_value(&filter))
}

/// Where [`substring`] puts its one part.
#[derive(Clone, Copy)]
enum Place {
    Start,
    Middle,
    End,
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::startsWith(string $attribute, tainted string $value): Filter`
    /// — RFC 4511's substring match with an initial part.
    fn nvs_core_ldap_filter_starts_with(_ctx, args: [2]) {
        substring(args,r"Core\Ldap\Filter::startsWith", Place::Start)
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::endsWith(string $attribute, tainted string $value): Filter`
    /// — RFC 4511's substring match with a final part.
    fn nvs_core_ldap_filter_ends_with(_ctx, args: [2]) {
        substring(args,r"Core\Ldap\Filter::endsWith", Place::End)
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::contains(string $attribute, tainted string $value): Filter`
    /// — RFC 4511's substring match with one middle part.
    fn nvs_core_ldap_filter_contains(_ctx, args: [2]) {
        substring(args,r"Core\Ldap\Filter::contains", Place::Middle)
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::atLeast(string $attribute, tainted string $value): Filter`
    /// — RFC 4511's `greaterOrEqual`, by the attribute's own ordering rule.
    fn nvs_core_ldap_filter_at_least(_ctx, args: [2]) {
        compared(args,r"Core\Ldap\Filter::atLeast", nvs_ldap::Filter::GreaterOrEqual)
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::atMost(string $attribute, tainted string $value): Filter`
    /// — RFC 4511's `lessOrEqual`.
    fn nvs_core_ldap_filter_at_most(_ctx, args: [2]) {
        compared(args,r"Core\Ldap\Filter::atMost", nvs_ldap::Filter::LessOrEqual)
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::approx(string $attribute, tainted string $value): Filter`
    /// — RFC 4511's `approxMatch`, which the server defines.
    fn nvs_core_ldap_filter_approx(_ctx, args: [2]) {
        compared(args,r"Core\Ldap\Filter::approx", nvs_ldap::Filter::Approx)
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::all(Filter ...$filters): Filter` — RFC 4511's `and`,
    /// whose children are the encodings the arguments already carry.
    fn nvs_core_ldap_filter_all(_ctx, args: [1]) {
        let filters = filters_arg(&args[0], r"Core\Ldap\Filter::all")?;
        Ok(filter_value(&nvs_ldap::Filter::And(filters)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::any(Filter ...$filters): Filter` — RFC 4511's `or`.
    fn nvs_core_ldap_filter_any(_ctx, args: [1]) {
        let filters = filters_arg(&args[0], r"Core\Ldap\Filter::any")?;
        Ok(filter_value(&nvs_ldap::Filter::Or(filters)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::not(Filter $filter): Filter` — RFC 4511's `not`.
    fn nvs_core_ldap_filter_not(_ctx, args: [1]) {
        let inner = ber_of(args[0], r"Core\Ldap\Filter::not")?;
        let filter = nvs_ldap::Filter::Not(Box::new(nvs_ldap::Filter::Encoded(inner)));
        Ok(filter_value(&filter))
    }
}

nvs_runtime::nvs_helper! {
    /// `$filter->toString(): tainted string` — the RFC 4515 text of the
    /// encoding, rendered by [`nvs_ldap::Filter::to_text`]. Nothing sends it.
    fn nvs_core_ldap_filter_to_string(_ctx, args: [1]) {
        let ber = ber_of(args[0], "toString")?;
        let text = nvs_ldap::Filter::Encoded(ber).to_text();
        Ok(Value::str(NvsStr::new(text.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Ldap\Filter::present(string $attribute): Filter` — RFC 4511's
    /// presence match: the entry has any value for the attribute.
    fn nvs_core_ldap_filter_present(_ctx, args: [1]) {
        let attribute = attribute_arg(&args[0], r"Core\Ldap\Filter::present")?;
        Ok(filter_value(&nvs_ldap::Filter::Present(attribute.to_owned())))
    }
}

/// The address of one of this module's symbols, or `None` for a symbol that
/// belongs to another module. See [`crate::address`].
pub(super) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_ldap_connection_whoami" => (nvs_core_ldap_connection_whoami as *const ()).cast(),
        "nvs_core_ldap_connection_search" => (nvs_core_ldap_connection_search as *const ()).cast(),
        "nvs_core_ldap_connection_read" => (nvs_core_ldap_connection_read as *const ()).cast(),
        ENTRIES_ITERATE_SYMBOL => (nvs_core_ldap_entries_iterate as *const ()).cast(),
        ENTRIES_ADVANCE_SYMBOL => (nvs_core_ldap_entries_advance as *const ()).cast(),
        ENTRIES_CURRENT_SYMBOL => (nvs_core_ldap_entries_current as *const ()).cast(),
        "nvs_core_ldap_entries_references" => {
            (nvs_core_ldap_entries_references as *const ()).cast()
        }
        "nvs_core_ldap_entry_to_array" => (nvs_core_ldap_entry_to_array as *const ()).cast(),
        "nvs_core_ldap_entry_dn" => (nvs_core_ldap_entry_dn as *const ()).cast(),
        "nvs_core_ldap_entry_has" => (nvs_core_ldap_entry_has as *const ()).cast(),
        "nvs_core_ldap_entry_string" => (nvs_core_ldap_entry_string as *const ()).cast(),
        "nvs_core_ldap_entry_strings" => (nvs_core_ldap_entry_strings as *const ()).cast(),
        "nvs_core_ldap_entry_bytes" => (nvs_core_ldap_entry_bytes as *const ()).cast(),
        "nvs_core_ldap_filter_equals" => (nvs_core_ldap_filter_equals as *const ()).cast(),
        "nvs_core_ldap_filter_present" => (nvs_core_ldap_filter_present as *const ()).cast(),
        "nvs_core_ldap_filter_starts_with" => {
            (nvs_core_ldap_filter_starts_with as *const ()).cast()
        }
        "nvs_core_ldap_filter_ends_with" => (nvs_core_ldap_filter_ends_with as *const ()).cast(),
        "nvs_core_ldap_filter_contains" => (nvs_core_ldap_filter_contains as *const ()).cast(),
        "nvs_core_ldap_filter_at_least" => (nvs_core_ldap_filter_at_least as *const ()).cast(),
        "nvs_core_ldap_filter_at_most" => (nvs_core_ldap_filter_at_most as *const ()).cast(),
        "nvs_core_ldap_filter_approx" => (nvs_core_ldap_filter_approx as *const ()).cast(),
        "nvs_core_ldap_filter_all" => (nvs_core_ldap_filter_all as *const ()).cast(),
        "nvs_core_ldap_filter_any" => (nvs_core_ldap_filter_any as *const ()).cast(),
        "nvs_core_ldap_filter_not" => (nvs_core_ldap_filter_not as *const ()).cast(),
        "nvs_core_ldap_filter_to_string" => (nvs_core_ldap_filter_to_string as *const ()).cast(),
        _ => return None,
    })
}
