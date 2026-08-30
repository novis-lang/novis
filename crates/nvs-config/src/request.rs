//! [ADR 0064] § 5: the configuration one request reads, and the overlay it writes over it.
//!
//! A request clones the published [`Snapshot`] once at start ([ADR 0078] § 1) and reads that clone
//! for its whole life. `Core\Config::set` never writes into it: what it writes is the
//! copy-on-write overlay this type holds beside it, so a value one request set is invisible to the
//! next request on the same core and to every request running beside it. That is the whole of what
//! makes the API request-local, and it is a property of the shape rather than of a discipline —
//! there is no `&mut Snapshot` anywhere for a member to reach.
//!
//! **A bare name is a limit's name.** § 5's API is keyed on a directive *name* and `m6.md`'s own
//! worked call is `Core\Config::set('memory', '512M')`, with no block in it. So a name with no dot
//! that [`value::unit_of`] knows is read and written in `[limits]` — the block ADR 0005 states the
//! request-settable default in — and every other name is the dotted path the file writes. This is
//! not a second rule: it is the empty block [`value::unit_of`] already answers for, made into the
//! one place that decides it.
//!
//! **What `set` refuses, it refuses by returning `false`** (ADR 0005, and `m6.md`'s *Verify*): a
//! name no [`Directive`](crate::directive::Directive) row governs, a `System` one, a value that
//! does not spell its unit, a value above the `[limits.hard]` ceiling, and a
//! [`RuntimeTighten`](Class::RuntimeTighten) one that does not narrow. Nothing on this path
//! throws, so a program cannot catch a refusal and cannot tell one from another — which is the API
//! ADR 0064 § 5 states and not an omission.
//!
//! **A `RuntimeTighten` directive that is not a quantity cannot be set at all**, and that is
//! deliberate. Narrowing is `[value] within [what is in force]`, which [`value::within_ceiling`]
//! answers for a size, a duration, a count or a ratio and for nothing else; `[capabilities]` is the
//! only such row today and a grant is a list. Refusing is the safe direction — a request that
//! cannot drop a capability is inconvenient, one that silently widens one is the failure ADR 0005
//! exists to prevent — and the capability model that will answer it properly is this milestone's
//! own Stage 4.
//!
//! Cost: one `Arc` clone per request, plus one `String` pair per key a request actually set.
//! O(in-flight requests) and never O(requests served).
//!
//! [ADR 0064]: ../../../docs/adr/0064-configuration-file-format.md
//! [ADR 0078]: ../../../docs/adr/0078-config-reload-and-control-socket.md

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::directive::{Class, lookup};
use crate::snapshot::{Snapshot, value_at};
use crate::tree::Setting;
use crate::value::{self, unit_of};

/// One request's view of the configuration: the snapshot it cloned at start, and what it has set
/// for itself since.
#[derive(Clone, Debug)]
pub struct Request {
    /// The snapshot serving when this request started. Shared, immutable, and dropped when the
    /// last request holding it finishes.
    base: Arc<Snapshot>,
    /// Every key `set` accepted, by canonical dotted name. Empty for a request that never called
    /// it, which is nearly all of them.
    overlay: BTreeMap<String, String>,
}

impl Request {
    /// The view a request starts with: `base`, and nothing set.
    #[must_use]
    pub fn new(base: Arc<Snapshot>) -> Self {
        Self {
            base,
            overlay: BTreeMap::new(),
        }
    }

    /// The snapshot underneath, for a reader that wants the whole tree rather than one directive —
    /// `[app] origin`, the block roster, the override record.
    #[must_use]
    pub fn snapshot(&self) -> &Arc<Snapshot> {
        &self.base
    }

