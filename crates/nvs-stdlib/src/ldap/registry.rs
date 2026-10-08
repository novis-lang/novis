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

/// `Core\Ldap\Dn`'s fully-qualified name.
pub(crate) const DN_NAME: &str = r"Core\Ldap\Dn";

/// `Core\Ldap\Ad`'s fully-qualified name.
pub(crate) const AD_NAME: &str = r"Core\Ldap\Ad";

/// `Core\Ldap\Sid`'s fully-qualified name.
pub(crate) const SID_NAME: &str = r"Core\Ldap\Sid";

/// `Core\Ldap\Ad\AccountFlags`' fully-qualified name.
pub(crate) const ACCOUNT_FLAGS_NAME: &str = r"Core\Ldap\Ad\AccountFlags";

/// `Core\Ldap\Ad\GroupType`'s fully-qualified name.
pub(crate) const GROUP_TYPE_NAME: &str = r"Core\Ldap\Ad\GroupType";

/// [`ACCOUNT_TYPE`]'s fully-qualified name.
pub(crate) const ACCOUNT_TYPE_NAME: &str = r"Core\Ldap\Ad\AccountType";

/// `Core\Ldap\Change`'s fully-qualified name.
pub(crate) const CHANGE_NAME: &str = r"Core\Ldap\Change";

/// [`SCOPE`]'s fully-qualified name.
pub(crate) const SCOPE_NAME: &str = r"Core\Ldap\Scope";

/// Every DN parameter's type, `rule:core-classes/ldap-dn-is-the-launderer`'s
/// `Dn|string`: a [`DN`], or text that refuses `tainted` because the server
/// parses it into a path in the tree.
const DN_OR_STRING: &[CoreTy] = &[CoreTy::Instance(DN_NAME), CoreTy::Text(Qual::Sink)];

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
/// [`ENTRIES`]' slot for the continuation references, filled when the search ends.
pub(super) const ENTRIES_REFERENCES_AT: usize = 3;
/// [`ENTRY`]'s DN slot.
pub(super) const ENTRY_DN_AT: usize = 0;
/// [`ENTRY`]'s slot for its attributes, keyed by name.
pub(super) const ENTRY_ATTRIBUTES_AT: usize = 1;
/// [`CHANGE`]'s slot for what it does: 0 adds, 1 removes, 2 replaces.
pub(super) const CHANGE_KIND_AT: usize = 0;
/// [`CHANGE`]'s slot for the attribute it changes.
pub(super) const CHANGE_ATTRIBUTE_AT: usize = 1;
/// [`CHANGE`]'s slot for the value it was given, unencoded, or `null`.
pub(super) const CHANGE_VALUE_AT: usize = 2;
/// [`FILTER`]'s slot for its BER encoding.
pub(super) const FILTER_BER_AT: usize = 0;
/// [`DN`]'s slot for its RFC 4514 text, as [`nvs_ldap::Dn::to_text`] wrote it.
pub(super) const DN_TEXT_AT: usize = 0;
/// [`SID`]'s slot for its binary form, as [`nvs_ldap::Sid::to_bytes`] wrote it.
pub(super) const SID_BYTES_AT: usize = 0;
/// [`ACCOUNT_FLAGS`]' and [`GROUP_TYPE`]'s slot for the integer AD wrote.
pub(super) const FLAGS_BITS_AT: usize = 0;
/// [`ACCOUNT_FLAGS`]' slot for `msDS-User-Account-Control-Computed`, or `null`.
pub(super) const FLAGS_COMPUTED_AT: usize = 1;
/// [`ACCOUNT_FLAGS`]' slot for whether `pwdLastSet` is `0`, as a `bool`.
pub(super) const FLAGS_MUST_CHANGE_AT: usize = 2;

