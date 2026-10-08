//! What `Core\Ldap` declares: its classes, its enums, and a reference card for
//! each of them and for every member.
//!
//! Rows, not behaviour, as in `crate::db`'s registry: every symbol named here is
//! defined by a sibling, and a card sits directly after the row it describes.

use crate::registry::{
    CaseDoc, ClassDoc, Const, CoreClass, CoreEnum, CoreField, CoreMethod, CoreOption, CoreTy,
    EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual, ShapeKeyDoc,
};

/// `Core\Ldap`'s fully-qualified name.
pub(crate) const NAME: &str = r"Core\Ldap";

/// `Core\Ldap\Connection`'s fully-qualified name.
pub(crate) const CONNECTION_NAME: &str = r"Core\Ldap\Connection";

/// [`TLS`]' fully-qualified name.
pub(crate) const TLS_NAME: &str = r"Core\Ldap\Tls";

/// `Core\Ldap\Entries`' fully-qualified name.
pub(crate) const ENTRIES_NAME: &str = r"Core\Ldap\Entries";

/// `Core\Ldap\Entry`'s fully-qualified name.
pub(crate) const ENTRY_NAME: &str = r"Core\Ldap\Entry";

/// `Core\Ldap\Filter`'s fully-qualified name.
pub(crate) const FILTER_NAME: &str = r"Core\Ldap\Filter";

/// [`SCOPE`]'s fully-qualified name.
pub(crate) const SCOPE_NAME: &str = r"Core\Ldap\Scope";

/// The slot a [`CONNECTION`] and an [`ENTRIES`] keep the connection's key in,
/// filed by [`nvs_runtime::Ctx::hold_open_connection`].
const HANDLE_SLOT: &str = "handle";

/// [`CONNECTION`]'s [`HANDLE_SLOT`].
pub(super) const CONNECTION_HANDLE_AT: usize = 0;
/// [`ENTRIES`]' [`HANDLE_SLOT`].
pub(super) const ENTRIES_HANDLE_AT: usize = 0;
/// [`ENTRIES`]' slot for the id its search is parked under.
pub(super) const ENTRIES_SEARCH_AT: usize = 1;
/// [`ENTRIES`]' slot for the entry the last `advance()` read.
pub(super) const ENTRIES_ENTRY_AT: usize = 2;
/// [`ENTRY`]'s DN slot.
pub(super) const ENTRY_DN_AT: usize = 0;
/// [`ENTRY`]'s slot for its attributes, keyed by name.
pub(super) const ENTRY_ATTRIBUTES_AT: usize = 1;
/// [`FILTER`]'s slot for its BER encoding.
pub(super) const FILTER_BER_AT: usize = 0;

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

/// `search`'s options, ADR 0278 § 1's `SearchOptions` as far as Stage 4 goes.
///
/// `base` and `select` are sinks: the base is a path the server walks, and
/// an attribute name is sent as it is written.
const SEARCH_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "base",
        ty: CoreTy::Text(Qual::Sink),
        // The block's own `base`, which `search` reads off the connection.
        default: Const::Null,
    },
    CoreOption {
        name: "scope",
        ty: CoreTy::Enum(SCOPE_NAME),
        default: Const::EnumCase(SCOPE_NAME, "Subtree"),
    },
    CoreOption {
        name: "select",
        ty: CoreTy::Array(&CoreTy::Str),
        // Every attribute the connection's account can read.
        default: Const::Null,
    },
    CoreOption {
        name: "pageSize",
        ty: CoreTy::Int,
        // [`super::PAGE_SIZE`].
        default: Const::Int(1000),
    },
    CoreOption {
        name: "sizeLimit",
        ty: CoreTy::Int,
        // The server's own limit.
        default: Const::Int(0),
    },
];

/// `read`'s one option.
const READ_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "select",
    ty: CoreTy::Array(&CoreTy::Str),
    default: Const::Null,
}];

