//! `rule:config/ini-set-is-core-config-set`: the configuration one request reads, and the overlay it writes over it.
//!
//! A request clones the published [`Snapshot`] once at start (`rule:config/the-config-is-an-immutable-snapshot`) and reads that clone
//! for its whole life. `Core\Config::set` never writes into it: what it writes is the
//! copy-on-write overlay this type holds beside it, so a value one request set is invisible to the
//! next request on the same core and to every request running beside it. That is the whole of what
//! makes the API request-local, and it is a property of the shape rather than of a discipline —
//! there is no `&mut Snapshot` anywhere for a member to reach.
//!
//! **A bare name is a limit's name.** § 5's API is keyed on a directive *name* and `m6.md`'s own
//! worked call is `Core\Config::set('memory', '512M')`, with no block in it. So a name with no dot
//! that [`value::unit_of`] knows is read and written in `[limits]` — the block `rule:config/three-changeability-classes` states the
//! request-settable default in — and every other name is the dotted path the file writes. This is
//! not a second rule: it is the empty block [`value::unit_of`] already answers for, made into the
//! one place that decides it.
//!
//! **What `set` refuses, it refuses by returning `false`** (`rule:config/three-changeability-classes`, and `m6.md`'s *Verify*): a
//! name no [`Directive`](crate::directive::Directive) row governs, a name a row governs that is not
//! a leaf key the file may write (`log`, `log.x`), a `System` one, a value that
//! does not spell its unit, a value above the `[limits.hard]` ceiling, a
//! [`RuntimeTighten`](Class::RuntimeTighten) one that does not narrow, an assignment that would
//! leave this request holding one of `rule:http-server/cors-is-closed-until-origins-are-named` and `rule:http-server/cookies-are-secure-httponly-and-lax`'s meaningless `[http]` pairs, and a mode
//! outside `rule:http-server/the-mode-ceiling-defaults-to-the-startup-mode`'s ceiling — a name that is not one of the two modes being outside every
//! ceiling. Nothing on this path throws, so a program cannot catch a refusal and cannot tell one
//! from another — which is the API `rule:config/ini-set-is-core-config-set` states and not an omission.
//!
//! **A `RuntimeTighten` directive that is not a quantity cannot be set at all**, and that is
//! deliberate. Narrowing is `[value] within [what is in force]`, which [`value::within_ceiling`]
//! answers for a size, a duration, a count or a ratio and for nothing else; `[capabilities]` is the
//! only such row and a grant is a list. Refusing is the safe direction — a request that
//! cannot drop a capability is inconvenient, one that silently widens one is the failure `rule:config/three-changeability-classes`
//! exists to prevent — and the capability model that will answer it properly is this milestone's
//! own Stage 4.
//!
//! Cost: one `Arc` clone per request, plus one `String` pair per key a request actually set.
//! O(in-flight requests) and never O(requests served).
//!

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use crate::directive::{Class, lookup};
use crate::mode;
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
    ///
    /// A secret is read before the table and not out of it (`rule:config/a-secret-is-a-file-whose-content-is-the-value`): `db.main.password` is a
    /// key in force whose value is a file's content, and the table holds only the `password_file`
    /// that named it. Nothing here redacts — this is the reader the value exists for, and the
    /// program asking is the one that will connect with it.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<String> {
        let key = canonical(name);
        if let Some(set) = self.overlay.get(key.as_ref()) {
            return Some(set.clone());
        }
        if let Some(secret) = self.base.secrets.get(key.as_ref()) {
            return Some(secret.value.clone());
        }
        value_at(&self.base.table, &key).and_then(as_text)
    }

    /// `Core\Config::set`: `true` when the change was made for this request, `false` when it was
    /// refused — the module doc lists what is refused, and none of it throws.
    pub fn set(&mut self, name: &str, value: &str) -> bool {
        let key = canonical(name);
        // A name under a settable block that is not one of its keys — `log`, `log.x`,
        // `limits..memory` — is still governed by that block's row, but no reader ever asks for it,
        // so a set of it would change nothing and say `true`.
        if !is_key(&key) {
            return false;
        }
        let Some(row) = lookup(&key) else {
            return false;
        };
        if !row.class.settable_by_a_request() {
            return false;
        }
        // `rule:config/a-program-may-read-and-flip-its-mode`'s flip is a `Runtime` set with a bound of its own — `[mode] ceiling` and not
        // `[limits.hard]` — and an effect beyond its own key, so it leaves the quantity path here
        // rather than threading two more conditions through it.
        if key == mode::KEY {
            return self.flip_mode(value);
        }
        let asked = Setting::Text(value.to_string());
        let bound = match row.class {
            // Up to the ceiling the operator kept, and freely below it (`rule:config/three-changeability-classes`).
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
                // the module doc's paragraph on a `RuntimeTighten` directive that is not a quantity
                // owns.
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
        if !self.stays_meaningful(&key, value) {
            return false;
        }
        self.overlay.insert(key.into_owned(), value.to_string());
        true
    }

    /// `rule:config/a-program-may-read-and-flip-its-mode`'s mode flip: the mode this request runs in, and § 3's five defaults with it.
    ///
    /// Two things make this more than an overlay insert, and both are in that ADR rather than in a
    /// choice made here:
    ///
    /// - **§ 5's ceiling bounds it.** A flip is allowed exactly where the mode asked for is no more
    ///   permissive than `[mode] ceiling`, which when unset is the mode the host started in — so a
    ///   production host that wrote no configuration refuses every flip, and the refusal is the
    ///   ordinary `false` [`set`](Self::set) answers with rather than a new failure shape.
    /// - **§ 4's last bullet re-derives § 3's rows into the overlay**, because a mode *is* those
    ///   five defaults and a flip that moved only its own key would change nothing observable. A row
    ///   already set explicitly is left alone, and "explicitly" covers the file as well as this
    ///   request: § 3's first property makes `mode = "development"` beside `[log] format = "json"`
    ///   legal and reads it as the operator overriding the default, which a re-derivation that
    ///   stomped the written line would silently undo.
    ///
    /// The flip is request-local like every other `set`, which is § 4's own answer for why allowing
    /// it is safe at all: nothing here is visible to another request or outlives this one.
    fn flip_mode(&mut self, asked: &str) -> bool {
        if !mode::within(asked, &self.started_ceiling()) {
            return false;
        }
        for row in mode::DERIVED {
            let Some(derived) = row.value(asked) else {
                continue;
            };
            if self.overlay.contains_key(row.key) || value_at(&self.base.table, row.key).is_some() {
                continue;
            }
            self.overlay
                .insert(row.key.to_string(), derived.to_string());
        }
        self.overlay
            .insert(mode::KEY.to_string(), asked.to_string());
        true
    }

    /// `rule:config/a-mode-is-five-defaults`'s run mode in force for this request — where `Core\Env::mode`'s answer comes
    /// from, and the whole of it.
    ///
    /// Deliberately **not** `get(mode::KEY)`. That reader answers the overlay and then the global
    /// key and knows nothing about the `[[app]]` block, so a request inside an application that
    /// selected its own mode (`rule:config/a-mount-routes-and-an-app-block-sets-policy`) would be told the host's instead of its own. The order
    /// is the flip this request made (§ 4), then [`started`](Self::started).
    ///
    /// Borrowed wherever the answer is already text in the snapshot or the overlay, so asking
    /// allocates nothing: `Core\Env::mode` returns an enum, and a program asks it per request.
    #[must_use]
    pub fn mode(&self) -> Cow<'_, str> {
        match self.overlay.get(mode::KEY) {
            Some(flipped) => Cow::Borrowed(flipped),
            None => self.started(),
        }
    }

    /// The mode the host started this request in — [`mode`](Self::mode) with the flip taken off.
    ///
    /// The `[[app]]` block's where one matched, since an application's mode is its own (`rule:config/a-mount-routes-and-an-app-block-sets-policy`
    /// ), then the global `mode.default`, then `production`, which is § 5's row for a host that
    /// wrote nothing at all.
    fn started(&self) -> Cow<'_, str> {
        if let Some(app) = &self.base.mode {
            return Cow::Borrowed(app);
        }
        match value_at(&self.base.table, mode::KEY) {
            Some(toml::Value::String(text)) => Cow::Borrowed(text),
            Some(other) => as_text(other).map_or(Cow::Borrowed(mode::PRODUCTION), Cow::Owned),
            None => Cow::Borrowed(mode::PRODUCTION),
        }
    }

    /// `rule:http-server/the-mode-ceiling-defaults-to-the-startup-mode`'s ceiling: `[mode] ceiling` where the tree states one, else the mode the host
    /// started in.
    ///
    /// Read off the snapshot and never off the overlay, which is the whole of what makes the ceiling
    /// unraisable: a request that had flipped itself once would otherwise be measured against what it
    /// had already asked for. That is the one difference between the fallback here and
    /// [`mode`](Self::mode), and the reason both go through [`started`](Self::started) rather than
    /// stating the chain twice.
    fn started_ceiling(&self) -> String {
        if let Some(stated) = value_at(&self.base.table, "mode.ceiling").and_then(as_text) {
            return stated;
        }
        self.started().into_owned()
    }

    /// `rule:http-server/cors-is-closed-until-origins-are-named` and `rule:http-server/cookies-are-secure-httponly-and-lax`, asked of what this request would be left holding.
    ///
    /// The same two combinations the boot refuses, refused here as `false` with the value unchanged
    /// — § 2 states both halves and [`http`](crate::http) is the one place the condition is
    /// written, so this reads those values off the snapshot, folds this request's overlay and
    /// then the proposed assignment over them, and asks. A key under neither block returns before
    /// any of that, `[errors] deprecated` with whether [`crate::errors::Deprecated::of`] names it.
    fn stays_meaningful(&self, key: &str, value: &str) -> bool {
        if key == crate::errors::KEY {
            return crate::errors::Deprecated::of(value).is_some();
        }
        if !key.starts_with("http.cors.") && !key.starts_with("http.cookies.") {
            return true;
        }
        let mut inbound = crate::http::Inbound::of(self.base.config.http.as_ref());
        for (set, held) in &self.overlay {
            inbound.assign(set, held);
        }
        inbound.assign(key, value);
        inbound.meaningless().is_none()
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
    /// has no rendering `set` would take back. § 7's secrets are folded in for the same reason they
    /// are in `get` — a key this answers `None` for and `get` answers a value for would be two
    /// readers disagreeing about what is set — so `all` and `get` are one answer set.
    #[must_use]
    pub fn all(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        flatten(&self.base.table, "", &mut out);
        for (key, secret) in &self.base.secrets {
            out.insert(key.clone(), secret.value.clone());
        }
        for (key, value) in &self.overlay {
            out.insert(key.clone(), value.clone());
        }
        out
    }

    /// The `[limits.hard]` ceiling bounding `key`, or `None` where the operator stated none.
    ///
    /// It is read off the typed tree through [`app::ceilings`](crate::app::ceilings), which is the
    /// one table naming the bounded limits — another one is a row there and not a new place to
    /// forget.
    fn ceiling(&self, key: &str) -> Option<Setting> {
        let leaf = key.strip_prefix("limits.")?;
        if leaf.contains('.') {
            // `limits.hard.*` is `System` and never reaches here; anything else under `limits`
            // that is not one of them has no ceiling to find.
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

/// Whether `key` is a leaf key an `nvs.toml` may write: one of the setting lines of the shipped
/// default file, which `bun nv directives --check-template` holds to the typed tree key for key.
///
/// That file is the one roster of leaf keys this crate carries, so reading it here keeps no second
/// list in step. The keys it writes under an example name — `db.main.path` — are in the set too,
/// and no request-settable row governs one.
///
/// Cost: the set is built once per process, on the first `set`, and is a few kilobytes.
fn is_key(key: &str) -> bool {
    static KEYS: OnceLock<BTreeSet<String>> = OnceLock::new();
    KEYS.get_or_init(|| leaf_keys(crate::default_file()))
        .contains(key)
}

/// Every setting line's dotted key in `file`: `#name = …` under the nearest `[block]` or
/// `[[entry]]` header above it, whether that header is live or commented out.
fn leaf_keys(file: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let mut block = "";
    for line in file.lines() {
        let line = line.trim();
        let setting = line.strip_prefix('#').unwrap_or(line);
        if let Some(header) = setting.strip_prefix('[') {
            block = header
                .split(']')
                .next()
                .unwrap_or_default()
                .trim_start_matches('[');
            continue;
        }
        // Prose is `# text`. A setting is `#name = value`, with no space after the `#`, so a line
        // whose name is not a bare lower-case key is prose.
        let Some((name, _)) = setting.split_once('=') else {
            continue;
        };
        let name = name.trim_end();
        let bare = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_';
        if name.is_empty() || !name.bytes().all(bare) {
            continue;
        }
        keys.insert(if block.is_empty() {
            name.to_string()
        } else {
            format!("{block}.{name}")
        });
    }
    keys
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
