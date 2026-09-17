//! `Core\Config` — `rule:config/ini-set-is-core-config-set`
//! 's four members, and PHP's `ini_get` family with the free functions taken
//! off it (`rule:classes/no-free-functions-or-constants`).
//!
//! Every member is four lines long, because none of the rules is here. What a
//! name resolves to, what a `set` is allowed to do and where the ceiling comes
//! from all live in `nvs_config::request`, whose module doc is the one home for
//! them; this module marshals a `string` in and a `string` out and does nothing
//! else. That split is the reason `rule:config/ini-set-is-core-config-set`'s "the registry parses the
//! string with the same parser the boot path uses" is true by construction: the
//! parser is not reachable from here.
//!
//! **The context is the configuration.** `Ctx::config` holds the snapshot the
//! request cloned at start plus its own overlay
//! (`rule:config/the-config-is-an-immutable-snapshot`
//! ), so a `set` moves one request's view and is invisible to the next
//! request on the same core. No member here reaches a process-wide table,
//! because there is none to reach.
//!
//! **A context nobody configured answers as an empty configuration**: `get` is
//! `null`, `all` is empty, `set` is `false` and `restore` does nothing. That is
//! every test context and any embedder that has not built a snapshot, and it is
//! deliberately not a throw — `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 3 makes "no configuration
//! file anywhere" a valid host and not an error, so a program has to be able to
//! ask on one.

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc, Qual};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Config";

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "get",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Text(Qual::Neutral)),
            symbol: "nvs_core_config_get",
            doc: Some(&GET_DOC),
        },
        CoreMethod {
            name: "set",
            names: &["name", "value"],
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_config_set",
            doc: Some(&SET_DOC),
        },
        CoreMethod {
            name: "restore",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_config_restore",
            doc: Some(&RESTORE_DOC),
        },
        CoreMethod {
            name: "all",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
            symbol: "nvs_core_config_all",
            doc: Some(&ALL_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Config::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "The configuration value in force for this request, replacing `ini_get`. A directive \
            this request set with `set` answers with that value; everything else answers with \
            what the configuration file resolved to.",
    params: &[ParamDoc {
        name: "name",
        desc: "The directive's dotted name, `log.level`. A bare limit name — `memory`, \
               `cpu_time`, `wall_time`, `max_tasks`, `max_output`, `max_regex_steps` — names the \
               `[limits]` entry.",
        shape: &[],
    }],
    ret: "The value as text, however the file spelled it, or `null` where nothing set one and \
          for a name that holds a table or a list rather than a value.",
    errors: &[],
};

/// `Core\Config::set`'s reference card — `rule:core-api/reference-card`.
const SET_DOC: MethodDoc = MethodDoc {
    short: "Sets a directive for this request only, replacing `ini_set`. The change is written to \
            a copy-on-write overlay over the configuration snapshot, so it is invisible to every \
            other request and is gone when this one ends.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The directive's name, read exactly as `get` reads it.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value, written the way the configuration file would write it — `512M`, \
                   `30s`, `64`.",
            shape: &[],
        },
    ],
    ret: "`true` when the change was made. `false`, with the previous value left in place, for a \
          directive `nvs.toml` alone may set, for one no directive registry row governs, for a \
          value that does not spell the quantity its name takes, and for a value above the \
          `[limits.hard]` ceiling the operator kept. A refusal never throws.",
    errors: &[],
};

/// `Core\Config::restore`'s reference card — `rule:core-api/reference-card`.
const RESTORE_DOC: MethodDoc = MethodDoc {
    short: "Drops what this request set for a directive, putting the configuration file's own \
            value back in force — `ini_restore`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The directive's name, read exactly as `get` reads it.",
        shape: &[],
    }],
    ret: "Nothing. A name this request never set is not an error: there is nothing to undo.",
    errors: &[],
};

/// `Core\Config::all`'s reference card — `rule:core-api/reference-card`.
const ALL_DOC: MethodDoc = MethodDoc {
    short: "Every directive in force for this request, keyed by dotted name, with what this \
            request set folded over the file — `ini_get_all`.",
    params: &[],
    ret: "An `array<string, string>` in name order, one entry per value; a name holding a table \
          contributes its leaves and a name holding a list contributes nothing, on `get`'s rule.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_config_get" => (nvs_core_config_get as *const ()).cast(),
        "nvs_core_config_set" => (nvs_core_config_set as *const ()).cast(),
        "nvs_core_config_restore" => (nvs_core_config_restore as *const ()).cast(),
        "nvs_core_config_all" => (nvs_core_config_all as *const ()).cast(),
        _ => return None,
    })
}

/// One `string` argument, or the engine fault a wrongly-tagged one is: the
/// signature is checked at compile time, so a bad tag here is a lowering bug
/// and not something a program can provoke.
fn text<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Config::{member} expected {:?} for its name, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Config::get(string $name): ?string` — replacing `ini_get`.
    fn nvs_core_config_get(ctx, args: [1]) {
        let name = text(&args[0], "get")?;
        match ctx.config().and_then(|config| config.get(name)) {
            Some(value) => Ok(Value::str(NvsStr::new(value.as_bytes()))),
            None => Ok(Value::null()),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Config::set(string $name, string $value): bool` — replacing
    /// `ini_set`.
    ///
    /// The `false` this can answer with is `rule:config/three-changeability-classes`'s refusal and never an
    /// exception: `m6.md`'s *Verify* pins a set above the `[limits.hard]`
    /// ceiling as `false` with the previous value intact, and a version of it
    /// that threw would be a different API.
    fn nvs_core_config_set(ctx, args: [2]) {
        let name = text(&args[0], "set")?;
        let value = text(&args[1], "set")?;
        // Cloned out of the arguments because the overlay is borrowed mutably
        // below and the arguments are borrowed from the same call frame; the
        // pair is two short strings, on a member no request calls in a loop.
        let (name, value) = (name.to_string(), value.to_string());
        let accepted = ctx
            .config_mut()
            .is_some_and(|config| config.set(&name, &value));
        // A ceiling the runtime caches off the snapshot moved with it — see
        // `nvs_runtime::Ctx::memory_limit`'s field doc for why the value is
        // held as an integer and refreshed here rather than parsed per poll.
        if accepted {
            ctx.refresh_limits();
        }
        Ok(Value::bool(accepted))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Config::restore(string $name): void` — replacing `ini_restore`.
    fn nvs_core_config_restore(ctx, args: [1]) {
        let name = text(&args[0], "restore")?.to_string();
        if let Some(config) = ctx.config_mut() {
            config.restore(&name);
        }
        ctx.refresh_limits();
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Config::all(): array<string, string>` — replacing `ini_get_all`.
    ///
    /// Name order, because the overlay and the tree it sits over are both
    /// ordered maps: two requests that set the same directives get the same
    /// array, which is what makes the result printable in a test.
    fn nvs_core_config_all(ctx, _args: [0]) {
        let mut out = NvsArray::new();
        if let Some(config) = ctx.config() {
            for (name, value) in config.all() {
                out.set(
                    NvsStr::new(name.as_bytes()),
                    Value::str(NvsStr::new(value.as_bytes())),
                );
            }
        }
        Ok(Value::array(out))
    }
}
