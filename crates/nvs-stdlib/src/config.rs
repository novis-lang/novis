//! `Core\Config` — `rule:config/ini-set-is-core-config-set`
//! 's six members, and PHP's `ini_get` family with the free functions taken
//! off it (`rule:classes/no-free-functions-or-constants`).
//!
//! # A program's own values
//!
//! `setting` and `secret` read `[settings]` and `[secrets]`
//! (`rule:config/settings-and-secrets-are-a-programs-own-values`), and the
//! other four never see those blocks. `setting` builds the value from its TOML
//! type, and a table is an array keyed by name. `secret` returns a `?secret
//! string`, and its name is a [`Qual::Sink`] for `Core\Env::secret`'s reason:
//! a request that chose the name would choose which credential it reads.
//! **What it spends:** one lookup in the request's snapshot and one value
//! built per call, and nothing read from disk.
//!
//! # The directives
//!
//! Every directive member is four lines long, because none of the rules is here. What a
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
//! **A host with no configuration file answers with the built-in defaults**:
//! `get` and `all` give every default in force, as `nvs_config::request` says.
//! **A context that was handed no configuration at all answers as an empty
//! one**: `get` is `null`, `all` is empty, `set` is `false` and `restore` does
//! nothing. That is a test context and any embedder that has not built a
//! snapshot, and it is deliberately not a throw — `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 3 makes "no configuration
//! file anywhere" a valid host and not an error, so a program has to be able to
//! ask on one.
//!
//! # A secret directive is read only by its written name
//!
//! `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s value is a
//! credential, and `secret` is a static qualifier the run time never sees. So
//! the checker types `get('db.main.password')` as `?secret string` when the
//! name is a literal [`nvs_config::secret::is_secret_key`] names, and says so
//! through `crate::registry::PREPARED_MEMBERS`' argument 0 as
//! [`PREPARED_SECRET`]. A name the program computed reaches `get` with no such
//! word, and a secret directive under it throws `LogicError`, because its
//! result would be a plain `?string` any sink takes. This is the one rule
//! `get` holds itself. **What it spends:** one walk of the secret rows per
//! call, and no memory.
//!
//! # A package may lower a limit and never raise one
//!
//! `rule:security/package-authority-is-granted-one-line-at-a-time` holds a
//! `set` written in a package's namespace below the value in force for every
//! `[limits]` key. Which namespace a call is in is known only while compiling,
//! so the checker hands a `set` written in the application's own code
//! [`PREPARED_APPLICATION`] as its argument 0, and every other call reaches
//! `nvs_config::Request::set_as_package`. A method reference carries no word,
//! so it can only lower, wherever it was written: the rule fails closed.
//! **What it spends:** one integer compare per call, and no memory.

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Config";

/// The word `crate::registry::PREPARED_MEMBERS`' argument 0 carries when
/// `get`'s name is a literal secret directive, so the checker typed the call
/// `?secret string`. Minted from the space [`crate::cldr::PREPARED_PATTERN`]
/// is, so a word meant for another member never reads as this one.
pub const PREPARED_SECRET: i64 = 4;

/// The word `crate::registry::PREPARED_MEMBERS`' argument 0 carries when a
/// `set` call is written in the application's own code: a namespace no
/// `[grants]` line matches. Any other word, a method reference's included,
/// reaches `set` as a package's call, which may lower a limit and never raise
/// one. Minted from the same space as [`PREPARED_SECRET`].
pub const PREPARED_APPLICATION: i64 = 5;

/// What `Core\Config::setting` returns when a block sets the name: each type a
/// TOML value can have. A union and not `mixed`, because text out of `mixed`
/// is `tainted` (`rule:security/taint-propagation`), and a setting is the
/// operator's and not the request's. A table's values are `mixed`, so a
/// program that wants one of them untainted reads it by its dotted name.
const SETTING: CoreTy = CoreTy::Union(&[
    CoreTy::Str,
    CoreTy::Int,
    CoreTy::Float,
    CoreTy::Bool,
    CoreTy::Array(&CoreTy::Mixed),
]);

/// `Core\Config`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reads the settings of `nvs.toml` for the current request. You can change some \
            settings for this request only, and reset them again.",
};