/// ADR 0278 § 1's `Ldap\Connection`: what `connect` and `open` return.
///
/// Its one slot is the key the connection is held under in the request's own
/// table, which [`super::held`] reads back.
pub(crate) const CONNECTION: CoreClass = CoreClass {
    name: CONNECTION_NAME,
    doc: Some(&CONNECTION_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "whoami",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_ldap_connection_whoami",
            doc: Some(&WHOAMI_DOC),
        },
        CoreMethod {
            name: "search",
            names: &["filter"],
            params: &[
                CoreTy::Instance(FILTER_NAME),
                CoreTy::Options(SEARCH_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(ENTRIES_NAME),
            symbol: "nvs_core_ldap_connection_search",
            doc: Some(&SEARCH_DOC),
        },
        CoreMethod {
            name: "read",
            names: &["dn"],
            // A sink: the server parses a DN into a path in the tree.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Options(READ_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(ENTRY_NAME)),
            symbol: "nvs_core_ldap_connection_read",
            doc: Some(&READ_DOC),
        },
    ],
    slots: &[HANDLE_SLOT],
    constants: &[],
};

/// `Ldap\Connection::whoami`'s reference card — `rule:core-api/reference-card`.
const WHOAMI_DOC: MethodDoc = MethodDoc {
    short: "Returns the account the connection is logged in as, as the server reports it.",
    params: &[],
    ret: "`dn:` and then the account's DN, or `u:` and then its login name. The result is an \
          empty string when the connection did not log in.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The connection is closed.",
        },
        ErrorDoc {
            error: "Core\\Ldap\\LdapError",
            desc: "The server did not answer, or it returned an error. `$kind` says why.",
        },
    ],
};

/// `Ldap\Connection::search`'s reference card — `rule:core-api/reference-card`.
const SEARCH_DOC: MethodDoc = MethodDoc {
    short: "Finds the entries that match a filter. The server sends the entries in pages, and \
            the next page is read when a `foreach` loop reaches it.",
    params: &[
        ParamDoc {
            name: "filter",
            desc: "Which entries to return, such as `Filter::equals('sAMAccountName', $login)`.",
            shape: &[],
        },
        ParamDoc {
            name: "base",
            desc: "The DN the search starts from. Left out, it is the `base` of the `[ldap]` \
                   block. It cannot be `tainted`.",
            shape: &[],
        },
        ParamDoc {
            name: "scope",
            desc: "How far below `base` the search looks. The default is `Scope::Subtree`.",
            shape: &[],
        },
        ParamDoc {
            name: "select",
            desc: "The attributes each entry has, such as `['cn', 'mail']`. Left out, an entry \
                   has every attribute the connection's account can read.",
            shape: &[],
        },
        ParamDoc {
            name: "pageSize",
            desc: "How many entries the server sends at once. The default is 1000.",
            shape: &[],
        },
        ParamDoc {
            name: "sizeLimit",
            desc: "The most entries the search may find. The default, 0, is the server's own \
                   limit.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Ldap\\Entries`. Use it in a `foreach` loop to get each `Core\\Ldap\\Entry`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "A name in `select` is not an attribute name, `pageSize` or `sizeLimit` is out \
                   of range, the search has no `base`, or the connection is closed.",
        },
        ErrorDoc {
            error: "Core\\Ldap\\LdapError",
            desc: "The server returned an error, such as `NoSuchObject` for a `base` that does \
                   not exist. A search that finds more entries than `sizeLimit` throws \
                   `SizeLimitExceeded` in the loop.",
        },
    ],
};

/// `Ldap\Connection::read`'s reference card — `rule:core-api/reference-card`.
const READ_DOC: MethodDoc = MethodDoc {
    short: "Reads the one entry at a DN.",
    params: &[
        ParamDoc {
            name: "dn",
            desc: "The entry's DN, such as `CN=Staff,OU=Groups,DC=example,DC=test`. It cannot be \
                   `tainted`.",
            shape: &[],
        },
        ParamDoc {
            name: "select",
            desc: "The attributes the entry has. Left out, it has every attribute the \
                   connection's account can read.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Ldap\\Entry`, or `null` when no entry has this DN.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "A name in `select` is not an attribute name, or the connection is closed.",
        },
        ErrorDoc {
            error: "Core\\Ldap\\LdapError",
            desc: "The server returned an error other than \"no such entry\".",
        },
    ],
};