    /// `Core\Config::get`: the value in force for `name`, or `None` where nothing set one.
    ///
    /// Values cross as text (§ 5), so a directive the file wrote as an integer, a float or a
    /// boolean answers with the way TOML spells it — `64`, `0.01`, `true`. A key naming a table or
    /// a list is `None` rather than a rendering nothing could set back.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<String> {
        let key = canonical(name);
        if let Some(set) = self.overlay.get(key.as_ref()) {
            return Some(set.clone());
        }
        value_at(&self.base.table, &key).and_then(as_text)
    }

    /// `Core\Config::set`: `true` when the change was made for this request, `false` when it was
    /// refused — the module doc lists the five refusals, and none of them throws.
    pub fn set(&mut self, name: &str, value: &str) -> bool {
        let key = canonical(name);
        let Some(row) = lookup(&key) else {
            return false;
        };
        if !row.class.settable_by_a_request() {
            return false;
        }
        let asked = Setting::Text(value.to_string());
        let bound = match row.class {
            // Up to the ceiling the operator kept, and freely below it (ADR 0005).
            Class::Runtime => self.ceiling(&key),
            // Narrowing only: what is in force *is* the ceiling, so the same comparison answers
            // both classes and a directive with nothing in force has nothing to narrow from.
            Class::RuntimeTighten => match self.in_force(&key) {
                Some(current) => Some(current),
                None => return false,
            },
            Class::System => return false,
        };
        match bound {
            Some(bound) => match value::within_ceiling(&key, &asked, &bound) {
                Ok(Some(true)) => {}
                // `Ok(None)` is a directive with no quantity to compare. A `Runtime` one is then
                // unbounded and accepted; a `RuntimeTighten` one cannot be shown to narrow, which
                // the module doc's fourth paragraph owns.
                Ok(None) if row.class == Class::Runtime => {}
                Ok(Some(false) | None) | Err(_) => return false,
            },
            // No ceiling stated. The value still has to spell its own unit, or `get` would answer
            // with something the boot path would have refused from the file.
            None => {
                if let Some(unit) = unit_of(&key)
                    && value::Quantity::parse(&key, unit, &asked).is_err()
                {
                    return false;
                }
            }
        }
        self.overlay.insert(key.into_owned(), value.to_string());
        true
    }

    /// `Core\Config::restore`: drops what this request set for `name`, leaving the snapshot's own
    /// value in force again. A name this request never set is not an error — there is nothing to
    /// undo and nothing to report.
    pub fn restore(&mut self, name: &str) {
        self.overlay.remove(canonical(name).as_ref());
    }

    /// `Core\Config::all`: every directive in force for this request, by dotted name, with what
    /// this request set folded over the snapshot.
    ///
    /// One entry per scalar leaf, on [`get`](Self::get)'s rule: a table is not a value and a list
    /// has no rendering `set` would take back.
    #[must_use]
    pub fn all(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        flatten(&self.base.table, "", &mut out);
        for (key, value) in &self.overlay {
            out.insert(key.clone(), value.clone());
        }
        out
    }

    /// The `[limits.hard]` ceiling bounding `key`, or `None` where the operator stated none.
    ///
    /// It is read off the typed tree through [`app::ceilings`](crate::app::ceilings), which is the
    /// one table naming the five limits — a sixth is a row there and not a sixth place to forget.
    fn ceiling(&self, key: &str) -> Option<Setting> {
        let leaf = key.strip_prefix("limits.")?;
        if leaf.contains('.') {
            // `limits.hard.*` is `System` and never reaches here; anything else under `limits`
            // that is not one of the five has no ceiling to find.
            return None;
        }
        let hard = self.base.config.limits.as_ref()?.hard.as_ref()?;
        crate::app::ceilings(hard)
            .into_iter()
            .find(|(name, _)| *name == leaf)
            .and_then(|(_, ceiling)| ceiling.cloned())
    }

    /// The value currently in force at a canonical key, as the boot path would have read it.
    fn in_force(&self, key: &str) -> Option<Setting> {
        if let Some(set) = self.overlay.get(key) {
            return Some(Setting::Text(set.clone()));
        }
        value_at(&self.base.table, key)?.clone().try_into().ok()
    }
}

/// The dotted key a `Core\Config` name resolves to — the module doc's *A bare name is a limit's
/// name*, in the one place both `get` and `set` reach it.
fn canonical(name: &str) -> Cow<'_, str> {
    if !name.contains('.') && unit_of(name).is_some() {
        return Cow::Owned(format!("limits.{name}"));
    }
    Cow::Borrowed(name)
}

/// A TOML scalar as the text § 5 crosses it as, and `None` for anything that is not one.
fn as_text(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(text) => Some(text.clone()),
        toml::Value::Integer(number) => Some(number.to_string()),
        toml::Value::Float(number) => Some(number.to_string()),
        toml::Value::Boolean(flag) => Some(flag.to_string()),
        toml::Value::Datetime(stamp) => Some(stamp.to_string()),
        toml::Value::Array(_) | toml::Value::Table(_) => None,
    }
}

/// Every scalar leaf of `table`, by dotted key, appended to `out`.
fn flatten(table: &toml::Table, prefix: &str, out: &mut BTreeMap<String, String>) {
    for (name, value) in table {
        let key = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        match value {
            toml::Value::Table(nested) => flatten(nested, &key, out),
            other => {
                if let Some(text) = as_text(other) {
                    out.insert(key, text);
                }
            }
        }
    }
}