/// ADR 0278 § 2's `Ldap\Settings`, the one shape `open` takes.
///
/// The qualifiers sit on the fields: `url` is a sink with no launderer, as
/// `rule:core-classes/db-capabilities` makes `Db\Settings.host` one, `user`
/// accepts `tainted` as a length-framed protocol field, and `password` is
/// `secret tainted string`. The ABI flattens the shape to one slot per field
/// in this order, and [`URL_ARG`] and the five consts after it are those slots.
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
    CoreField {
        name: "base",
        // A DN, as `search`'s `base` option is: it is where every search
        // that names no base starts.
        ty: CoreTy::Union(DN_OR_STRING),
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
/// [`SETTINGS`]' `base` slot.
pub(super) const BASE_ARG: usize = 5;

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
            ShapeKeyDoc {
                key: "base",
                ty: "Dn|string",
                desc: "The DN a search starts from when it does not give its own `base`. Text \
                       cannot be `tainted`.",
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
        ty: CoreTy::Union(DN_OR_STRING),
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
            params: &[CoreTy::Union(DN_OR_STRING), CoreTy::Options(READ_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(ENTRY_NAME)),
            symbol: "nvs_core_ldap_connection_read",
            doc: Some(&READ_DOC),
        },
        CoreMethod {
            name: "add",
            names: &["dn", "attributes"],
            params: &[CoreTy::Union(DN_OR_STRING), CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_ldap_connection_add",
            doc: Some(&ADD_DOC),
        },
        CoreMethod {
            name: "modify",
            names: &["dn", "changes"],
            params: &[
                CoreTy::Union(DN_OR_STRING),
                CoreTy::Array(&CoreTy::Instance(CHANGE_NAME)),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_ldap_connection_modify",
            doc: Some(&MODIFY_DOC),
        },
        CoreMethod {
            name: "delete",
            names: &["dn"],
            params: &[CoreTy::Union(DN_OR_STRING)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_ldap_connection_delete",
            doc: Some(&DELETE_DOC),
        },
        CoreMethod {
            name: "rename",
            names: &["from", "to"],
            params: &[CoreTy::Union(DN_OR_STRING), CoreTy::Union(DN_OR_STRING)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_ldap_connection_rename",
            doc: Some(&RENAME_DOC),
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
            desc: "The DN the search starts from, as a `Dn` or as text. Left out, it is the \
                   `base` of the `[ldap]` block. Text cannot be `tainted`.",
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
            desc: "The entry's DN, as a `Dn` or as text such as \
                   `CN=Staff,OU=Groups,DC=example,DC=test`. Text cannot be `tainted`. Build a DN \
                   from user input with `Dn::of` or `child`.",
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

/// The DN parameter of a write — `rule:core-api/reference-card`.
const WRITE_DN_PARAM: ParamDoc = ParamDoc {
    name: "dn",
    desc: "The entry's DN, as a `Dn` or as text. Text cannot be `tainted`. Build a DN from user \
           input with `Dn::of` or `child`.",
    shape: &[],
};

/// The `LdapError` every write throws — `rule:core-api/reference-card`.
const WRITE_ERRORS: &[ErrorDoc] = &[
    ErrorDoc {
        error: "LogicError",
        desc: "An attribute name is not a name, a value has no LDAP form, such as a `float`, \
               or the connection is closed.",
    },
    ErrorDoc {
        error: "Core\\Ldap\\LdapError",
        desc: "The server returned an error. `$kind` says why, such as `NoSuchObject`, \
               `AlreadyExists`, `InsufficientAccess` or `ConstraintViolation`.",
    },
];

/// `Ldap\Connection::add`'s reference card — `rule:core-api/reference-card`.
const ADD_DOC: MethodDoc = MethodDoc {
    short: "Adds a new entry to the directory.",
    params: &[
        WRITE_DN_PARAM,
        ParamDoc {
            name: "attributes",
            desc: "The entry's attributes, keyed by name, such as `['objectClass' => ['top', \
                   'group'], 'cn' => 'Staff']`. A value is written in the form the readers of \
                   `Entry` return it, and a list writes one value per element.",
            shape: &[],
        },
    ],
    ret: "Nothing.",
    errors: WRITE_ERRORS,
};

/// `Ldap\Connection::modify`'s reference card — `rule:core-api/reference-card`.
const MODIFY_DOC: MethodDoc = MethodDoc {
    short: "Changes the attributes of one entry. The server makes every change in the list, in \
            order, or none of them.",
    params: &[
        WRITE_DN_PARAM,
        ParamDoc {
            name: "changes",
            desc: "The changes, each made with `Change::add`, `remove`, `removeAll` or \
                   `replace`. The list cannot be empty.",
            shape: &[],
        },
    ],
    ret: "Nothing.",
    errors: WRITE_ERRORS,
};

/// `Ldap\Connection::delete`'s reference card — `rule:core-api/reference-card`.
const DELETE_DOC: MethodDoc = MethodDoc {
    short: "Deletes one entry. An entry with entries below it cannot be deleted.",
    params: &[WRITE_DN_PARAM],
    ret: "Nothing.",
    errors: WRITE_ERRORS,
};

/// `Ldap\Connection::rename`'s reference card — `rule:core-api/reference-card`.
const RENAME_DOC: MethodDoc = MethodDoc {
    short: "Gives an entry a new DN. The entry moves when the new DN has another parent.",
    params: &[
        ParamDoc {
            name: "from",
            desc: "The entry's DN now, as a `Dn` or as text. Text cannot be `tainted`.",
            shape: &[],
        },
        ParamDoc {
            name: "to",
            desc: "The entry's new DN, such as `CN=Shop,OU=Groups,DC=example,DC=test`. Text \
                   cannot be `tainted`.",
            shape: &[],
        },
    ],
    ret: "Nothing.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`from` or `to` is text that is not a DN, or the connection is closed.",
        },
        ErrorDoc {
            error: "Core\\Ldap\\LdapError",
            desc: "The server returned an error, such as `NoSuchObject` or `AlreadyExists`.",
        },
    ],
};

/// `Ldap\Change`'s class card — `rule:core-api/reference-card`.
const CHANGE_CARD: ClassDoc = ClassDoc {
    short: "One change to one attribute, for `Connection::modify`.",
};

/// The attribute parameter of every `Change` constructor.
const CHANGE_ATTRIBUTE_PARAM: ParamDoc = ParamDoc {
    name: "attribute",
    desc: "The attribute's name, such as `description`. It cannot be `tainted`.",
    shape: &[],
};

/// The value parameter of every `Change` constructor that takes one.
const CHANGE_VALUE_PARAM: ParamDoc = ParamDoc {
    name: "value",
    desc: "One value, or a list of values. A value is written in the form the readers of \
           `Entry` return it, such as an `int`, a `Uuid` or an `Instant`.",
    shape: &[],
};

/// The error every `Change` constructor throws.
const CHANGE_ERRORS: &[ErrorDoc] = &[ErrorDoc {
    error: "LogicError",
    desc: "`attribute` is not an attribute name.",
}];

/// ADR 0278 § 1's `Ldap\Change`: one change of a `modify`, its value kept
/// unencoded until `modify` sends it. `super::write`'s module doc says why.
pub(crate) const CHANGE: CoreClass = CoreClass {
    name: CHANGE_NAME,
    doc: Some(&CHANGE_CARD),
    methods: &[
        CoreMethod {
            name: "add",
            names: &["attribute", "value"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Instance(CHANGE_NAME),
            symbol: "nvs_core_ldap_change_add",
            doc: Some(&CHANGE_ADD_DOC),
        },
        CoreMethod {
            name: "remove",
            names: &["attribute", "value"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Instance(CHANGE_NAME),
            symbol: "nvs_core_ldap_change_remove",
            doc: Some(&CHANGE_REMOVE_DOC),
        },
        CoreMethod {
            name: "removeAll",
            names: &["attribute"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Instance(CHANGE_NAME),
            symbol: "nvs_core_ldap_change_remove_all",
            doc: Some(&CHANGE_REMOVE_ALL_DOC),
        },
        CoreMethod {
            name: "replace",
            names: &["attribute", "value"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Instance(CHANGE_NAME),
            symbol: "nvs_core_ldap_change_replace",
            doc: Some(&CHANGE_REPLACE_DOC),
        },
    ],
    instance: &[],
    slots: &["kind", "attribute", "value"],
    constants: &[],
};

/// `Ldap\Change::add`'s reference card — `rule:core-api/reference-card`.
const CHANGE_ADD_DOC: MethodDoc = MethodDoc {
    short: "Adds values to an attribute. The entry must not have them yet.",
    params: &[CHANGE_ATTRIBUTE_PARAM, CHANGE_VALUE_PARAM],
    ret: "The change.",
    errors: CHANGE_ERRORS,
};

/// `Ldap\Change::remove`'s reference card — `rule:core-api/reference-card`.
const CHANGE_REMOVE_DOC: MethodDoc = MethodDoc {
    short: "Removes values from an attribute. The entry must have them.",
    params: &[CHANGE_ATTRIBUTE_PARAM, CHANGE_VALUE_PARAM],
    ret: "The change.",
    errors: CHANGE_ERRORS,
};

/// `Ldap\Change::removeAll`'s reference card — `rule:core-api/reference-card`.
const CHANGE_REMOVE_ALL_DOC: MethodDoc = MethodDoc {
    short: "Removes an attribute and every value it has.",
    params: &[CHANGE_ATTRIBUTE_PARAM],
    ret: "The change.",
    errors: CHANGE_ERRORS,
};

/// `Ldap\Change::replace`'s reference card — `rule:core-api/reference-card`.
const CHANGE_REPLACE_DOC: MethodDoc = MethodDoc {
    short: "Replaces every value of an attribute. With `null` or an empty list, the attribute \
            is removed.",
    params: &[CHANGE_ATTRIBUTE_PARAM, CHANGE_VALUE_PARAM],
    ret: "The change.",
    errors: CHANGE_ERRORS,
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
    instance: &[CoreMethod {
        name: "references",
        names: &[],
        params: &[],
        defaults: &[],
        return_ty: CoreTy::Array(&CoreTy::TaintedStr),
        symbol: "nvs_core_ldap_entries_references",
        doc: Some(&ENTRIES_REFERENCES_DOC),
    }],
    slots: &[HANDLE_SLOT, "search", "entry", "references"],
    constants: &[],
};

/// `Ldap\Entries::references`' reference card — `rule:core-api/reference-card`.
const ENTRIES_REFERENCES_DOC: MethodDoc = MethodDoc {
    short: "Returns the URLs of other servers that the server named for this search. Novis does \
            not connect to them. The URLs are `tainted`, because the server sent them.",
    params: &[],
    ret: "The URLs in the order the server sent them. The list is complete after the loop \
          ends, and it is empty when the server named no other server.",
    errors: &[],
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

/// ADR 0278 § 1's `Ldap\Entry`: the DN, the readers for text and bytes, and
/// § 8's typed readers. `super::search`'s module doc says what each slot is,
/// and `super::value`'s what each typed reader accepts.
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
            return_ty: CoreTy::Instance(DN_NAME),
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
        CoreMethod {
            name: "int",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Int),
            symbol: "nvs_core_ldap_entry_int",
            doc: Some(&ENTRY_INT_DOC),
        },
        CoreMethod {
            name: "bool",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Bool),
            symbol: "nvs_core_ldap_entry_bool",
            doc: Some(&ENTRY_BOOL_DOC),
        },
        CoreMethod {
            name: "uuid",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(crate::uuid::NAME)),
            symbol: "nvs_core_ldap_entry_uuid",
            doc: Some(&ENTRY_UUID_DOC),
        },
        CoreMethod {
            name: "sid",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(SID_NAME)),
            symbol: "nvs_core_ldap_entry_sid",
            doc: Some(&ENTRY_SID_DOC),
        },
        CoreMethod {
            name: "sids",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Array(&CoreTy::Instance(SID_NAME))),
            symbol: "nvs_core_ldap_entry_sids",
            doc: Some(&ENTRY_SIDS_DOC),
        },
        CoreMethod {
            name: "instant",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(crate::time::INSTANT_NAME)),
            symbol: "nvs_core_ldap_entry_instant",
            doc: Some(&ENTRY_INSTANT_DOC),
        },
        CoreMethod {
            name: "duration",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(crate::time::DURATION_NAME)),
            symbol: "nvs_core_ldap_entry_duration",
            doc: Some(&ENTRY_DURATION_DOC),
        },
        CoreMethod {
            name: "accountFlags",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(ACCOUNT_FLAGS_NAME)),
            symbol: "nvs_core_ldap_entry_account_flags",
            doc: Some(&ENTRY_ACCOUNT_FLAGS_DOC),
        },
        CoreMethod {
            name: "groupType",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(GROUP_TYPE_NAME)),
            symbol: "nvs_core_ldap_entry_group_type",
            doc: Some(&ENTRY_GROUP_TYPE_DOC),
        },
        CoreMethod {
            name: "accountType",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Enum(ACCOUNT_TYPE_NAME)),
            symbol: "nvs_core_ldap_entry_account_type",
            doc: Some(&ENTRY_ACCOUNT_TYPE_DOC),
        },
        CoreMethod {
            name: "toArray",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Array(&CoreTy::TaintedBytes)),
            symbol: "nvs_core_ldap_entry_to_array",
            doc: Some(&ENTRY_TO_ARRAY_DOC),
        },
    ],
    slots: &["dn", "attributes"],
    constants: &[],
};