/// `Ldap\Entries`' class card — `rule:core-api/reference-card`.
const ENTRIES_CARD: ClassDoc = ClassDoc {
    short: "The entries a search found. A `foreach` loop gets them one at a time, and reads the \
            next page from the server when it needs it.",
};

/// ADR 0278 § 5's `Ldap\Entries`: what `search` returns, and its own iterator.
///
/// `ITERABLES` in [`crate::registry`] declares its element, and
/// [`crate::instance`]'s dispatch roster gives it the three names a `foreach`
/// calls. `super::search`'s module doc says what each slot is.
pub(crate) const ENTRIES: CoreClass = CoreClass {
    name: ENTRIES_NAME,
    doc: Some(&ENTRIES_CARD),
    methods: &[],
    instance: &[],
    slots: &[HANDLE_SLOT, "search", "entry"],
    constants: &[],
};

/// `Ldap\Entry`'s class card — `rule:core-api/reference-card`.
const ENTRY_CARD: ClassDoc = ClassDoc {
    short: "One entry from a directory: its DN and its attributes. An attribute name is found \
            without case, so `mail` and `Mail` are the same attribute.",
};

/// The card shared by [`ENTRY`]'s readers' one parameter.
const ATTRIBUTE_NAME_PARAM: ParamDoc = ParamDoc {
    name: "name",
    desc: "The attribute's name, such as `mail`. Upper and lower case are the same.",
    shape: &[],
};

/// ADR 0278 § 1's `Ldap\Entry`, as far as Stage 4 goes: the DN and the
/// readers for text and bytes. `super::search`'s module doc says what each
/// slot is.
pub(crate) const ENTRY: CoreClass = CoreClass {
    name: ENTRY_NAME,
    doc: Some(&ENTRY_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "dn",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_ldap_entry_dn",
            doc: Some(&ENTRY_DN_DOC),
        },
        CoreMethod {
            name: "has",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_ldap_entry_has",
            doc: Some(&ENTRY_HAS_DOC),
        },
        CoreMethod {
            name: "string",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_ldap_entry_string",
            doc: Some(&ENTRY_STRING_DOC),
        },
        CoreMethod {
            name: "strings",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Array(&CoreTy::TaintedStr)),
            symbol: "nvs_core_ldap_entry_strings",
            doc: Some(&ENTRY_STRINGS_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedBytes),
            symbol: "nvs_core_ldap_entry_bytes",
            doc: Some(&ENTRY_BYTES_DOC),
        },
    ],
    slots: &["dn", "attributes"],
    constants: &[],
};

/// `Ldap\Entry::dn`'s reference card — `rule:core-api/reference-card`.
const ENTRY_DN_DOC: MethodDoc = MethodDoc {
    short: "Returns the entry's DN, as the server sent it.",
    params: &[],
    ret: "The DN, such as `CN=Administrator,CN=Users,DC=example,DC=test`.",
    errors: &[],
};

/// `Ldap\Entry::has`'s reference card — `rule:core-api/reference-card`.
const ENTRY_HAS_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the entry has a value for an attribute.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "`true` when the entry has the attribute, and `false` when it does not or the search \
          did not select it.",
    errors: &[],
};

/// `Ldap\Entry::string`'s reference card — `rule:core-api/reference-card`.
const ENTRY_STRING_DOC: MethodDoc = MethodDoc {
    short: "Returns the one value of an attribute as text. The result is `tainted`, because the \
            directory's data can come from anyone who can write to it.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "The value, or `null` when the entry has no value for the attribute.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The attribute has more than one value, so use `strings`. Or the value is not \
               UTF-8 text, so use `bytes`.",
    }],
};

