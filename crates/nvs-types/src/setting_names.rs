//! The names a program passes to `Core\Config::setting` and `Core\Config::secret`, recorded while
//! compiling so `nvs config check` can warn about a key no program reads (`W1028`,
//! `rule:config/settings-and-secrets-are-a-programs-own-values`).
//!
//! **Only a written name is recorded.** A name the program computes, a `...` spread, or a method
//! reference to either member reads a key nobody can name before it runs. Such a call sets
//! [`SettingNames::computed`], and `nvs config check` then judges no key at all: a warning about a
//! key the computed name does read would be wrong.
//!
//! Nothing here changes how the call is typed or run. The boot never compiles a program to ask this,
//! so the table costs one small set per compile and is read only by `nvs config check`.

use std::collections::BTreeSet;

use nvs_hir::QName;
use nvs_syntax::ast::CallArgs;

use crate::Env;
use crate::defaults::ConstArg;
use crate::ty::Ty;

/// The class whose `setting` and `secret` these are.
const OWNER: &str = "Core\\Config";

/// What one program reads of the four open blocks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingNames {
    settings: BTreeSet<String>,
    secrets: BTreeSet<String>,
    computed: bool,
}

impl SettingNames {
    /// Whether some call passed a name that is not written in the source, so no key can be judged.
    #[must_use]
    pub fn computed(&self) -> bool {
        self.computed
    }

    /// Whether a written name reads the setting at the dotted `key`: the key itself, or a table
    /// above it, which `setting` returns whole.
    #[must_use]
    pub fn reads_setting(&self, key: &str) -> bool {
        self.settings.iter().any(|name| {
            key.strip_prefix(name.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
        })
    }

    /// Whether a written name reads the secret `name`. A secret has no sub-table, so only the same
    /// name reads it.
    #[must_use]
    pub fn reads_secret(&self, name: &str) -> bool {
        self.secrets.contains(name)
    }

    /// Adds what `other` reads, so one table answers for every program a tree names.
    pub fn extend(&mut self, other: &Self) {
        self.settings.extend(other.settings.iter().cloned());
        self.secrets.extend(other.secrets.iter().cloned());
        self.computed |= other.computed;
    }
}

/// Records the name a `Core\Config::setting` or `secret` call reads, or that it computes one.
///
/// The hook `crate::expr::calls` reaches for a static call after the target has resolved, beside
/// [`crate::secret_directive::types_secret`].
pub(crate) fn record(owner: &QName, member: &str, args: &CallArgs, env: &mut Env<'_>) {
    let secret = match member {
        "setting" => false,
        "secret" => true,
        _ => return,
    };
    if owner.to_string() != OWNER {
        return;
    }
    let CallArgs::List(list) = args else {
        // A method reference is called later with a name nobody wrote here.
        env.exprs.setting_names_mut().computed = true;
        return;
    };
    // Both members have one parameter, so a `name:` argument fills it as a positional one does.
    let Some(arg) = list.first() else {
        return;
    };
    let declared = env.interner.intern(Ty::String);
    let written = match arg.spread {
        true => None,
        false => crate::defaults::literal_default(&arg.value, declared, env),
    };
    let names = env.exprs.setting_names_mut();
    match written {
        Some(ConstArg::Str(name)) if secret => {
            names.secrets.insert(name);
        }
        Some(ConstArg::Str(name)) => {
            names.settings.insert(name);
        }
        _ => names.computed = true,
    }
}
