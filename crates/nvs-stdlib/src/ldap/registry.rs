//! What `Core\Ldap` declares: its classes, its enums, and a reference card for
//! each of them and for every member.
//!
//! Rows, not behaviour, as in `crate::db`'s registry: every symbol named here is
//! defined by a sibling, and a card sits directly after the row it describes.

use crate::registry::{
    CaseDoc, ClassDoc, Const, CoreClass, CoreEnum, CoreField, CoreMethod, CoreTy, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual, ShapeKeyDoc,
};

/// `Core\Ldap`'s fully-qualified name.
pub(crate) const NAME: &str = r"Core\Ldap";

/// `Core\Ldap\Connection`'s fully-qualified name.
pub(crate) const CONNECTION_NAME: &str = r"Core\Ldap\Connection";

/// [`TLS`]' fully-qualified name.
pub(crate) const TLS_NAME: &str = r"Core\Ldap\Tls";

/// The slot a [`CONNECTION`] keeps its key in, filed by
/// [`nvs_runtime::Ctx::hold_open_connection`].
const HANDLE_SLOT: &str = "handle";

/// ADR 0278 § 2's `Ldap\Settings`, the one shape `open` takes.
///
/// The qualifiers sit on the fields: `url` is a sink with no launderer, as
/// `rule:core-classes/db-capabilities` makes `Db\Settings.host` one, `user`
/// accepts `tainted` as a length-framed protocol field, and `password` is
/// `secret tainted string`. The ABI flattens the shape to one slot per field
/// in this order, and [`URL_ARG`] and the four consts after it are those slots.
pub(super) const SETTINGS: &[&[CoreField]] = &[&[
    CoreField {
        name: "url",
        ty: CoreTy::Text(Qual::Sink),
        default: None,
    },
    CoreField {
        name: "user",
        ty: CoreTy::TaintedStr,
        // No `user` is an anonymous session that never binds.
        default: Some(Const::Null),
    },
    CoreField {
        name: "password",
        ty: CoreTy::SecretTaintedStr,
        default: Some(Const::Null),
    },
    CoreField {
        name: "tls",
        ty: CoreTy::Enum(TLS_NAME),
        // `Tls::Required`, which the helper reads an absent value as.
        default: Some(Const::Null),
    },
    CoreField {
        name: "timeout",
        ty: CoreTy::Instance(crate::time::DURATION_NAME),
        default: Some(Const::Null),
    },
]];

/// [`SETTINGS`]' `url` slot in `open`'s arguments.
pub(super) const URL_ARG: usize = 0;
/// [`SETTINGS`]' `user` slot.
pub(super) const USER_ARG: usize = 1;
/// [`SETTINGS`]' `password` slot.
pub(super) const PASSWORD_ARG: usize = 2;
/// [`SETTINGS`]' `tls` slot.
pub(super) const TLS_ARG: usize = 3;
/// [`SETTINGS`]' `timeout` slot.
pub(super) const TIMEOUT_ARG: usize = 4;

/// `Core\Ldap`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Opens a connection to an LDAP directory, such as Active Directory. The name of a \
            directory in the server's configuration is enough, or the program can give the \
            settings itself.",
};

/// `Core\Ldap\Connection`'s class card — `rule:core-api/reference-card`.
const CONNECTION_CARD: ClassDoc = ClassDoc {
    short: "An open connection to one LDAP directory. The connection is closed when the request \
            ends.",
};

