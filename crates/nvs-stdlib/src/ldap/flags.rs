//! `Ldap\Ad\AccountFlags` and `Ldap\Ad\GroupType`: AD's two flag fields, as readonly objects with one `bool` reader per flag and `bits()`
//!
//! ADR 0278 §§ 1 and 8, `rule:core-classes/ldap-value-types`. An object keeps
//! the integer AD wrote in its `bits` slot, so a bit no reader names is kept
//! and `bits()` returns it unchanged. Each reader finds its bit by its own
//! name in [`nvs_ldap::value::ACCOUNT_FLAGS`] or
//! [`nvs_ldap::value::GROUP_TYPE_FLAGS`], so the table there is the one list
//! of what a flag is.
//!
//! **Readers rather than properties**, for the reason
//! [`crate::registry::CoreTy::Instance`] states: a `Core` instance has no
//! property a program can reach.
//!
//! **Lockout and an expired password are computed by AD**, not stored in
//! `userAccountControl`. `AccountFlags` keeps
//! `msDS-User-Account-Control-Computed` in its `computed` slot, which `search`
//! requests whenever `userAccountControl` is selected, and `lockedOut` and
//! `passwordExpired` read it. Where the entry did not carry it the slot is
//! `null`, and the two read `bits` as every other flag does.

use nvs_ldap::value::{ACCOUNT_FLAGS as ACCOUNT_BITS, COMPUTED_ACCOUNT_FLAGS, Flag};
use nvs_ldap::value::{GROUP_TYPE_FLAGS as GROUP_BITS, has_flag};
use nvs_runtime::{Fault, Value};

use super::{ACCOUNT_FLAGS, FLAGS_BITS_AT, FLAGS_COMPUTED_AT, GROUP_TYPE};
use crate::registry::CoreClass;

/// An `Ad\AccountFlags` over `userAccountControl`'s `bits` and, where the
/// entry carried it, the computed attribute's.
pub(super) fn account_flags_value(bits: i64, computed: Option<i64>) -> Value {
    crate::instance::build(
        &ACCOUNT_FLAGS,
        [
            Value::int(bits),
            computed.map_or_else(Value::null, Value::int),
        ],
    )
}

/// An `Ad\GroupType` over `groupType`'s `bits`.
pub(super) fn group_type_value(bits: i64) -> Value {
    crate::instance::build(&GROUP_TYPE, [Value::int(bits)])
}

/// The integer in slot `at` of a `class` receiver, or `None` for `null`.
fn int_in(value: Value, class: &CoreClass, at: usize, member: &str) -> Result<Option<i64>, Fault> {
    let receiver = crate::instance::receiver(value, class, member)?;
    Ok(crate::instance::slot(receiver, at).as_int())
}

/// The `bits` a `class` receiver keeps.
fn bits_of(value: Value, class: &CoreClass, member: &str) -> Result<i64, Fault> {
    int_in(value, class, FLAGS_BITS_AT, member)?.ok_or_else(|| {
        // Unreachable from source: only this module builds these classes.
        Fault::fatal(format!(
            "{}::{member} expected an `int` in its `bits` slot",
            class.name
        ))
    })
}

/// The reader named `member`: whether its bit is set.
fn flag(args: &[Value], class: &CoreClass, flags: &[Flag], member: &str) -> Result<Value, Fault> {
    let bit = flags
        .iter()
        .find(|flag| flag.name == member)
        .map(|flag| flag.bit)
        .ok_or_else(|| {
            // Unreachable from source: every reader row names a flag.
            Fault::fatal(format!("{}::{member} names no flag", class.name))
        })?;
    let mut bits = bits_of(args[0], class, member)?;
    if class.name == ACCOUNT_FLAGS.name
        && bit & COMPUTED_ACCOUNT_FLAGS != 0
        && let Some(computed) = int_in(args[0], class, FLAGS_COMPUTED_AT, member)?
    {
        bits = computed;
    }
    Ok(Value::bool(has_flag(bits, bit)))
}

