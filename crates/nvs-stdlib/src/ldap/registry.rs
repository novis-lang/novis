//! What `Core\Ldap` declares: its enums, and a reference card for each of them.
//!
//! Rows, not behaviour, as in `crate::db`'s registry: every symbol named here is
//! defined by a sibling, and a card sits directly after the row it describes.

use crate::registry::{CaseDoc, CoreEnum, EnumDoc};

/// [`ERROR_KIND`]'s fully-qualified name, written once for the row and every
/// message that quotes it.
///
/// `nvs_types::error_lib` spells it a second time, because that crate seeds
/// `Core\Ldap\LdapError::$kind`'s type and cannot reach a `pub(crate)` const
/// here. `the_ldap_kind_property_names_a_registered_enum` holds the two
/// spellings together.
pub(crate) const ERROR_KIND_NAME: &str = r"Core\Ldap\ErrorKind";

/// ADR 0278 § 10's `ErrorKind`, the registry half of [`nvs_ldap::Kind`].
///
/// **The two halves are one enum and the wire one is authoritative**: a
/// program matches on this table, and `nvs_ldap::Kind::of_result` is where a
/// result code and AD's sub-code become a kind. [`super::error_kind_value`]
/// joins them by name, and `every_ldap_kind_names_a_registered_case` holds the
/// join total.
///
/// The values are declaration ordinals in § 10's order and mean nothing else.
/// `Protocol` is last, and it is also what `nvs_ldap::Kind::of_result` gives a
/// result code it does not name, which is why a `LdapError` a program builds
/// itself starts at that case (`nvs_ir::lower::exception`'s `LDAP_KIND_PROTOCOL`
/// restates the ordinal).
pub(crate) const ERROR_KIND: CoreEnum = CoreEnum {
    name: ERROR_KIND_NAME,
    cases: &[
        ("InvalidCredentials", 0),
        ("AccountDisabled", 1),
        ("AccountLocked", 2),
        ("PasswordExpired", 3),
        ("MustChangePassword", 4),
        ("AccountExpired", 5),
        ("NotAllowedNow", 6),
        ("EncryptionRequired", 7),
        ("PasswordPolicy", 8),
        ("NoSuchObject", 9),
        ("AlreadyExists", 10),
        ("InsufficientAccess", 11),
        ("ConstraintViolation", 12),
        ("SizeLimitExceeded", 13),
        ("TimeLimitExceeded", 14),
        ("Referral", 15),
        ("ReadOnly", 16),
        ("Unsupported", 17),
        ("Unavailable", 18),
        ("Timeout", 19),
        ("Protocol", 20),
    ],
    doc: Some(&ERROR_KIND_DOC),
};