/// The registry row. See [`crate::registry::CLASSES`].
///
/// `rule:config/ini-set-is-core-config-set`: `ini_get`'s family are free
/// functions in PHP, so that rule decides what replaces them. [`crate::config`]
/// owns why every member of it is four lines long.
// rank: 640
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
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
        CoreMethod {
            name: "setting",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&SETTING),
            symbol: "nvs_core_config_setting",
            doc: Some(&SETTING_DOC),
        },
        CoreMethod {
            name: "secret",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::SecretStr),
            symbol: "nvs_core_config_secret",
            doc: Some(&SECRET_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Config::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Returns the value of one setting for this request. A setting this request changed with \
            `set` gives the new value. Any other setting gives the value from the configuration \
            file, or its built-in default.",
    params: &[ParamDoc {
        name: "name",
        desc: "The dotted name of the setting, such as `log.level`. A limit name alone is the \
               setting of that name in `[limits]`. The limit names are `memory`, `cpu_time`, \
               `wall_time`, `max_tasks`, `max_output` and `max_regex_steps`.",
        shape: &[],
    }],
    ret: "The value as text, written the way the configuration file writes it, such as `128M` or \
          `30s`. The result is `null` for a setting with no default, such as `wall_time`. It is \
          also `null` for a name that has a group of settings or a list. A secret setting, such \
          as `db.main.password` or `http.csrf_key`, gives a `?secret string`. Your program \
          cannot print it or write it to a log.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The name is a secret setting, and the call does not write it as a string literal. \
               Write the name in the call, such as `Core\\Config::get('db.main.password')`.",
    }],
};

/// `Core\Config::set`'s reference card — `rule:core-api/reference-card`.
const SET_DOC: MethodDoc = MethodDoc {
    short: "Changes a setting for this request only. Other requests do not see the change, and it \
            ends when this request ends.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The name of the setting, in the same form that `get` uses.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value, written the way the configuration file writes it, such as `512M`, \
                   `30s` or `64`.",
            shape: &[],
        },
    ],
    ret: "`true` when the change was made. `false` when it is not allowed, and the previous value \
          stays in place. It is not allowed for a setting that only `nvs.toml` may set, or for a \
          name that is not a setting. It is not allowed for a value that is not a valid size, \
          duration or count. It is not allowed for a value above the limit's ceiling. The \
          ceiling is `[limits.hard]`, or the value the program started with when \
          `[limits.hard]` does not set that limit. `set` never throws an error.",
    errors: &[],
};

/// `Core\Config::restore`'s reference card — `rule:core-api/reference-card`.
const RESTORE_DOC: MethodDoc = MethodDoc {
    short: "Deletes this request's change to one setting. The value from the configuration file \
            is in force again, or the built-in default when the file does not write the \
            setting.",
    params: &[ParamDoc {
        name: "name",
        desc: "The name of the setting, in the same form that `get` uses.",
        shape: &[],
    }],
    ret: "Nothing. A name this request never changed is not an error. There is nothing to undo.",
    errors: &[],
};

/// `Core\Config::all`'s reference card — `rule:core-api/reference-card`.
const ALL_DOC: MethodDoc = MethodDoc {
    short: "Returns every setting in force for this request, with the dotted names as keys. A \
            setting this request changed has the new value. A setting the configuration file does \
            not write has its built-in default.",
    params: &[],
    ret: "An `array<string, string>` sorted by name, with one entry per value. A name that has a \
          group of settings gives one entry for each setting in it. A name that has a list gives \
          no entry. A secret setting, such as `db.main.password` or `http.csrf_key`, has the \
          value `<secret>`. `get` returns the real value.",
    errors: &[],
};

/// `Core\Config::setting`'s reference card — `rule:core-api/reference-card`.
const SETTING_DOC: MethodDoc = MethodDoc {
    short: "Returns a value your program reads from the `[settings]` block of `nvs.toml`. A value \
            in the `[app.settings]` block of the application replaces the value in `[settings]`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The name of the value, such as `page_size`. A value in a table, such as \
               `[settings.api]` with `base = \"...\"`, has the name `api.base`.",
        shape: &[],
    }],
    ret: "The value with the type it has in the file. A TOML string is a `string`, an integer is \
          an `int`, a float is a `float` and a boolean is a `bool`. A TOML array is an array, and \
          a table is an array with the names as keys. The result is `null` when no block sets the \
          name. Use `as` to convert it: `Core\\Config::setting('page_size') as int` throws an \
          error when the value is missing or is not a whole number.",
    errors: &[],
};