/// `Ldap\Entry::strings`' reference card — `rule:core-api/reference-card`.
const ENTRY_STRINGS_DOC: MethodDoc = MethodDoc {
    short: "Returns every value of an attribute as text, such as every `member` of a group. The \
            values are `tainted`.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "The values in the order the server sent them, or `null` when the entry has no value \
          for the attribute.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "A value is not UTF-8 text.",
    }],
};

/// `Ldap\Entry::bytes`' reference card — `rule:core-api/reference-card`.
const ENTRY_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns the one value of an attribute as bytes, exactly as the server sent it. Use \
            it for binary values, such as `objectGUID`. The result is `tainted`.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "The value, or `null` when the entry has no value for the attribute.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The attribute has more than one value.",
    }],
};

/// `Ldap\Filter`'s class card — `rule:core-api/reference-card`.
const FILTER_CARD: ClassDoc = ClassDoc {
    short: "Which entries a search returns. A filter is built from an attribute name and a \
            value. The value is sent as data and never as filter text, so a value from a user \
            cannot change what the filter matches.",
};

/// ADR 0278 § 6's `Ldap\Filter`, as far as Stage 4 goes: `equals` and
/// `present`. `super::search`'s module doc says what its slot is.
pub(crate) const FILTER: CoreClass = CoreClass {
    name: FILTER_NAME,
    doc: Some(&FILTER_CARD),
    methods: &[
        CoreMethod {
            name: "equals",
            names: &["attribute", "value"],
            // The name is a sink, checked against RFC 4512. The value is
            // data in the BER the filter is built into, so it may be
            // `tainted`.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_filter_equals",
            doc: Some(&FILTER_EQUALS_DOC),
        },
        CoreMethod {
            name: "present",
            names: &["attribute"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_filter_present",
            doc: Some(&FILTER_PRESENT_DOC),
        },
    ],
    instance: &[],
    slots: &["ber"],
    constants: &[],
};

/// The card shared by [`FILTER`]'s `attribute` parameters.
const FILTER_ATTRIBUTE_PARAM: ParamDoc = ParamDoc {
    name: "attribute",
    desc: "The attribute's name, such as `sAMAccountName`. It cannot be `tainted`.",
    shape: &[],
};

/// `Ldap\Filter::equals`' reference card — `rule:core-api/reference-card`.
const FILTER_EQUALS_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries where an attribute has this value.",
    params: &[
        FILTER_ATTRIBUTE_PARAM,
        ParamDoc {
            name: "value",
            desc: "The value to match. It may be `tainted`, such as a login name from a form. \
                   A `*` or `)` in it is a normal character.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$attribute` is not an attribute name.",
    }],
};

/// `Ldap\Filter::present`'s reference card — `rule:core-api/reference-card`.
const FILTER_PRESENT_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries that have any value for an attribute.",
    params: &[FILTER_ATTRIBUTE_PARAM],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$attribute` is not an attribute name.",
    }],
};

/// ADR 0278 § 5's `Ldap\Scope`. The values are declaration ordinals.
pub(crate) const SCOPE: CoreEnum = CoreEnum {
    name: SCOPE_NAME,
    cases: &[("Base", 0), ("OneLevel", 1), ("Subtree", 2)],
    doc: Some(&SCOPE_DOC),
};

/// [`SCOPE`]'s reference card — `rule:core-api/reference-card`.
const SCOPE_DOC: EnumDoc = EnumDoc {
    short: "How far below its `base` a search looks.",
    cases: &[
        CaseDoc {
            name: "Base",
            desc: "Only the `base` entry itself.",
        },
        CaseDoc {
            name: "OneLevel",
            desc: "The entries directly below `base`, and not `base` itself.",
        },
        CaseDoc {
            name: "Subtree",
            desc: "`base` and every entry below it, at any depth. This is the default.",
        },
    ],
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