/// `Ldap\Entry::dn`'s reference card — `rule:core-api/reference-card`.
const ENTRY_DN_DOC: MethodDoc = MethodDoc {
    short: "Returns the entry's DN as a `Core\\Ldap\\Dn`.",
    params: &[],
    ret: "The DN, such as `CN=Administrator,CN=Users,DC=example,DC=test`.",
    errors: &[ErrorDoc {
        error: "Core\\Ldap\\LdapError",
        desc: "The server sent a DN that is not correct DN text. `$kind` is \
               `ErrorKind::Protocol`.",
    }],
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

/// The error every one of [`ENTRY`]'s single-value typed readers throws.
const ENTRY_TYPED_ERRORS: &[ErrorDoc] = &[ErrorDoc {
    error: "LogicError",
    desc: "The attribute has more than one value, or its value is not in the form this \
           function reads. The message names the attribute.",
}];

/// `Ldap\Entry::int`'s reference card — `rule:core-api/reference-card`.
const ENTRY_INT_DOC: MethodDoc = MethodDoc {
    short: "Returns a number attribute, such as `logonCount` or `primaryGroupID`, as an `int`.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "The number, or `null` when the entry has no value for the attribute.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The attribute has more than one value, or its value is not a whole number, or \
               the number is too large for an `int`. The message names the attribute.",
    }],
};

/// `Ldap\Entry::bool`'s reference card — `rule:core-api/reference-card`.
const ENTRY_BOOL_DOC: MethodDoc = MethodDoc {
    short: "Returns a yes-or-no attribute, such as `isCriticalSystemObject`, as a `bool`. The \
            directory writes these values as `TRUE` and `FALSE`.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "`true` for `TRUE`, `false` for `FALSE`, or `null` when the entry has no value for the \
          attribute.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The attribute has more than one value, or its value is not `TRUE` or `FALSE`. \
               The message names the attribute.",
    }],
};

/// `Ldap\Entry::uuid`'s reference card — `rule:core-api/reference-card`.
const ENTRY_UUID_DOC: MethodDoc = MethodDoc {
    short: "Returns a GUID attribute, such as `objectGUID`, as a `Core\\Uuid`. Active Directory \
            stores the first three groups of a GUID in reverse byte order. This function puts \
            them in the usual order, so the text is the same as Windows shows.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "The UUID, or `null` when the entry has no value for the attribute.",
    errors: ENTRY_TYPED_ERRORS,
};

/// `Ldap\Entry::sid`'s reference card — `rule:core-api/reference-card`.
const ENTRY_SID_DOC: MethodDoc = MethodDoc {
    short: "Returns a SID attribute, such as `objectSid`, as a `Core\\Ldap\\Sid`.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "The SID, or `null` when the entry has no value for the attribute.",
    errors: ENTRY_TYPED_ERRORS,
};

/// `Ldap\Entry::sids`' reference card — `rule:core-api/reference-card`.
const ENTRY_SIDS_DOC: MethodDoc = MethodDoc {
    short: "Returns every value of a SID attribute, such as `tokenGroups` or `sIDHistory`, as a \
            list of `Core\\Ldap\\Sid`.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "The SIDs in the order the server sent them, or `null` when the entry has no value for \
          the attribute.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "A value is not a SID. The message names the attribute.",
    }],
};

/// `Ldap\Entry::instant`'s reference card — `rule:core-api/reference-card`.
const ENTRY_INSTANT_DOC: MethodDoc = MethodDoc {
    short: "Returns a time attribute as a `Core\\Time\\Instant`. It reads both forms a \
            directory uses: a number of 100-nanosecond steps since the year 1601, such as \
            `pwdLastSet`, and text such as `20240101120000.0Z`, such as `whenCreated`.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "The instant, or `null` when the entry has no value for the attribute. It is also \
          `null` when the number is `0` or `9223372036854775807`. Active Directory uses those \
          two numbers for \"never\".",
    errors: ENTRY_TYPED_ERRORS,
};

/// `Ldap\Entry::duration`'s reference card — `rule:core-api/reference-card`.
const ENTRY_DURATION_DOC: MethodDoc = MethodDoc {
    short: "Returns a length of time, such as `maxPwdAge` or `lockoutDuration`, as a \
            `Core\\Time\\Duration`. Active Directory stores it as a negative number of \
            100-nanosecond steps.",
    params: &[ATTRIBUTE_NAME_PARAM],
    ret: "The duration, or `null` when the entry has no value for the attribute. It is also \
          `null` when the number is `-9223372036854775808`. Active Directory uses that number \
          for \"never\".",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The attribute has more than one value, or its value is not zero or a negative \
               number, or it is longer than a `Duration` can be. The message names the \
               attribute.",
    }],
};

/// The error [`ENTRY`]'s three AD readers throw, which read a fixed attribute.
const ENTRY_AD_ERRORS: &[ErrorDoc] = &[ErrorDoc {
    error: "LogicError",
    desc: "The attribute has more than one value, or its value is not in the form this \
           function reads. The message names the attribute.",
}];

/// `Ldap\Entry::accountFlags`' reference card — `rule:core-api/reference-card`.
const ENTRY_ACCOUNT_FLAGS_DOC: MethodDoc = MethodDoc {
    short: "Returns the account's `userAccountControl` as a `Core\\Ldap\\Ad\\AccountFlags`. \
            Select `userAccountControl` in the search. The search then also returns \
            `msDS-User-Account-Control-Computed`, which says if the account is locked out or \
            its password has expired.",
    params: &[],
    ret: "The flags, or `null` when the entry has no `userAccountControl`, such as a group.",
    errors: ENTRY_AD_ERRORS,
};

/// `Ldap\Entry::groupType`'s reference card — `rule:core-api/reference-card`.
const ENTRY_GROUP_TYPE_DOC: MethodDoc = MethodDoc {
    short: "Returns the group's `groupType` as a `Core\\Ldap\\Ad\\GroupType`.",
    params: &[],
    ret: "The group type, or `null` when the entry has no `groupType`, such as a user.",
    errors: ENTRY_AD_ERRORS,
};

/// `Ldap\Entry::accountType`'s reference card — `rule:core-api/reference-card`.
const ENTRY_ACCOUNT_TYPE_DOC: MethodDoc = MethodDoc {
    short: "Returns the entry's `sAMAccountType` as a `Core\\Ldap\\Ad\\AccountType`, such as \
            `AccountType::User`.",
    params: &[],
    ret: "The account type, or `null` when the entry has no `sAMAccountType`.",
    errors: ENTRY_AD_ERRORS,
};

/// `Ldap\Entry::toArray`'s reference card — `rule:core-api/reference-card`.
const ENTRY_TO_ARRAY_DOC: MethodDoc = MethodDoc {
    short: "Returns every attribute of the entry as an array. Each key is an attribute name, \
            written the way the server wrote it. Each value is a list of `tainted bytes`.",
    params: &[],
    ret: "The attributes, in the order the server sent them. The DN is not in the array, so use \
          `dn` for it.",
    errors: &[],
};

/// `Ldap\Filter`'s class card — `rule:core-api/reference-card`.
const FILTER_CARD: ClassDoc = ClassDoc {
    short: "Which entries a search returns. A filter is built from an attribute name and a \
            value. The value is sent as data and never as filter text, so a value from a user \
            cannot change what the filter matches.",
};

/// The parameters of every [`FILTER`] row that compares an attribute with a
/// value. The name is a sink, checked against RFC 4512. The value is data in
/// the BER the filter is built into, so it may be `tainted`.
const FILTER_COMPARE: &[CoreTy] = &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Neutral)];

/// A [`FILTER`] row named `name` that compares an attribute with a value.
const fn filter_compare(
    name: &'static str,
    symbol: &'static str,
    doc: &'static MethodDoc,
) -> CoreMethod {
    CoreMethod {
        name,
        names: &["attribute", "value"],
        params: FILTER_COMPARE,
        defaults: &[],
        return_ty: CoreTy::Instance(FILTER_NAME),
        symbol,
        doc: Some(doc),
    }
}