/// [`ERROR_KIND`]'s reference card — `rule:core-api/reference-card`.
const ERROR_KIND_DOC: EnumDoc = EnumDoc {
    short: "Why an LDAP operation failed. `Core\\Ldap\\LdapError::$kind` is one of these, and \
            `$code` beside it is the LDAP result code the server sent.",
    cases: &[
        CaseDoc {
            name: "InvalidCredentials",
            desc: "The login or the password is wrong. An account that does not exist gives this \
                   kind too, so the error does not show which accounts exist.",
        },
        CaseDoc {
            name: "AccountDisabled",
            desc: "The account is disabled. Active Directory sends this as `data 533`.",
        },
        CaseDoc {
            name: "AccountLocked",
            desc: "The account is locked after too many wrong passwords. Active Directory sends \
                   this as `data 775`.",
        },
        CaseDoc {
            name: "PasswordExpired",
            desc: "The password has expired. Active Directory sends this as `data 532`.",
        },
        CaseDoc {
            name: "MustChangePassword",
            desc: "The user must set a new password before they can log in. Active Directory \
                   sends this as `data 773`.",
        },
        CaseDoc {
            name: "AccountExpired",
            desc: "The account has expired. Active Directory sends this as `data 701`.",
        },
        CaseDoc {
            name: "NotAllowedNow",
            desc: "The account may not log in at this time or from this computer. Active \
                   Directory sends this as `data 530` or `data 531`.",
        },
        CaseDoc {
            name: "EncryptionRequired",
            desc: "The server needs an encrypted connection for this operation.",
        },
        CaseDoc {
            name: "PasswordPolicy",
            desc: "The new password does not meet the directory's password rules.",
        },
        CaseDoc {
            name: "NoSuchObject",
            desc: "The entry, or the base of the search, does not exist.",
        },
        CaseDoc {
            name: "AlreadyExists",
            desc: "The entry, or the attribute value, already exists.",
        },
        CaseDoc {
            name: "InsufficientAccess",
            desc: "The connection's account is not allowed to do this.",
        },
        CaseDoc {
            name: "ConstraintViolation",
            desc: "The directory's schema does not allow this value or this change.",
        },
        CaseDoc {
            name: "SizeLimitExceeded",
            desc: "The search found more entries than the server or the search allows.",
        },
        CaseDoc {
            name: "TimeLimitExceeded",
            desc: "The search ran longer than the server allows.",
        },
        CaseDoc {
            name: "Referral",
            desc: "The server sent a referral to another server. Novis never follows a referral.",
        },
        CaseDoc {
            name: "ReadOnly",
            desc: "The server only allows reads, and the operation was a write.",
        },
        CaseDoc {
            name: "Unsupported",
            desc: "The server does not support this operation or this option.",
        },
        CaseDoc {
            name: "Unavailable",
            desc: "The server cannot be reached, or it closed the connection.",
        },
        CaseDoc {
            name: "Timeout",
            desc: "The operation took longer than the connection's `timeout`.",
        },
        CaseDoc {
            name: "Protocol",
            desc: "The server sent a message the LDAP protocol does not allow, or a result code \
                   no other kind covers. A `LdapError` you create yourself has this kind.",
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Every [`nvs_ldap::Kind`], in § 10's order. The `match` below has no
    /// wildcard, so a kind added to that enum fails to compile here until it
    /// is listed.
    const EVERY_KIND: [nvs_ldap::Kind; 21] = {
        use nvs_ldap::Kind::*;
        [
            InvalidCredentials,
            AccountDisabled,
            AccountLocked,
            PasswordExpired,
            MustChangePassword,
            AccountExpired,
            NotAllowedNow,
            EncryptionRequired,
            PasswordPolicy,
            NoSuchObject,
            AlreadyExists,
            InsufficientAccess,
            ConstraintViolation,
            SizeLimitExceeded,
            TimeLimitExceeded,
            Referral,
            ReadOnly,
            Unsupported,
            Unavailable,
            Timeout,
            Protocol,
        ]
    };

    #[test]
    fn every_ldap_kind_names_a_registered_case() {
        let _: fn(nvs_ldap::Kind) = |kind| match kind {
            nvs_ldap::Kind::InvalidCredentials
            | nvs_ldap::Kind::AccountDisabled
            | nvs_ldap::Kind::AccountLocked
            | nvs_ldap::Kind::PasswordExpired
            | nvs_ldap::Kind::MustChangePassword
            | nvs_ldap::Kind::AccountExpired
            | nvs_ldap::Kind::NotAllowedNow
            | nvs_ldap::Kind::EncryptionRequired
            | nvs_ldap::Kind::PasswordPolicy
            | nvs_ldap::Kind::NoSuchObject
            | nvs_ldap::Kind::AlreadyExists
            | nvs_ldap::Kind::InsufficientAccess
            | nvs_ldap::Kind::ConstraintViolation
            | nvs_ldap::Kind::SizeLimitExceeded
            | nvs_ldap::Kind::TimeLimitExceeded
            | nvs_ldap::Kind::Referral
            | nvs_ldap::Kind::ReadOnly
            | nvs_ldap::Kind::Unsupported
            | nvs_ldap::Kind::Unavailable
            | nvs_ldap::Kind::Timeout
            | nvs_ldap::Kind::Protocol => {}
        };
        let named: Vec<&str> = EVERY_KIND.into_iter().map(nvs_ldap::Kind::name).collect();
        let registered: Vec<&str> = ERROR_KIND.cases.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            named, registered,
            "`nvs_ldap::Kind` and `{ERROR_KIND_NAME}` are one enum"
        );
        for (ordinal, kind) in EVERY_KIND.into_iter().enumerate() {
            let expected = i64::try_from(ordinal).expect("twenty-one cases");
            assert_eq!(
                super::super::error_kind_value(kind).as_int(),
                Some(expected)
            );
        }
    }
}