/// ADR 0278 § 1's `Core\Ldap`: `connect` and `open`.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "connect",
            names: &["name"],
            // A sink, for `Core\Db::connect`'s reason: the name selects which
            // operator-written credential the program binds with.
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Instance(CONNECTION_NAME),
            symbol: "nvs_core_ldap_connect",
            doc: Some(&CONNECT_DOC),
        },
        CoreMethod {
            name: "open",
            names: &["settings"],
            params: &[CoreTy::Shape(SETTINGS)],
            defaults: &[],
            return_ty: CoreTy::Instance(CONNECTION_NAME),
            symbol: "nvs_core_ldap_open",
            doc: Some(&OPEN_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Ldap::connect`'s reference card — `rule:core-api/reference-card`.
const CONNECT_DOC: MethodDoc = MethodDoc {
    short: "Opens the directory named by an `[ldap.<name>]` block in `nvs.toml`. A second call \
            with the same name in one request returns the same connection. Needs the \
            `ldap.connect` capability for that name.",
    params: &[ParamDoc {
        name: "name",
        desc: "The block to open: `\"corp\"` is `[ldap.corp]`.",
        shape: &[],
    }],
    ret: "A `Core\\Ldap\\Connection`, bound as the block's `user`. It is closed when the request \
          ends.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`ldap.connect` does not grant `$name`, or no `[ldap.<name>]` block has that \
                   name.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "This core already has the block's `pool.max` connections open, and none was \
                   free within the block's `timeout`.",
        },
        ErrorDoc {
            error: "Core\\Ldap\\LdapError",
            desc: "No URL in the block answered, or the server did not accept the block's user \
                   and password. `$kind` says why.",
        },
    ],
};

/// `Core\Ldap::open`'s reference card — `rule:core-api/reference-card`.
const OPEN_DOC: MethodDoc = MethodDoc {
    short: "Opens a directory at a URL the program gives. Needs the `ldap.open` capability for \
            the URL's host. The host's address is also checked against the addresses no \
            program may reach, such as private networks.",
    params: &[ParamDoc {
        name: "settings",
        desc: "The directory to open and the account to log in with.",
        shape: &[
            ShapeKeyDoc {
                key: "url",
                ty: "string",
                desc: "One `ldaps://` or `ldap://` URL. It cannot be `tainted`, because the \
                       password is sent to this host.",
            },
            ShapeKeyDoc {
                key: "user",
                ty: "tainted string",
                desc: "The account to log in as, as a DN, `user@example.test` or \
                       `EXAMPLE\\user`. Left out, the connection does not log in.",
            },
            ShapeKeyDoc {
                key: "password",
                ty: "secret tainted string",
                desc: "The account's password. An empty password throws before anything is \
                       sent.",
            },
            ShapeKeyDoc {
                key: "tls",
                ty: "Tls",
                desc: "`Tls::Required`, the default, encrypts the connection. `Tls::None` sends \
                       everything as plain text, and needs the host in `[capabilities.ldap] \
                       cleartext`.",
            },
            ShapeKeyDoc {
                key: "timeout",
                ty: "Duration",
                desc: "How long one operation may take. Left out, it is 30 seconds.",
            },
        ],
    }],
    ret: "A `Core\\Ldap\\Connection`. It is closed when the request ends.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`url` is not an `ldap://` or `ldaps://` URL, `ldap.open` does not grant its \
                   host, or the host's address is one no program may reach.",
        },
        ErrorDoc {
            error: "Core\\Ldap\\LdapError",
            desc: "The server did not answer, the connection is not encrypted and the host may \
                   not use plain text, or the server did not accept the user and password. \
                   `$kind` says why.",
        },
    ],
};

/// ADR 0278 § 1's `Ldap\Connection`: what `connect` and `open` return.
///
/// Its one slot is the key the connection is held under in the request's own
/// table, which [`super::held`] reads back. The members that run on it are
/// owed, and the goal's handoff names them.
pub(crate) const CONNECTION: CoreClass = CoreClass {
    name: CONNECTION_NAME,
    doc: Some(&CONNECTION_CARD),
    methods: &[],
    instance: &[],
    slots: &[HANDLE_SLOT],
    constants: &[],
};

/// ADR 0278 § 2's `Ldap\Tls`. The values are declaration ordinals.
pub(crate) const TLS: CoreEnum = CoreEnum {
    name: TLS_NAME,
    cases: &[("Required", 0), ("None", 1)],
    doc: Some(&TLS_DOC),
};

/// [`TLS`]' reference card — `rule:core-api/reference-card`.
const TLS_DOC: EnumDoc = EnumDoc {
    short: "Whether a connection to a directory is encrypted.",
    cases: &[
        CaseDoc {
            name: "Required",
            desc: "The connection is encrypted. An `ldaps://` URL starts with TLS, and an \
                   `ldap://` URL switches to TLS before it logs in.",
        },
        CaseDoc {
            name: "None",
            desc: "The connection is not encrypted. The password and every result cross the \
                   network as plain text. The host must be in `[capabilities.ldap] cleartext`.",
        },
    ],
};

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