/// ADR 0278 § 6's `Ldap\Filter`. `super::search`'s module doc says what its
/// slot is.
pub(crate) const FILTER: CoreClass = CoreClass {
    name: FILTER_NAME,
    doc: Some(&FILTER_CARD),
    methods: &[
        filter_compare("equals", "nvs_core_ldap_filter_equals", &FILTER_EQUALS_DOC),
        filter_compare(
            "startsWith",
            "nvs_core_ldap_filter_starts_with",
            &FILTER_STARTS_WITH_DOC,
        ),
        filter_compare(
            "endsWith",
            "nvs_core_ldap_filter_ends_with",
            &FILTER_ENDS_WITH_DOC,
        ),
        filter_compare(
            "contains",
            "nvs_core_ldap_filter_contains",
            &FILTER_CONTAINS_DOC,
        ),
        CoreMethod {
            name: "present",
            names: &["attribute"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_filter_present",
            doc: Some(&FILTER_PRESENT_DOC),
        },
        filter_compare(
            "atLeast",
            "nvs_core_ldap_filter_at_least",
            &FILTER_AT_LEAST_DOC,
        ),
        filter_compare(
            "atMost",
            "nvs_core_ldap_filter_at_most",
            &FILTER_AT_MOST_DOC,
        ),
        filter_compare("approx", "nvs_core_ldap_filter_approx", &FILTER_APPROX_DOC),
        CoreMethod {
            name: "all",
            names: &["filters"],
            params: &[CoreTy::Variadic(&CoreTy::Instance(FILTER_NAME))],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_filter_all",
            doc: Some(&FILTER_ALL_DOC),
        },
        CoreMethod {
            name: "any",
            names: &["filters"],
            params: &[CoreTy::Variadic(&CoreTy::Instance(FILTER_NAME))],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_filter_any",
            doc: Some(&FILTER_ANY_DOC),
        },
        CoreMethod {
            name: "not",
            names: &["filter"],
            params: &[CoreTy::Instance(FILTER_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_filter_not",
            doc: Some(&FILTER_NOT_DOC),
        },
        CoreMethod {
            name: "parse",
            names: &["text"],
            // The server would read this text as a filter's structure, so it
            // is a sink and refuses `tainted`.
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_filter_parse",
            doc: Some(&FILTER_PARSE_DOC),
        },
    ],
    instance: &[CoreMethod {
        name: "toString",
        names: &[],
        params: &[],
        defaults: &[],
        // A value inside the filter may be `tainted`, so the text is.
        return_ty: CoreTy::TaintedStr,
        symbol: "nvs_core_ldap_filter_to_string",
        doc: Some(&FILTER_TO_STRING_DOC),
    }],
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

/// The card shared by the `value` parameter of [`FILTER`]'s substring rows.
const FILTER_PART_PARAM: ParamDoc = ParamDoc {
    name: "value",
    desc: "The text to look for. It may be `tainted`. A `*` in it is a normal character, and \
           not a wildcard.",
    shape: &[],
};

/// The errors of [`FILTER`]'s substring rows.
const FILTER_PART_ERRORS: &[ErrorDoc] = &[ErrorDoc {
    error: "LogicError",
    desc: "`$attribute` is not an attribute name, or `$value` is empty.",
}];

/// `Ldap\Filter::startsWith`' reference card — `rule:core-api/reference-card`.
const FILTER_STARTS_WITH_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries where a value of an attribute starts with this text.",
    params: &[FILTER_ATTRIBUTE_PARAM, FILTER_PART_PARAM],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_PART_ERRORS,
};

/// `Ldap\Filter::endsWith`' reference card — `rule:core-api/reference-card`.
const FILTER_ENDS_WITH_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries where a value of an attribute ends with this text.",
    params: &[FILTER_ATTRIBUTE_PARAM, FILTER_PART_PARAM],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_PART_ERRORS,
};

/// `Ldap\Filter::contains`' reference card — `rule:core-api/reference-card`.
const FILTER_CONTAINS_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries where a value of an attribute contains this text.",
    params: &[FILTER_ATTRIBUTE_PARAM, FILTER_PART_PARAM],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_PART_ERRORS,
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

/// The card shared by the `value` parameter of [`FILTER`]'s ordering rows.
const FILTER_BOUND_PARAM: ParamDoc = ParamDoc {
    name: "value",
    desc: "The value to compare with. It may be `tainted`. The server compares by the \
           attribute's own order, so `'10'` comes after `'9'` for a number.",
    shape: &[],
};

/// The error of a [`FILTER`] row whose only check is the attribute name.
const FILTER_NAME_ERRORS: &[ErrorDoc] = &[ErrorDoc {
    error: "LogicError",
    desc: "`$attribute` is not an attribute name.",
}];

/// `Ldap\Filter::atLeast`'s reference card — `rule:core-api/reference-card`.
const FILTER_AT_LEAST_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries where a value of an attribute is equal to or greater than this \
            value. LDAP has no \"greater than\" filter. Use `not` and `atMost` for it.",
    params: &[FILTER_ATTRIBUTE_PARAM, FILTER_BOUND_PARAM],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_NAME_ERRORS,
};

/// `Ldap\Filter::atMost`'s reference card — `rule:core-api/reference-card`.
const FILTER_AT_MOST_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries where a value of an attribute is equal to or less than this \
            value. LDAP has no \"less than\" filter. Use `not` and `atLeast` for it.",
    params: &[FILTER_ATTRIBUTE_PARAM, FILTER_BOUND_PARAM],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_NAME_ERRORS,
};

/// `Ldap\Filter::approx`' reference card — `rule:core-api/reference-card`.
const FILTER_APPROX_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries where a value of an attribute is close to this value, such as a \
            name that sounds the same. The server decides what is close.",
    params: &[
        FILTER_ATTRIBUTE_PARAM,
        ParamDoc {
            name: "value",
            desc: "The value to compare with. It may be `tainted`.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_NAME_ERRORS,
};

/// The error of [`FILTER`]'s `all` and `any`.
const FILTER_LIST_ERRORS: &[ErrorDoc] = &[ErrorDoc {
    error: "LogicError",
    desc: "No filter is given.",
}];

/// `Ldap\Filter::all`'s reference card — `rule:core-api/reference-card`.
const FILTER_ALL_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries that every one of the filters matches.",
    params: &[ParamDoc {
        name: "filters",
        desc: "One filter or more.",
        shape: &[],
    }],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_LIST_ERRORS,
};

/// `Ldap\Filter::any`'s reference card — `rule:core-api/reference-card`.
const FILTER_ANY_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries that at least one of the filters matches.",
    params: &[ParamDoc {
        name: "filters",
        desc: "One filter or more.",
        shape: &[],
    }],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_LIST_ERRORS,
};

/// `Ldap\Filter::not`'s reference card — `rule:core-api/reference-card`.
const FILTER_NOT_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries that the filter does not match.",
    params: &[ParamDoc {
        name: "filter",
        desc: "The filter to reverse.",
        shape: &[],
    }],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: &[],
};

/// `Ldap\Filter::parse`'s reference card — `rule:core-api/reference-card`.
const FILTER_PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads LDAP filter text, such as `(&(objectClass=user)(cn=Ann))`, and returns the \
            filter it describes. Use it for a filter that you wrote, for example in a \
            configuration file.",
    params: &[ParamDoc {
        name: "text",
        desc: "The filter text, as RFC 4515 writes it. It cannot be `tainted`. Build a filter \
               from user input with `equals` and the other functions of this class.",
        shape: &[],
    }],
    ret: "A `Core\\Ldap\\Filter`. Its `toString` returns text that `parse` reads back as the \
          same filter.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The text is not a filter, or it has a name that is not an attribute name. The \
               message gives the position of the first wrong character.",
    }],
};

/// `Ldap\Filter->toString`'s reference card — `rule:core-api/reference-card`.
const FILTER_TO_STRING_DOC: MethodDoc = MethodDoc {
    short: "Returns the filter as LDAP filter text, such as `(&(objectClass=user)(cn=Ann))`. \
            Use it to write the filter to a log. A search does not use this text.",
    params: &[],
    ret: "The text. A `*`, `(`, `)` or `\\` in a value is written as `\\` and two hex digits. \
          The text is `tainted`, because a value in the filter may be.",
    errors: &[],
};

/// `Ldap\Dn`'s class card — `rule:core-api/reference-card`.
const DN_CARD: ClassDoc = ClassDoc {
    short: "A DN (distinguished name), the path of an entry in a directory, such as \
            `CN=Ann Lee,OU=Staff,DC=example,DC=test`. Build one from an attribute and a value, \
            and the value can be user input: each special character in it is escaped.",
};

/// The `attribute` parameter of [`DN`]'s builders.
const DN_ATTRIBUTE_PARAM: ParamDoc = ParamDoc {
    name: "attribute",
    desc: "The attribute's name, such as `CN` or `OU`. It cannot be `tainted`.",
    shape: &[],
};

/// The `value` parameter of [`DN`]'s builders.
const DN_VALUE_PARAM: ParamDoc = ParamDoc {
    name: "value",
    desc: "The value, such as `Ann Lee`. It can be `tainted`. A `,`, `+`, `=` or other special \
           character in it is escaped, so it stays one value.",
    shape: &[],
};

/// The error of [`DN`]'s builders.
const DN_PART_ERRORS: &[ErrorDoc] = &[ErrorDoc {
    error: "LogicError",
    desc: "The attribute is not an attribute name, or the value is empty.",
}];