/// One helper per reader, and the `address()` arms for all of them and for
/// the two `bits()` bodies below.
macro_rules! flag_helpers {
    ($($class:ident, $flags:ident => [$($symbol:ident => $name:literal),* $(,)?];)*) => {
        $($(
            nvs_runtime::nvs_helper! {
                /// One flag reader, found by its name by [`flag`].
                fn $symbol(_ctx, args: [1]) {
                    flag(args, &$class, $flags, $name)
                }
            }
        )*)*

        /// The address of one of this module's symbols, or `None` for a
        /// symbol that belongs to another module. See [`crate::address`].
        pub(super) fn address(symbol: &str) -> Option<*const u8> {
            Some(match symbol {
                $($(stringify!($symbol) => ($symbol as *const ()).cast(),)*)*
                "nvs_core_ldap_ad_account_flags_bits" => {
                    (nvs_core_ldap_ad_account_flags_bits as *const ()).cast()
                }
                "nvs_core_ldap_ad_group_type_bits" => {
                    (nvs_core_ldap_ad_group_type_bits as *const ()).cast()
                }
                _ => return None,
            })
        }
    };
}

flag_helpers! {
    ACCOUNT_FLAGS, ACCOUNT_BITS => [
        nvs_core_ldap_ad_account_flags_script => "script",
        nvs_core_ldap_ad_account_flags_disabled => "disabled",
        nvs_core_ldap_ad_account_flags_home_directory_required => "homeDirectoryRequired",
        nvs_core_ldap_ad_account_flags_locked_out => "lockedOut",
        nvs_core_ldap_ad_account_flags_password_not_required => "passwordNotRequired",
        nvs_core_ldap_ad_account_flags_reversible_encryption => "reversibleEncryption",
        nvs_core_ldap_ad_account_flags_temporary_duplicate_account => "temporaryDuplicateAccount",
        nvs_core_ldap_ad_account_flags_normal_account => "normalAccount",
        nvs_core_ldap_ad_account_flags_interdomain_trust_account => "interdomainTrustAccount",
        nvs_core_ldap_ad_account_flags_workstation_trust_account => "workstationTrustAccount",
        nvs_core_ldap_ad_account_flags_server_trust_account => "serverTrustAccount",
        nvs_core_ldap_ad_account_flags_password_never_expires => "passwordNeverExpires",
        nvs_core_ldap_ad_account_flags_mns_logon_account => "mnsLogonAccount",
        nvs_core_ldap_ad_account_flags_smartcard_required => "smartcardRequired",
        nvs_core_ldap_ad_account_flags_trusted_for_delegation => "trustedForDelegation",
        nvs_core_ldap_ad_account_flags_not_delegated => "notDelegated",
        nvs_core_ldap_ad_account_flags_use_des_key_only => "useDesKeyOnly",
        nvs_core_ldap_ad_account_flags_no_preauth_required => "noPreauthRequired",
        nvs_core_ldap_ad_account_flags_password_expired => "passwordExpired",
        nvs_core_ldap_ad_account_flags_trusted_to_auth_for_delegation => "trustedToAuthForDelegation",
        nvs_core_ldap_ad_account_flags_partial_secrets_account => "partialSecretsAccount",
    ];
    GROUP_TYPE, GROUP_BITS => [
        nvs_core_ldap_ad_group_type_system => "system",
        nvs_core_ldap_ad_group_type_global => "global",
        nvs_core_ldap_ad_group_type_domain_local => "domainLocal",
        nvs_core_ldap_ad_group_type_universal => "universal",
        nvs_core_ldap_ad_group_type_app_basic => "appBasic",
        nvs_core_ldap_ad_group_type_app_query => "appQuery",
        nvs_core_ldap_ad_group_type_security => "security",
    ];
}

nvs_runtime::nvs_helper! {
    /// `$flags->bits(): int` — `userAccountControl` as AD wrote it, with
    /// every bit no reader names.
    fn nvs_core_ldap_ad_account_flags_bits(_ctx, args: [1]) {
        Ok(Value::int(bits_of(args[0], &ACCOUNT_FLAGS, "bits")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `$type->bits(): int` — `groupType` as AD wrote it, which is negative
    /// for a security group.
    fn nvs_core_ldap_ad_group_type_bits(_ctx, args: [1]) {
        Ok(Value::int(bits_of(args[0], &GROUP_TYPE, "bits")?))
    }
}