/// `Core\Config::secret`'s reference card — `rule:core-api/reference-card`.
const SECRET_DOC: MethodDoc = MethodDoc {
    short: "Returns a secret your program reads from the `[secrets]` block of `nvs.toml`, such as \
            an API key. A secret in the `[app.secrets]` block of the application replaces the one \
            in `[secrets]`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The name of the secret, such as `api_key`. The block may write the value, or read \
               it from a file with `api_key_file`. The name cannot come from the request.",
        shape: &[],
    }],
    ret: "The secret as a `?secret string`. Your program cannot print it or write it to a log. You \
          can send it in an outgoing header, a form field or a database parameter. The result is \
          `null` when no block sets the name.",
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
        "nvs_core_config_setting" => (nvs_core_config_setting as *const ()).cast(),
        "nvs_core_config_secret" => (nvs_core_config_secret as *const ()).cast(),
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
    /// `Core\Config::get(string $name): ?string` — replacing `ini_get`. Its
    /// argument 0 is the prepared word, and a secret directive needs
    /// [`PREPARED_SECRET`] there, as the module doc says.
    fn nvs_core_config_get(ctx, args: [2]) {
        let name = text(&args[1], "get")?;
        if nvs_config::secret::is_secret_key(name) && args[0].as_int() != Some(PREPARED_SECRET) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "Core\\Config::get(): `{name}` is a secret setting, so the call must write \
                     its name as a string literal and the result is a `secret string`"
                ),
            ));
        }
        match ctx.config().and_then(|config| config.get(name)) {
            Some(value) => Ok(Value::text(&value)),
            None => Ok(Value::null()),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Config::set(string $name, string $value): bool` — replacing
    /// `ini_set`.
    ///
    /// The `false` this can answer with is `rule:config/three-changeability-classes`'s refusal and never an
    /// exception: `rule:config/ini-set-is-core-config-set` pins a set above the `[limits.hard]`
    /// ceiling as `false` with the previous value intact, and a version of it
    /// that threw would be a different API.
    ///
    /// Argument 0 is the prepared word. Without [`PREPARED_APPLICATION`] the
    /// call is a package's, and a `[limits]` key may only move down
    /// (`rule:security/package-authority-is-granted-one-line-at-a-time`).
    fn nvs_core_config_set(ctx, args: [3]) {
        let application = args[0].as_int() == Some(PREPARED_APPLICATION);
        let name = text(&args[1], "set")?;
        let value = text(&args[2], "set")?;
        // Cloned out of the arguments because the overlay is borrowed mutably
        // below and the arguments are borrowed from the same call frame; the
        // pair is two short strings, on a member no request calls in a loop.
        let (name, value) = (name.to_string(), value.to_string());
        let accepted = ctx.config_mut().is_some_and(|config| {
            if application {
                config.set(&name, &value)
            } else {
                config.set_as_package(&name, &value)
            }
        });
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
                    Value::text(&value),
                );
            }
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Config::setting(string $name): mixed` — a `[settings]` value
    /// with its TOML type, or `null` where no block writes it.
    fn nvs_core_config_setting(ctx, args: [1]) {
        let name = text(&args[0], "setting")?;
        Ok(ctx
            .config()
            .and_then(|config| config.setting(name))
            .map_or_else(Value::null, of_toml))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Config::secret(string $name): ?secret string` — a `[secrets]`
    /// value, or `null` where no block names it. The qualifier is the
    /// checker's alone: at run time a `secret string` is the same string.
    fn nvs_core_config_secret(ctx, args: [1]) {
        let name = text(&args[0], "secret")?;
        Ok(ctx
            .config()
            .and_then(|config| config.secret(name))
            .map_or_else(Value::null, Value::text))
    }
}

/// The Novis value a `[settings]` value is: each scalar its own type, an
/// array a list, and a table an array keyed by name.
fn of_toml(value: &toml::Value) -> Value {
    match value {
        toml::Value::String(text) => Value::text(text),
        toml::Value::Integer(int) => Value::int(*int),
        toml::Value::Float(float) => Value::float(*float),
        toml::Value::Boolean(flag) => Value::bool(*flag),
        // `nvs_config::settings::check_file` refuses a date or time at boot,
        // so this arm is unreachable from a loaded file. Its TOML text is the
        // value the refusal's help tells the operator to write instead.
        toml::Value::Datetime(when) => Value::text(&when.to_string()),
        toml::Value::Array(items) => {
            let mut out = NvsArray::with_capacity(items.len());
            for item in items {
                out.append(of_toml(item));
            }
            Value::array(out)
        }
        toml::Value::Table(table) => {
            let mut out = NvsArray::with_capacity(table.len());
            for (key, item) in table {
                out.set(NvsStr::new(key.as_bytes()), of_toml(item));
            }
            Value::array(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nvs_runtime::{Ctx, NvsArray, Value, call};

    use super::{
        PREPARED_APPLICATION, PREPARED_SECRET, nvs_core_config_all, nvs_core_config_get,
        nvs_core_config_restore, nvs_core_config_set,
    };

    /// A context configured out of the TOML an operator would have written.
    ///
    /// Both halves are built from that one text, the way a boot builds them:
    /// `get` and `all` read the table, and the `[limits.hard]` ceiling a `set`
    /// is bounded by is read off the typed tree beside it. A fixture that
    /// filled only one of them would answer plausibly and bound nothing.
    fn configured(written: &str) -> Ctx {
        let table: toml::Table = written.parse().expect("the fixture is valid TOML");
        let config = toml::Value::Table(table.clone())
            .try_into()
            .expect("the fixture deserializes into the tree it is written for");
        let mut ctx = Ctx::buffered();
        ctx.set_config(Arc::new(nvs_config::Snapshot {
            config,
            table,
            ..nvs_config::Snapshot::default()
        }));
        ctx
    }

    /// One `string` argument, as a call site builds it.
    fn text(value: &str) -> Value {
        Value::text(value)
    }

    /// What a member answered as text, or `None` where it answered `null`.
    fn answered_text(value: Value) -> Option<String> {
        let out = value.as_str_bytes().map(|bytes| {
            String::from_utf8(bytes.to_vec())
                .expect("`rule:types/bytes` guarantees a `string` is UTF-8")
        });
        #[expect(unsafe_code, reason = "this frame owns the reference the member built")]
        unsafe {
            value.release();
        }
        out
    }

    /// `Core\Config::get`'s answer for `name`, called the way lowering calls
    /// it for a literal name: argument 0 is [`PREPARED_SECRET`] when the
    /// literal names a secret directive, and the absent word otherwise.
    fn get(ctx: &mut Ctx, name: &str) -> Option<String> {
        let word = if nvs_config::secret::is_secret_key(name) {
            PREPARED_SECRET
        } else {
            crate::regex::PREPARED_NONE
        };
        let argument = text(name);
        let answered = call(nvs_core_config_get, ctx, &[Value::int(word), argument])
            .expect("`get` refuses nothing");
        let out = answered_text(answered);
        #[expect(
            unsafe_code,
            reason = "this frame owns the argument it built, and the member \
                      borrows rather than consumes it"
        )]
        unsafe {
            argument.release();
        }
        out
    }

    /// `Core\Config::set`'s answer for a name and a value, called from the
    /// application's own code.
    fn set(ctx: &mut Ctx, name: &str, value: &str) -> bool {
        set_with(ctx, PREPARED_APPLICATION, name, value)
    }

    /// `Core\Config::set` called from a package's namespace, which lowering
    /// hands the absent word.
    fn set_from_package(ctx: &mut Ctx, name: &str, value: &str) -> bool {
        set_with(ctx, crate::regex::PREPARED_NONE, name, value)
    }

    /// `Core\Config::set` with `word` as its argument 0.
    fn set_with(ctx: &mut Ctx, word: i64, name: &str, value: &str) -> bool {
        let args = [Value::int(word), text(name), text(value)];
        let answered =
            call(nvs_core_config_set, ctx, &args).expect("a refusal is `false` and never a fault");
        #[expect(
            unsafe_code,
            reason = "this frame owns the two arguments it built, and the \
                      member borrows rather than consumes them"
        )]
        unsafe {
            args[1].release();
            args[2].release();
        }
        answered.as_bool().expect("`set` answers a bool")
    }

    /// `Core\Config::restore` for one name.
    fn restore(ctx: &mut Ctx, name: &str) {
        let argument = text(name);
        let answered =
            call(nvs_core_config_restore, ctx, &[argument]).expect("`restore` refuses nothing");
        #[expect(
            unsafe_code,
            reason = "this frame owns the argument it built, and the answer is \
                      the `null` a `void` member leaves behind"
        )]
        unsafe {
            answered.release();
            argument.release();
        }
    }

    /// `Core\Config::all`'s answer, in the order the member reported it.
    fn all(ctx: &mut Ctx) -> Vec<(String, String)> {
        let answered = call(nvs_core_config_all, ctx, &[]).expect("`all` refuses nothing");
        #[expect(
            unsafe_code,
            reason = "this frame owns the array the member built, and the \
                      handle releases it"
        )]
        let array =
            unsafe { NvsArray::from_raw(answered.array_ptr().expect("`all` answers an array")) };
        array
            .keys()
            .iter()
            .map(|key| {
                let value = array.get(key).expect("a key `all` has just reported");
                let text = String::from_utf8(
                    value
                        .as_str_bytes()
                        .expect("`all` answers one `string` per entry")
                        .to_vec(),
                )
                .expect("`rule:types/bytes` guarantees a `string` is UTF-8");
                (
                    String::from_utf8(key.clone()).expect("a directive name is UTF-8"),
                    text,
                )
            })
            .collect()
    }

    /// `all` and `get` are one answer set, which is the claim no single line of
    /// either member shows: every name `all` reports answers the same from
    /// `get`, in name order, with what this request set folded over the file.
    ///
    /// The two shapes that are in neither answer are asserted here too, because
    /// a member that reported them would still look right on its own line: a
    /// name holding a group of settings is not a value, and a name holding a
    /// list has no spelling a `set` would take back.
    // covers: Core\Config::all
    #[test]
    fn all_and_get_report_one_answer_set_in_name_order() {
        let mut ctx = configured(
            "[limits]\nmemory = \"256M\"\nmax_tasks = 8\n\n[debug]\nmode = [\"coverage\"]\n",
        );
        assert!(
            set(&mut ctx, "log.level", "debug"),
            "the fixture's own set was refused, so the fold below asserts nothing"
        );

        let reported = all(&mut ctx);
        let names: Vec<&str> = reported.iter().map(|(name, _)| name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "`all` reports in name order");
        let value = |name: &str| {
            reported
                .iter()
                .find(|(reported, _)| reported == name)
                .map(|(_, value)| value.as_str())
        };
        assert_eq!(
            value("limits.max_tasks"),
            Some("8"),
            "a number is reported the way the file spelled it"
        );
        assert_eq!(value("limits.memory"), Some("256M"));
        assert_eq!(
            value("log.level"),
            Some("debug"),
            "the overlay is folded over the file"
        );
        assert_eq!(
            value("limits.max_regex_steps"),
            Some("1000000"),
            "a default the file does not write is in force, and reported"
        );
        assert_eq!(value("debug.mode"), None, "a list is not reported");

        for (name, value) in &reported {
            assert_eq!(
                get(&mut ctx, name).as_deref(),
                Some(value.as_str()),
                "`get` and `all` disagree about {name}"
            );
        }
        assert_eq!(get(&mut ctx, "limits").as_deref(), None);
        assert_eq!(get(&mut ctx, "debug.mode").as_deref(), None);
    }

    /// `restore` drops this request's own change and nothing else: the value
    /// the file gives is in force again rather than nothing, and a name the
    /// request never set is left exactly as the file wrote it.
    ///
    /// Every `.nvst` case over these two runs on a host with no configuration
    /// file, where the file's own value *is* nothing and the two outcomes
    /// cannot be told apart. A fixture with a table under it is what this side
    /// adds.
    // covers: Core\Config::get, Core\Config::restore
    #[test]
    fn restore_drops_this_requests_change_and_leaves_the_files_value() {
        let mut ctx = configured(
            "[limits]\nmemory = \"256M\"\n\n[limits.hard]\nmemory = \"512M\"\n\n[log]\nlevel = \"info\"\n",
        );
        assert_eq!(get(&mut ctx, "memory").as_deref(), Some("256M"));

        assert!(
            set(&mut ctx, "memory", "384M"),
            "the fixture's ceiling is above this set, so it is what the rest reads"
        );
        assert_eq!(
            get(&mut ctx, "limits.memory").as_deref(),
            Some("384M"),
            "a short limit name and its dotted spelling are one key"
        );

        restore(&mut ctx, "limits.memory");
        assert_eq!(
            get(&mut ctx, "memory").as_deref(),
            Some("256M"),
            "the file's own value is what comes back, not nothing"
        );

        restore(&mut ctx, "log.level");
        assert_eq!(
            get(&mut ctx, "log.level").as_deref(),
            Some("info"),
            "a name this request never set is left as the file wrote it"
        );
    }

    /// The `[limits.hard]` ceiling, asserted on both sides: the value at the
    /// ceiling is taken and the next one up is refused, with what was in force
    /// left alone. A `set` that stopped one step early reads plausibly against
    /// either half on its own.
    ///
    /// The ceiling the runtime enforces moves with the set it accepted, which
    /// is the whole reason the member refreshes the limits after one.
    // covers: Core\Config::set
    #[test]
    fn a_set_at_the_ceiling_is_taken_and_the_next_one_up_is_refused() {
        let mut ctx =
            configured("[limits]\nmemory = \"128M\"\n\n[limits.hard]\nmemory = \"512M\"\n");

        assert!(
            set(&mut ctx, "memory", "512M"),
            "the value at the ceiling is the last one allowed"
        );
        assert_eq!(get(&mut ctx, "memory").as_deref(), Some("512M"));
        // The figure itself is the ceiling less the limit handler's reserve,
        // which is `Ctx::memory_limit`'s own rule and not this member's.
        let enforced = ctx.memory_limit();
        assert!(
            enforced > 128 << 20,
            "the ceiling the runtime enforces did not move with the set"
        );

        assert!(
            !set(&mut ctx, "memory", "513M"),
            "one step above the ceiling is the first one refused"
        );
        assert_eq!(
            get(&mut ctx, "memory").as_deref(),
            Some("512M"),
            "a refusal leaves what was in force alone"
        );
        assert_eq!(
            ctx.memory_limit(),
            enforced,
            "a refused set moved the ceiling the runtime enforces"
        );
    }

    /// A context configured out of `written`, running as a request.
    fn serving(written: &str) -> Ctx {
        let mut ctx = configured(written);
        ctx.set_inbound(nvs_runtime::Inbound::new("GET", "/", ""));
        ctx
    }

    /// In server work with no `[limits.hard]` written, the ceiling is the
    /// `[limits]` value the run started with — the file's, or the default
    /// where the file writes none. A request can lower its own limit and
    /// cannot raise it, until the operator writes a hard ceiling above the
    /// start.
    // covers: Core\Config::set
    #[test]
    fn an_unwritten_hard_ceiling_is_the_value_the_run_started_with() {
        let mut ctx = serving("");
        assert!(
            !set(&mut ctx, "memory", "256M"),
            "a request raised its memory past the 128M default with no ceiling written"
        );
        assert!(
            set(&mut ctx, "memory", "128M"),
            "the start value itself is allowed"
        );
        assert!(set(&mut ctx, "memory", "64M"), "lowering is allowed");
        assert!(
            set(&mut ctx, "memory", "128M"),
            "a lowered limit goes back up to the start value"
        );

        let mut written = serving("[limits]\nmemory = \"32M\"\n");
        assert!(
            !set(&mut written, "memory", "64M"),
            "a request raised its memory past the value the file wrote"
        );

        let mut ceilinged = serving("[limits.hard]\nmemory = \"512M\"\n");
        assert!(
            set(&mut ceilinged, "memory", "256M"),
            "a written hard ceiling did not allow a raise below it"
        );
        assert!(!set(&mut ceilinged, "memory", "513M"));

        let mut removed = serving("[limits.hard]\nmemory = false\n");
        assert!(
            set(&mut removed, "memory", "4G"),
            "a hard `false` did not remove the ceiling"
        );
    }

    /// A CLI run keeps the 128M default, and its own code may raise it past
    /// that while no `[limits.hard] memory` is written. A written one bounds
    /// the raise exactly as it bounds a request's.
    // covers: Core\Config::set
    #[test]
    fn a_cli_run_raises_its_memory_freely_until_a_hard_ceiling_is_written() {
        let mut command = configured("");
        assert_eq!(get(&mut command, "memory").as_deref(), Some("128M"));
        assert_eq!(
            get(&mut command, "limits.hard.memory"),
            None,
            "a CLI run reported a hard memory ceiling nobody wrote"
        );
        assert!(
            set(&mut command, "memory", "2G"),
            "a CLI program could not raise its memory past 128M"
        );
        assert!(
            command.memory_limit() > 1 << 30,
            "the ceiling the runtime enforces did not move with the raise"
        );
        assert!(
            !set(&mut command, "memory", "lots"),
            "a value that does not spell a size was taken"
        );

        let mut ceilinged = configured("[limits.hard]\nmemory = \"512M\"\n");
        assert!(set(&mut ceilinged, "memory", "512M"));
        assert!(
            !set(&mut ceilinged, "memory", "513M"),
            "a CLI program raised its memory past the hard ceiling the file wrote"
        );
    }

    /// `rule:security/package-authority-is-granted-one-line-at-a-time`: on the
    /// CLI run where the application raises `memory` freely, a package's `set`
    /// lowers it and never raises it, even back to where it started.
    #[test]
    fn a_package_lowers_a_limit_and_never_raises_one() {
        let mut command = configured("");
        assert!(
            !set_from_package(&mut command, "memory", "2G"),
            "a package raised the memory limit"
        );
        assert_eq!(get(&mut command, "memory").as_deref(), Some("128M"));
        assert!(set_from_package(&mut command, "memory", "64M"));
        assert!(
            !set_from_package(&mut command, "memory", "128M"),
            "a package raised the memory limit back up"
        );
        assert!(
            set(&mut command, "memory", "2G"),
            "the application could not raise what a package lowered"
        );
        assert!(
            set_from_package(&mut command, "limits.memory", "1G"),
            "a package could not lower a limit the application raised"
        );
    }

    /// The memory-limit error names the run and the two ways to raise the
    /// limit: "the program" and an unbounded `Core\Config::set` on a CLI run,
    /// "the request" and a set bounded by `[limits.hard] memory` in server work.
    #[test]
    fn the_memory_limit_error_names_the_run_and_how_to_raise_it() {
        let breach = |ctx: Ctx| {
            let held = vec![1_u8; 4 << 20];
            let breach = ctx.memory_breach();
            drop(held);
            match breach {
                Some(nvs_runtime::Fault::Fatal(message)) => message.into_owned(),
                other => panic!("4M held under a 1M ceiling gave {other:?}"),
            }
        };
        let small = "[limits]\nmemory = \"1M\"\n";
        let command = breach(configured(small));
        assert!(
            command.starts_with("the program exceeded its memory limit"),
            "{command}"
        );
        assert!(
            command.ends_with(
                "help: raise `memory` under `[limits]` in nvs.toml, or call \
                 `Core\\Config::set('memory', '2M')` in the program"
            ),
            "{command}"
        );

        let request = breach(serving(small));
        assert!(
            request.starts_with("the request exceeded its memory limit"),
            "{request}"
        );
        assert!(
            request.ends_with("in the request, up to `[limits.hard] memory`"),
            "{request}"
        );
    }

    /// `cpu_time` has a start value only for server work. A CLI program has
    /// none, so nothing bounds its set; a request starts at 30 seconds and
    /// cannot raise itself past them.
    // covers: Core\Config::set
    #[test]
    fn an_unwritten_cpu_time_bounds_a_request_and_not_a_command() {
        let mut command = configured("");
        assert_eq!(command.cpu_limit(), 0, "a CLI program took a CPU ceiling");
        assert!(
            set(&mut command, "cpu_time", "1h"),
            "a CLI program with no CPU ceiling could not set one"
        );

        let mut request = configured("");
        request.set_inbound(nvs_runtime::Inbound::new("GET", "/", ""));
        assert_eq!(
            request.cpu_limit() + request.fatal_reserve_time(),
            30_000_000_000,
            "a request did not start at the 30s default"
        );
        assert!(
            !set(&mut request, "cpu_time", "60s"),
            "a request raised its CPU time past the default with no ceiling written"
        );
        assert!(set(&mut request, "cpu_time", "10s"), "lowering is allowed");
    }
}