/// ADR 0278 § 7's `Ldap\Dn`, `rule:core-classes/ldap-dn-is-the-launderer`.
/// `super::dn`'s module doc says what its slot is.
///
/// `of` and `child` launder their `value` for the DN sink and nothing else:
/// they return a `Dn`, which only a DN parameter takes, and every text a `Dn`
/// gives back is `tainted`.
pub(crate) const DN: CoreClass = CoreClass {
    name: DN_NAME,
    doc: Some(&DN_CARD),
    methods: &[
        CoreMethod {
            name: "parse",
            names: &["text"],
            // The server would read this text as a path, so it is a sink.
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Instance(DN_NAME),
            symbol: "nvs_core_ldap_dn_parse",
            doc: Some(&DN_PARSE_DOC),
        },
        CoreMethod {
            name: "of",
            names: &["attribute", "value"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Instance(DN_NAME),
            symbol: "nvs_core_ldap_dn_of",
            doc: Some(&DN_OF_DOC),
        },
    ],
    instance: &[
        CoreMethod {
            name: "child",
            names: &["attribute", "value"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Instance(DN_NAME),
            symbol: "nvs_core_ldap_dn_child",
            doc: Some(&DN_CHILD_DOC),
        },
        CoreMethod {
            name: "parent",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(DN_NAME)),
            symbol: "nvs_core_ldap_dn_parent",
            doc: Some(&DN_PARENT_DOC),
        },
        CoreMethod {
            name: "rdnAttribute",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_ldap_dn_rdn_attribute",
            doc: Some(&DN_RDN_ATTRIBUTE_DOC),
        },
        CoreMethod {
            name: "rdnValue",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_ldap_dn_rdn_value",
            doc: Some(&DN_RDN_VALUE_DOC),
        },
        CoreMethod {
            name: "isWithin",
            names: &["other"],
            params: &[CoreTy::Instance(DN_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_ldap_dn_is_within",
            doc: Some(&DN_IS_WITHIN_DOC),
        },
        CoreMethod {
            name: "toString",
            names: &[],
            params: &[],
            defaults: &[],
            // A value in the DN may be `tainted`, so the text is.
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_ldap_dn_to_string",
            doc: Some(&DN_TO_STRING_DOC),
        },
    ],
    slots: &["text"],
    constants: &[],
};

/// `Ldap\Dn::parse`'s reference card — `rule:core-api/reference-card`.
const DN_PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads DN text, such as `OU=Staff,DC=example,DC=test`, and returns the DN. Use it \
            for a DN that you wrote, for example in a configuration file.",
    params: &[ParamDoc {
        name: "text",
        desc: "The DN text, as RFC 4514 writes it. It cannot be `tainted`. Build a DN from user \
               input with `of` and `child`.",
        shape: &[],
    }],
    ret: "A `Core\\Ldap\\Dn`. Spaces around `,`, `+` and `=` are removed.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The text is not a DN. The message gives the position of the first wrong \
               character.",
    }],
};

/// `Ldap\Dn::of`'s reference card — `rule:core-api/reference-card`.
const DN_OF_DOC: MethodDoc = MethodDoc {
    short: "Returns a DN with one part, `attribute=value`.",
    params: &[DN_ATTRIBUTE_PARAM, DN_VALUE_PARAM],
    ret: "A `Core\\Ldap\\Dn`.",
    errors: DN_PART_ERRORS,
};

/// `Ldap\Dn->child`'s reference card — `rule:core-api/reference-card`.
const DN_CHILD_DOC: MethodDoc = MethodDoc {
    short: "Returns the DN of an entry directly below this one. `attribute=value` is added at \
            the start.",
    params: &[DN_ATTRIBUTE_PARAM, DN_VALUE_PARAM],
    ret: "A new `Core\\Ldap\\Dn`. This DN does not change.",
    errors: DN_PART_ERRORS,
};

/// `Ldap\Dn->parent`'s reference card — `rule:core-api/reference-card`.
const DN_PARENT_DOC: MethodDoc = MethodDoc {
    short: "Returns the DN one level up. The first part is removed.",
    params: &[],
    ret: "The parent DN, or `null` when this DN has only one part.",
    errors: &[],
};

/// `Ldap\Dn->rdnAttribute`'s reference card — `rule:core-api/reference-card`.
const DN_RDN_ATTRIBUTE_DOC: MethodDoc = MethodDoc {
    short: "Returns the attribute name of the first part. For \
            `CN=Ann Lee,OU=Staff,DC=example,DC=test` this is `CN`.",
    params: &[],
    ret: "The name, written as in the DN. When the first part has several values joined by \
          `+`, it is the name of the first one.",
    errors: &[],
};

/// `Ldap\Dn->rdnValue`'s reference card — `rule:core-api/reference-card`.
const DN_RDN_VALUE_DOC: MethodDoc = MethodDoc {
    short: "Returns the value of the first part, with escapes removed. For \
            `CN=Lee\\, Ann,OU=Staff,DC=example,DC=test` this is `Lee, Ann`.",
    params: &[],
    ret: "The value. It is `tainted`. When the first part has several values joined by `+`, it \
          is the first one.",
    errors: &[],
};

/// `Ldap\Dn->isWithin`'s reference card — `rule:core-api/reference-card`.
const DN_IS_WITHIN_DOC: MethodDoc = MethodDoc {
    short: "Checks if this DN is `other` or an entry below it. Names and values are compared \
            without case.",
    params: &[ParamDoc {
        name: "other",
        desc: "The DN to compare with, such as `Dn::parse('DC=example,DC=test')`.",
        shape: &[],
    }],
    ret: "`true` when this DN ends with every part of `other`.",
    errors: &[],
};

/// `Ldap\Dn->toString`'s reference card — `rule:core-api/reference-card`.
const DN_TO_STRING_DOC: MethodDoc = MethodDoc {
    short: "Returns the DN as text, with each special character in a value escaped.",
    params: &[],
    ret: "The text. `Dn::parse` reads it back as the same DN. The text is `tainted`, because a \
          value in the DN may be.",
    errors: &[],
};

/// `Ldap\Sid`'s class card — `rule:core-api/reference-card`.
const SID_CARD: ClassDoc = ClassDoc {
    short: "A SID (security identifier), the number Windows uses for a user, a group or a \
            computer, such as `S-1-5-21-1004336348-1177238915-682003330-512`.",
};

/// ADR 0278 § 1's `Ldap\Sid`: one value, its binary form in one slot. The
/// bodies are `super::sid`'s.
pub(crate) const SID: CoreClass = CoreClass {
    name: SID_NAME,
    doc: Some(&SID_CARD),
    methods: &[CoreMethod {
        name: "parse",
        names: &["text"],
        // The text is read into numbers, and nothing of it is sent as text.
        params: &[CoreTy::Text(Qual::Neutral)],
        defaults: &[],
        return_ty: CoreTy::Instance(SID_NAME),
        symbol: "nvs_core_ldap_sid_parse",
        doc: Some(&SID_PARSE_DOC),
    }],
    instance: &[
        CoreMethod {
            name: "toString",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_ldap_sid_to_string",
            doc: Some(&SID_TO_STRING_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_ldap_sid_bytes",
            doc: Some(&SID_BYTES_DOC),
        },
        CoreMethod {
            name: "domain",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(SID_NAME)),
            symbol: "nvs_core_ldap_sid_domain",
            doc: Some(&SID_DOMAIN_DOC),
        },
        CoreMethod {
            name: "rid",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_ldap_sid_rid",
            doc: Some(&SID_RID_DOC),
        },
    ],
    slots: &["bytes"],
    constants: &[],
};

/// `Ldap\Sid::parse`'s reference card — `rule:core-api/reference-card`.
const SID_PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads SID text, such as `S-1-5-32-544`, and returns the SID.",
    params: &[ParamDoc {
        name: "text",
        desc: "The text. It starts with `S-1-`, then the authority, then 1 to 15 numbers, all \
               joined by `-`.",
        shape: &[],
    }],
    ret: "A `Core\\Ldap\\Sid`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The text is not a SID. The message says which part is wrong.",
    }],
};

/// `Ldap\Sid->toString`'s reference card — `rule:core-api/reference-card`.
const SID_TO_STRING_DOC: MethodDoc = MethodDoc {
    short: "Returns the SID as text, such as `S-1-5-32-544`.",
    params: &[],
    ret: "The text. `Sid::parse` reads it back as the same SID.",
    errors: &[],
};

/// `Ldap\Sid->bytes`'s reference card — `rule:core-api/reference-card`.
const SID_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns the SID in its binary form, the form Active Directory stores in `objectSid`.",
    params: &[],
    ret: "The bytes: 8 bytes, then 4 bytes for each number after the authority.",
    errors: &[],
};

/// `Ldap\Sid->domain`'s reference card — `rule:core-api/reference-card`.
const SID_DOMAIN_DOC: MethodDoc = MethodDoc {
    short: "Returns the SID without its last number. For a user or a group, this is the SID of \
            its domain.",
    params: &[],
    ret: "The shorter SID, or `null` when this SID has only one number after the authority.",
    errors: &[],
};

/// `Ldap\Sid->rid`'s reference card — `rule:core-api/reference-card`.
const SID_RID_DOC: MethodDoc = MethodDoc {
    short: "Returns the last number of the SID, the RID (relative identifier). In a domain, \
            `500` is the built-in `Administrator` and `512` is `Domain Admins`.",
    params: &[],
    ret: "The RID, from `0` to `4294967295`.",
    errors: &[],
};

/// `Ldap\Ad`'s class card — `rule:core-api/reference-card`.
const AD_CARD: ClassDoc = ClassDoc {
    short: "Filters that only Active Directory understands: group membership through nested \
            groups, enabled and disabled accounts, and a test of the bits in a flag attribute. \
            Each function returns a `Core\\Ldap\\Filter`, which you can combine with the \
            filters of that class.",
};

