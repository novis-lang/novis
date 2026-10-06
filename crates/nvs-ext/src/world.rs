//! The `nvs:ext` world's own imports beside `types`: `nvs:ext/log` and `nvs:ext/settings`
//! (ADR 0246 § 2 and § 9).
//!
//! - `log` — `write(level, message)` writes `message` to the calling request's log through
//!   [`Budget::log`], at the WIT level's [`Level`], with the extension's class as the channel.
//! - `settings` — `get(key)` reads the key from the extension's `[ext.<name>]` block in the
//!   calling request's configuration snapshot, through [`Budget::setting`]. A key the block does
//!   not set reads as its declared default, and a key the manifest does not declare, or a guest
//!   whose manifest declares no block, reads `none`. The value crosses in the type the manifest
//!   declares for the key. A block's keys and their types are checked once, at boot, by
//!   [`Settings::check`], so a value of another type only reaches here from a budget that skipped
//!   that check, and it reads as the default.
//!
//! Neither import grants anything: a setting narrows what a component does, and § 7 is the only
//! place I/O is granted.

use serde_json::Value;
use wasmtime::component::{Linker, Val};

use crate::call::{Budget, Guest, Level};
use crate::manifest::{SettingType, Settings};

/// Defines `nvs:ext/log` and `nvs:ext/settings` in `linker`.
///
/// # Errors
///
/// When a definition is refused, which is a name defined twice.
pub(crate) fn link(linker: &mut Linker<Guest>) -> wasmtime::Result<()> {
    linker
        .instance("nvs:ext/log@1.0.0")?
        .func_new("write", |store, _, params, _| {
            let [Val::Enum(level), Val::String(message)] = params else {
                wasmtime::bail!("`write` takes a level and a message");
            };
            let level = match level.as_str() {
                "debug" => Level::Debug,
                "info" => Level::Info,
                "warn" => Level::Warn,
                "error" => Level::Error,
                "critical" => Level::Critical,
                other => wasmtime::bail!("`{other}` is not a log level"),
            };
            let guest = store.data();
            guest.budget().log(level, guest.wasi.channel(), message);
            Ok(())
        })?;
    linker
        .instance("nvs:ext/settings@1.0.0")?
        .func_new("get", |store, _, params, results| {
            let [Val::String(key)] = params else {
                wasmtime::bail!("`get` takes one key");
            };
            let guest = store.data();
            let value = guest
                .settings()
                .and_then(|settings| read(settings, guest.budget().as_ref(), key));
            if let Some(slot) = results.first_mut() {
                *slot = Val::Option(value.map(Box::new));
            }
            Ok(())
        })?;
    Ok(())
}

/// The setting `key` of `settings` as the guest reads it: the request's value, or the default.
fn read(settings: &Settings, budget: &dyn Budget, key: &str) -> Option<Val> {
    let declared = settings.keys.iter().find(|k| k.name == key)?;
    budget
        .setting(&settings.name, key)
        .and_then(|value| crossing(declared.ty, &value))
        .or_else(|| crossing(declared.ty, &declared.default))
}

/// `value` as `nvs:ext/settings`'s `setting` case for `ty`, or `None` when it is not of `ty`.
fn crossing(ty: SettingType, value: &Value) -> Option<Val> {
    let (case, payload) = match ty {
        SettingType::Bool => ("bool", Val::Bool(value.as_bool()?)),
        SettingType::Int => ("int", Val::S64(value.as_i64()?)),
        SettingType::Uint => ("uint", Val::U64(value.as_u64()?)),
        SettingType::Float => ("float", Val::Float64(value.as_f64()?)),
        SettingType::String => ("string", Val::String(value.as_str()?.to_owned())),
        SettingType::Strings => (
            "strings",
            Val::List(
                value
                    .as_array()?
                    .iter()
                    .map(|item| item.as_str().map(|s| Val::String(s.to_owned())))
                    .collect::<Option<_>>()?,
            ),
        ),
    };
    Some(Val::Variant(case.to_owned(), Some(Box::new(payload))))
}