/// `memberOf`'s one option.
const AD_MEMBER_OF_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "nested",
    ty: CoreTy::Bool,
    default: Const::Bool(false),
}];

/// The parameters of [`AD`]'s `bitAnd` and `bitOr`. The name is a sink, as
/// every attribute name in a filter is.
const AD_BITS: &[CoreTy] = &[CoreTy::Text(Qual::Sink), CoreTy::Int];

/// ADR 0278 § 1's `Ldap\Ad`: AD's matching rules, as `Ldap\Filter` values.
/// The bodies are `super::search`'s, beside the other filter constructors.
pub(crate) const AD: CoreClass = CoreClass {
    name: AD_NAME,
    doc: Some(&AD_CARD),
    methods: &[
        CoreMethod {
            name: "memberOf",
            names: &["group"],
            params: &[
                CoreTy::Union(DN_OR_STRING),
                CoreTy::Options(AD_MEMBER_OF_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_ad_member_of",
            doc: Some(&AD_MEMBER_OF_DOC),
        },
        CoreMethod {
            name: "enabled",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_ad_enabled",
            doc: Some(&AD_ENABLED_DOC),
        },
        CoreMethod {
            name: "disabled",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_ad_disabled",
            doc: Some(&AD_DISABLED_DOC),
        },
        CoreMethod {
            name: "bitAnd",
            names: &["attribute", "bits"],
            params: AD_BITS,
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_ad_bit_and",
            doc: Some(&AD_BIT_AND_DOC),
        },
        CoreMethod {
            name: "bitOr",
            names: &["attribute", "bits"],
            params: AD_BITS,
            defaults: &[],
            return_ty: CoreTy::Instance(FILTER_NAME),
            symbol: "nvs_core_ldap_ad_bit_or",
            doc: Some(&AD_BIT_OR_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Ldap\Ad::memberOf`'s reference card — `rule:core-api/reference-card`.
const AD_MEMBER_OF_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries that are members of a group.",
    params: &[
        ParamDoc {
            name: "group",
            desc: "The group's DN. A string cannot be `tainted`. Build the DN with \
                   `Core\\Ldap\\Dn::of` when a part of it comes from a user.",
            shape: &[],
        },
        ParamDoc {
            name: "nested",
            desc: "With `true`, the filter also matches the members of a group that is itself \
                   a member of the group, at any depth. The default is `false`, which matches \
                   only the direct members.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: &[],
};

/// `Ldap\Ad::enabled`'s reference card — `rule:core-api/reference-card`.
const AD_ENABLED_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries whose account is not disabled. It tests the bit with the value \
            `2` in `userAccountControl`, and matches when that bit is not set.",
    params: &[],
    ret: "A `Core\\Ldap\\Filter`. An entry with no `userAccountControl`, such as a group, also \
          matches. Combine it with a filter on `objectClass` to find only users.",
    errors: &[],
};

/// `Ldap\Ad::disabled`'s reference card — `rule:core-api/reference-card`.
const AD_DISABLED_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries whose account is disabled. It tests the bit with the value `2` \
            in `userAccountControl`, and matches when that bit is set.",
    params: &[],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: &[],
};

/// The card shared by the `bits` parameter of [`AD`]'s `bitAnd` and `bitOr`.
const AD_BITS_PARAM: ParamDoc = ParamDoc {
    name: "bits",
    desc: "The bits to test, as one number, such as `2 | 512`.",
    shape: &[],
};

/// `Ldap\Ad::bitAnd`'s reference card — `rule:core-api/reference-card`.
const AD_BIT_AND_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries where a number attribute has every one of these bits set.",
    params: &[FILTER_ATTRIBUTE_PARAM, AD_BITS_PARAM],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_NAME_ERRORS,
};

/// `Ldap\Ad::bitOr`'s reference card — `rule:core-api/reference-card`.
const AD_BIT_OR_DOC: MethodDoc = MethodDoc {
    short: "Matches the entries where a number attribute has at least one of these bits set.",
    params: &[FILTER_ATTRIBUTE_PARAM, AD_BITS_PARAM],
    ret: "A `Core\\Ldap\\Filter`.",
    errors: FILTER_NAME_ERRORS,
};

/// A flag object's `bool` reader named `name`. The bit it reads is found by
/// that name in `nvs_ldap::value`'s table, so the body is `super::flags`'.
const fn flag_reader(
    name: &'static str,
    symbol: &'static str,
    doc: &'static MethodDoc,
) -> CoreMethod {
    CoreMethod {
        name,
        names: &[],
        params: &[],
        defaults: &[],
        return_ty: CoreTy::Bool,
        symbol,
        doc: Some(doc),
    }
}

/// A flag reader's card, whose `short` says what the bit means.
const fn flag_doc(short: &'static str) -> MethodDoc {
    MethodDoc {
        short,
        params: &[],
        ret: "`true` when the bit is set, and `false` when it is not.",
        errors: &[],
    }
}

/// `Ldap\Ad\AccountFlags`' class card — `rule:core-api/reference-card`.
const ACCOUNT_FLAGS_CARD: ClassDoc = ClassDoc {
    short: "The flags of a user or computer account, read from `userAccountControl`. Each flag \
            is a function that returns a `bool`. `bits` returns the whole number, with every \
            bit, also the bits no function names.",
};

/// ADR 0278 §§ 1 and 8's `Ldap\Ad\AccountFlags`: one reader per flag
/// `nvs_ldap::value::ACCOUNT_FLAGS` names, in its order. `super::flags`' module
/// doc says what its slots are.
pub(crate) const ACCOUNT_FLAGS: CoreClass = CoreClass {
    name: ACCOUNT_FLAGS_NAME,
    doc: Some(&ACCOUNT_FLAGS_CARD),
    methods: &[],
    instance: &[
        flag_reader(
            "script",
            "nvs_core_ldap_ad_account_flags_script",
            &AF_SCRIPT_DOC,
        ),
        flag_reader(
            "disabled",
            "nvs_core_ldap_ad_account_flags_disabled",
            &AF_DISABLED_DOC,
        ),
        flag_reader(
            "homeDirectoryRequired",
            "nvs_core_ldap_ad_account_flags_home_directory_required",
            &AF_HOME_DIRECTORY_REQUIRED_DOC,
        ),
        flag_reader(
            "lockedOut",
            "nvs_core_ldap_ad_account_flags_locked_out",
            &AF_LOCKED_OUT_DOC,
        ),
        flag_reader(
            "passwordNotRequired",
            "nvs_core_ldap_ad_account_flags_password_not_required",
            &AF_PASSWORD_NOT_REQUIRED_DOC,
        ),
        flag_reader(
            "reversibleEncryption",
            "nvs_core_ldap_ad_account_flags_reversible_encryption",
            &AF_REVERSIBLE_ENCRYPTION_DOC,
        ),
        flag_reader(
            "temporaryDuplicateAccount",
            "nvs_core_ldap_ad_account_flags_temporary_duplicate_account",
            &AF_TEMPORARY_DUPLICATE_ACCOUNT_DOC,
        ),
        flag_reader(
            "normalAccount",
            "nvs_core_ldap_ad_account_flags_normal_account",
            &AF_NORMAL_ACCOUNT_DOC,
        ),
        flag_reader(
            "interdomainTrustAccount",
            "nvs_core_ldap_ad_account_flags_interdomain_trust_account",
            &AF_INTERDOMAIN_TRUST_ACCOUNT_DOC,
        ),
        flag_reader(
            "workstationTrustAccount",
            "nvs_core_ldap_ad_account_flags_workstation_trust_account",
            &AF_WORKSTATION_TRUST_ACCOUNT_DOC,
        ),
        flag_reader(
            "serverTrustAccount",
            "nvs_core_ldap_ad_account_flags_server_trust_account",
            &AF_SERVER_TRUST_ACCOUNT_DOC,
        ),
        flag_reader(
            "passwordNeverExpires",
            "nvs_core_ldap_ad_account_flags_password_never_expires",
            &AF_PASSWORD_NEVER_EXPIRES_DOC,
        ),
        flag_reader(
            "mnsLogonAccount",
            "nvs_core_ldap_ad_account_flags_mns_logon_account",
            &AF_MNS_LOGON_ACCOUNT_DOC,
        ),
        flag_reader(
            "smartcardRequired",
            "nvs_core_ldap_ad_account_flags_smartcard_required",
            &AF_SMARTCARD_REQUIRED_DOC,
        ),
        flag_reader(
            "trustedForDelegation",
            "nvs_core_ldap_ad_account_flags_trusted_for_delegation",
            &AF_TRUSTED_FOR_DELEGATION_DOC,
        ),
        flag_reader(
            "notDelegated",
            "nvs_core_ldap_ad_account_flags_not_delegated",
            &AF_NOT_DELEGATED_DOC,
        ),
        flag_reader(
            "useDesKeyOnly",
            "nvs_core_ldap_ad_account_flags_use_des_key_only",
            &AF_USE_DES_KEY_ONLY_DOC,
        ),
        flag_reader(
            "noPreauthRequired",
            "nvs_core_ldap_ad_account_flags_no_preauth_required",
            &AF_NO_PREAUTH_REQUIRED_DOC,
        ),
        flag_reader(
            "passwordExpired",
            "nvs_core_ldap_ad_account_flags_password_expired",
            &AF_PASSWORD_EXPIRED_DOC,
        ),
        flag_reader(
            "trustedToAuthForDelegation",
            "nvs_core_ldap_ad_account_flags_trusted_to_auth_for_delegation",
            &AF_TRUSTED_TO_AUTH_FOR_DELEGATION_DOC,
        ),
        flag_reader(
            "partialSecretsAccount",
            "nvs_core_ldap_ad_account_flags_partial_secrets_account",
            &AF_PARTIAL_SECRETS_ACCOUNT_DOC,
        ),
        CoreMethod {
            name: "bits",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_ldap_ad_account_flags_bits",
            doc: Some(&AF_BITS_DOC),
        },
        CoreMethod {
            name: "mustChangePassword",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_ldap_ad_account_flags_must_change_password",
            doc: Some(&AF_MUST_CHANGE_PASSWORD_DOC),
        },
        CoreMethod {
            name: "with",
            names: &[],
            params: &[CoreTy::Options(AF_WITH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(ACCOUNT_FLAGS_NAME),
            symbol: "nvs_core_ldap_ad_account_flags_with",
            doc: Some(&AF_WITH_DOC),
        },
    ],
    slots: &["bits", "computed", "mustChange"],
    constants: &[],
};

const AF_SCRIPT_DOC: MethodDoc = flag_doc("Checks the bit `0x1`: a logon script runs.");
const AF_DISABLED_DOC: MethodDoc =
    flag_doc("Checks the bit `0x2`: the account is disabled, so nobody can log in with it.");
const AF_HOME_DIRECTORY_REQUIRED_DOC: MethodDoc =
    flag_doc("Checks the bit `0x8`: the account needs a home directory.");
const AF_LOCKED_OUT_DOC: MethodDoc = flag_doc(
    "Checks the bit `0x10`: the account is locked out after too many wrong passwords. Active \
     Directory computes this bit. It is read from `msDS-User-Account-Control-Computed` when the \
     entry has it.",
);
const AF_PASSWORD_NOT_REQUIRED_DOC: MethodDoc =
    flag_doc("Checks the bit `0x20`: the account may have an empty password.");
const AF_REVERSIBLE_ENCRYPTION_DOC: MethodDoc = flag_doc(
    "Checks the bit `0x80`: the password is stored with an encryption that can be reversed.",
);
const AF_TEMPORARY_DUPLICATE_ACCOUNT_DOC: MethodDoc = flag_doc(
    "Checks the bit `0x100`: the account is a local account for a user from another domain.",
);
const AF_NORMAL_ACCOUNT_DOC: MethodDoc =
    flag_doc("Checks the bit `0x200`: the account is an ordinary user account.");
const AF_INTERDOMAIN_TRUST_ACCOUNT_DOC: MethodDoc =
    flag_doc("Checks the bit `0x800`: the account is a trust with another domain.");
const AF_WORKSTATION_TRUST_ACCOUNT_DOC: MethodDoc =
    flag_doc("Checks the bit `0x1000`: the account is a computer that is a domain member.");
const AF_SERVER_TRUST_ACCOUNT_DOC: MethodDoc =
    flag_doc("Checks the bit `0x2000`: the account is a domain controller.");
const AF_PASSWORD_NEVER_EXPIRES_DOC: MethodDoc =
    flag_doc("Checks the bit `0x10000`: the password never expires.");
const AF_MNS_LOGON_ACCOUNT_DOC: MethodDoc =
    flag_doc("Checks the bit `0x20000`: the account is an MNS logon account.");
const AF_SMARTCARD_REQUIRED_DOC: MethodDoc =
    flag_doc("Checks the bit `0x40000`: the user must log in with a smart card.");
const AF_TRUSTED_FOR_DELEGATION_DOC: MethodDoc = flag_doc(
    "Checks the bit `0x80000`: a service on this account may act for any user it receives.",
);
const AF_NOT_DELEGATED_DOC: MethodDoc =
    flag_doc("Checks the bit `0x100000`: no service may act for this user, even a trusted one.");
const AF_USE_DES_KEY_ONLY_DOC: MethodDoc =
    flag_doc("Checks the bit `0x200000`: Kerberos uses only DES keys for this account.");
const AF_NO_PREAUTH_REQUIRED_DOC: MethodDoc = flag_doc(
    "Checks the bit `0x400000`: Kerberos gives a ticket for this account without first \
     checking the password.",
);
const AF_PASSWORD_EXPIRED_DOC: MethodDoc = flag_doc(
    "Checks the bit `0x800000`: the password has expired. Active Directory computes this bit. \
     It is read from `msDS-User-Account-Control-Computed` when the entry has it.",
);
const AF_TRUSTED_TO_AUTH_FOR_DELEGATION_DOC: MethodDoc = flag_doc(
    "Checks the bit `0x1000000`: a service on this account may act for a user without that \
     user's password.",
);
const AF_PARTIAL_SECRETS_ACCOUNT_DOC: MethodDoc =
    flag_doc("Checks the bit `0x4000000`: the account is a read-only domain controller.");

/// `Ldap\Ad\AccountFlags::bits`' reference card — `rule:core-api/reference-card`.
const AF_BITS_DOC: MethodDoc = MethodDoc {
    short: "Returns `userAccountControl` as the number Active Directory stored, such as `512`.",
    params: &[],
    ret: "The number, with every bit. It does not include the bits Active Directory computes.",
    errors: &[],
};

/// `Ldap\Ad\AccountFlags::mustChangePassword`' reference card — `rule:core-api/reference-card`.
const AF_MUST_CHANGE_PASSWORD_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the user must change the password at the next login. This is true \
            when `pwdLastSet` is `0`. A search that selects `userAccountControl` also reads \
            `pwdLastSet`.",
    params: &[],
    ret: "`true` when `pwdLastSet` is `0`, and `false` when it is not or the entry has no \
          `pwdLastSet`.",
    errors: &[],
};

/// One option of a flag field's `with`: a flag to set or clear, left as it
/// is when the call does not name it.
const fn flag_option(name: &'static str) -> CoreOption {
    CoreOption {
        name,
        ty: CoreTy::Bool,
        default: Const::Null,
    }
}

/// The card of one option of a flag field's `with`.
const fn flag_option_doc(name: &'static str) -> ParamDoc {
    ParamDoc {
        name,
        desc: "`true` sets the flag and `false` clears it. Leave it out to keep the flag as it is.",
        shape: &[],
    }
}

/// `Ldap\Ad\AccountFlags::with`'s options: every flag in
/// `nvs_ldap::value::ACCOUNT_FLAGS` but the two AD computes, in its order.
const AF_WITH_OPTIONS: &[CoreOption] = &[
    flag_option("script"),
    flag_option("disabled"),
    flag_option("homeDirectoryRequired"),
    flag_option("passwordNotRequired"),
    flag_option("reversibleEncryption"),
    flag_option("temporaryDuplicateAccount"),
    flag_option("normalAccount"),
    flag_option("interdomainTrustAccount"),
    flag_option("workstationTrustAccount"),
    flag_option("serverTrustAccount"),
    flag_option("passwordNeverExpires"),
    flag_option("mnsLogonAccount"),
    flag_option("smartcardRequired"),
    flag_option("trustedForDelegation"),
    flag_option("notDelegated"),
    flag_option("useDesKeyOnly"),
    flag_option("noPreauthRequired"),
    flag_option("trustedToAuthForDelegation"),
    flag_option("partialSecretsAccount"),
];

/// `Ldap\Ad\AccountFlags::with`'s reference card — `rule:core-api/reference-card`.
const AF_WITH_DOC: MethodDoc = MethodDoc {
    short: "Returns a copy with the flags you name set or cleared, such as \
            `$flags->with({disabled: true})`. Every other bit stays as it is. `lockedOut` and \
            `passwordExpired` are not options, because Active Directory computes them.",
    params: &[
        flag_option_doc("script"),
        flag_option_doc("disabled"),
        flag_option_doc("homeDirectoryRequired"),
        flag_option_doc("passwordNotRequired"),
        flag_option_doc("reversibleEncryption"),
        flag_option_doc("temporaryDuplicateAccount"),
        flag_option_doc("normalAccount"),
        flag_option_doc("interdomainTrustAccount"),
        flag_option_doc("workstationTrustAccount"),
        flag_option_doc("serverTrustAccount"),
        flag_option_doc("passwordNeverExpires"),
        flag_option_doc("mnsLogonAccount"),
        flag_option_doc("smartcardRequired"),
        flag_option_doc("trustedForDelegation"),
        flag_option_doc("notDelegated"),
        flag_option_doc("useDesKeyOnly"),
        flag_option_doc("noPreauthRequired"),
        flag_option_doc("trustedToAuthForDelegation"),
        flag_option_doc("partialSecretsAccount"),
    ],
    ret: "A new `Core\\Ldap\\Ad\\AccountFlags`. The object you call it on does not change.",
    errors: &[],
};

/// `Ldap\Ad\GroupType`'s class card — `rule:core-api/reference-card`.
const GROUP_TYPE_CARD: ClassDoc = ClassDoc {
    short: "The type of a group, read from `groupType`. Each flag is a function that returns a \
            `bool`. `bits` returns the whole number.",
};

/// ADR 0278 §§ 1 and 8's `Ldap\Ad\GroupType`: one reader per flag
/// `nvs_ldap::value::GROUP_TYPE_FLAGS` names, in its order.
pub(crate) const GROUP_TYPE: CoreClass = CoreClass {
    name: GROUP_TYPE_NAME,
    doc: Some(&GROUP_TYPE_CARD),
    methods: &[],
    instance: &[
        flag_reader(
            "system",
            "nvs_core_ldap_ad_group_type_system",
            &GT_SYSTEM_DOC,
        ),
        flag_reader(
            "global",
            "nvs_core_ldap_ad_group_type_global",
            &GT_GLOBAL_DOC,
        ),
        flag_reader(
            "domainLocal",
            "nvs_core_ldap_ad_group_type_domain_local",
            &GT_DOMAIN_LOCAL_DOC,
        ),
        flag_reader(
            "universal",
            "nvs_core_ldap_ad_group_type_universal",
            &GT_UNIVERSAL_DOC,
        ),
        flag_reader(
            "appBasic",
            "nvs_core_ldap_ad_group_type_app_basic",
            &GT_APP_BASIC_DOC,
        ),
        flag_reader(
            "appQuery",
            "nvs_core_ldap_ad_group_type_app_query",
            &GT_APP_QUERY_DOC,
        ),
        flag_reader(
            "security",
            "nvs_core_ldap_ad_group_type_security",
            &GT_SECURITY_DOC,
        ),
        CoreMethod {
            name: "bits",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_ldap_ad_group_type_bits",
            doc: Some(&GT_BITS_DOC),
        },
        CoreMethod {
            name: "with",
            names: &[],
            params: &[CoreTy::Options(GT_WITH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(GROUP_TYPE_NAME),
            symbol: "nvs_core_ldap_ad_group_type_with",
            doc: Some(&GT_WITH_DOC),
        },
    ],
    slots: &["bits"],
    constants: &[],
};

const GT_SYSTEM_DOC: MethodDoc = flag_doc("Checks the bit `0x1`: the system created the group.");
const GT_GLOBAL_DOC: MethodDoc =
    flag_doc("Checks the bit `0x2`: the group is global. Its members are from its own domain.");
const GT_DOMAIN_LOCAL_DOC: MethodDoc =
    flag_doc("Checks the bit `0x4`: the group is domain local. It is used only in its own domain.");
const GT_UNIVERSAL_DOC: MethodDoc =
    flag_doc("Checks the bit `0x8`: the group is universal. It can have members from any domain.");
const GT_APP_BASIC_DOC: MethodDoc =
    flag_doc("Checks the bit `0x10`: the group is an application group.");
const GT_APP_QUERY_DOC: MethodDoc = flag_doc(
    "Checks the bit `0x20`: the group is an application group whose members come from a query.",
);
const GT_SECURITY_DOC: MethodDoc = flag_doc(
    "Checks the bit `0x80000000`: the group is a security group, which can be given \
     permissions. Without it, the group is a distribution group, which is a mailing list.",
);

/// `Ldap\Ad\GroupType::bits`' reference card — `rule:core-api/reference-card`.
const GT_BITS_DOC: MethodDoc = MethodDoc {
    short: "Returns `groupType` as the number Active Directory stored. A security group's \
            number is negative, such as `-2147483646`.",
    params: &[],
    ret: "The number, with every bit.",
    errors: &[],
};

/// `Ldap\Ad\GroupType::with`'s options: every flag in
/// `nvs_ldap::value::GROUP_TYPE_FLAGS`, in its order.
const GT_WITH_OPTIONS: &[CoreOption] = &[
    flag_option("system"),
    flag_option("global"),
    flag_option("domainLocal"),
    flag_option("universal"),
    flag_option("appBasic"),
    flag_option("appQuery"),
    flag_option("security"),
];

/// `Ldap\Ad\GroupType::with`'s reference card — `rule:core-api/reference-card`.
const GT_WITH_DOC: MethodDoc = MethodDoc {
    short: "Returns a copy with the flags you name set or cleared, such as \
            `$type->with({security: false})`. Every other bit stays as it is.",
    params: &[
        flag_option_doc("system"),
        flag_option_doc("global"),
        flag_option_doc("domainLocal"),
        flag_option_doc("universal"),
        flag_option_doc("appBasic"),
        flag_option_doc("appQuery"),
        flag_option_doc("security"),
    ],
    ret: "A new `Core\\Ldap\\Ad\\GroupType`. The object you call it on does not change.",
    errors: &[],
};

/// ADR 0278 § 8's `Ldap\Ad\AccountType`, the registry half of
/// [`nvs_ldap::value::AccountType`]. The values are declaration ordinals, and
/// `super::value` joins the two halves by name.
pub(crate) const ACCOUNT_TYPE: CoreEnum = CoreEnum {
    name: ACCOUNT_TYPE_NAME,
    cases: &[
        ("Domain", 0),
        ("Group", 1),
        ("NonSecurityGroup", 2),
        ("Alias", 3),
        ("NonSecurityAlias", 4),
        ("User", 5),
        ("Machine", 6),
        ("Trust", 7),
        ("AppBasicGroup", 8),
        ("AppQueryGroup", 9),
    ],
    doc: Some(&ACCOUNT_TYPE_DOC),
};

/// [`ACCOUNT_TYPE`]'s reference card — `rule:core-api/reference-card`.
const ACCOUNT_TYPE_DOC: EnumDoc = EnumDoc {
    short: "What kind of object an entry is, read from `sAMAccountType`.",
    cases: &[
        CaseDoc {
            name: "Domain",
            desc: "The domain itself.",
        },
        CaseDoc {
            name: "Group",
            desc: "A global or universal security group.",
        },
        CaseDoc {
            name: "NonSecurityGroup",
            desc: "A global or universal distribution group.",
        },
        CaseDoc {
            name: "Alias",
            desc: "A domain local security group.",
        },
        CaseDoc {
            name: "NonSecurityAlias",
            desc: "A domain local distribution group.",
        },
        CaseDoc {
            name: "User",
            desc: "A user account.",
        },
        CaseDoc {
            name: "Machine",
            desc: "A computer account.",
        },
        CaseDoc {
            name: "Trust",
            desc: "A trust with another domain.",
        },
        CaseDoc {
            name: "AppBasicGroup",
            desc: "An application group.",
        },
        CaseDoc {
            name: "AppQueryGroup",
            desc: "An application group whose members come from a query.",
        },
    ],
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

    #[test]
    fn every_flag_reader_names_a_bit_of_its_field_in_order() {
        for (class, flags) in [
            (&ACCOUNT_FLAGS, nvs_ldap::value::ACCOUNT_FLAGS),
            (&GROUP_TYPE, nvs_ldap::value::GROUP_TYPE_FLAGS),
        ] {
            let readers: Vec<&str> = class
                .instance
                .iter()
                .map(|row| row.name)
                .filter(|name| !["bits", "with", "mustChangePassword"].contains(name))
                .collect();
            let named: Vec<&str> = flags.iter().map(|flag| flag.name).collect();
            assert_eq!(readers, named, "{}", class.name);
        }
    }

    #[test]
    fn every_with_option_names_a_flag_ad_does_not_compute_in_order() {
        let computed = nvs_ldap::value::COMPUTED_ACCOUNT_FLAGS;
        for (options, flags) in [
            (AF_WITH_OPTIONS, nvs_ldap::value::ACCOUNT_FLAGS),
            (GT_WITH_OPTIONS, nvs_ldap::value::GROUP_TYPE_FLAGS),
        ] {
            let written: Vec<&str> = options.iter().map(|option| option.name).collect();
            let settable: Vec<&str> = flags
                .iter()
                .filter(|flag| flags != nvs_ldap::value::ACCOUNT_FLAGS || flag.bit & computed == 0)
                .map(|flag| flag.name)
                .collect();
            assert_eq!(written, settable);
        }
    }

    #[test]
    fn every_account_type_names_a_registered_case() {
        let cases: Vec<&str> = ACCOUNT_TYPE.cases.iter().map(|(name, _)| *name).collect();
        let kinds: Vec<&str> = nvs_ldap::value::AccountType::ALL
            .iter()
            .map(|(kind, _)| kind.name())
            .collect();
        assert_eq!(cases, kinds);
    }

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
