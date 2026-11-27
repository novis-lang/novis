//! The warnings, `W1xxx`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `W1xxx` fails the
//! crate's tests.

codes! {
    range = "W1";

    // `W1001` is retired and is never reused: no check raised an
    // unreachable-code warning.
    // `W1002` is retired and is never reused: no rule asks for an
    // unused-variable warning, and no check raised it.
    /// A use of a declaration that carries `#[Core\Deprecated]`, or an
    /// override of a deprecated method.
    pub const W_DEPRECATED: Code = Code::new("W1003").card(
        "This code uses something that is deprecated. It still works, but a future version may \
         remove it. The message names it, then the version that deprecated it, then the note, \
         then the code to write instead.",
    );
    // `W1004` is retired and is never reused: no check raised a warning for
    // a construct whose result differs from PHP's.
    /// A secret file the configuration reads can be read by an account other
    /// than the one the runtime runs as. `rule:config/a-secret-is-a-file-whose-content-is-the-value` advises here where it
    /// refuses on writability: a Compose secret is mounted `0444` and a
    /// Kubernetes secret volume defaults to `0644`, so inside a container this
    /// mode is the norm and on a shared host it is not, and nothing the
    /// runtime can read tells it which it is in.
    pub const W_SECRET_FILE_READABLE: Code = Code::new("W1005").card(
        "Other user accounts on this host can read this secret file. Inside a container this \
         is normal. On a shared host, make the file readable only by the account that runs \
         Novis, for example with `chmod 600`.",
    );
    /// `catch (Throwable) => value` — an expression-level arm naming the root
    /// of the exception tree, binding nothing, whose body is not a `throw`
    /// (`rule:expressions/bare-throwable-arm-warns`). In the block form a `catch (Throwable)` has a body with room to
    /// log or re-raise; in the expression form the body *is* the value, so an
    /// unbound arm over the root is by construction "discard every failure,
    /// including the ones this site never anticipated" — the suppression
    /// operator Novis does not have, regrown as a one-liner. The two honest spellings are naming
    /// the class the site expects and binding `$e` to carry the value. A
    /// warning rather than an error because the hazard is a habit and not a
    /// type error, the reason
    /// `rule:expressions/nullable-condition-lint`
    /// gives for its own.
    pub const W_CATCH_ARM_DISCARDS_EVERY_FAILURE: Code = Code::new("W1006").card(
        "This `catch (Throwable)` arm turns every error into a value, also errors you did not \
         expect. Name the error class you expect, or write `catch (Throwable $e)` and use \
         `$e`.",
    );
    /// A credential the configuration puts in force begins or ends with
    /// whitespace. `rule:config/a-secret-is-a-file-whose-content-is-the-value` keeps such a value exactly as it was written —
    /// a password may legitimately carry an edge space, and removing it would
    /// be `rule:errors/ambiguous-input-refused`'s repair of input in place of a reading of it — so the value
    /// is used as-is and this is an advisory, never a refusal: refusing it
    /// would wall off a credential issued somewhere else, with no remedy in
    /// the file that names it.
    ///
    /// It is said out loud because that space is invisible in the one place an
    /// operator looks. In the file half of a `password`/`password_file` pair
    /// there is nothing to see at all, and at the end of a TOML line it is a space before a quote. A
    /// reader that quietly trims it instead turns a working credential into an
    /// authentication failure at the far end, which is the bug this code
    /// exists to make loud.
    pub const W_CREDENTIAL_HAS_EDGE_WHITESPACE: Code = Code::new("W1007").card(
        "This credential starts or ends with a space or a line break, and Novis uses it \
         exactly as written. If you did not mean that, remove the whitespace. Write a secret \
         file with `printf %s`, because `echo` adds a line break.",
    );
    /// The configuration names a shared store and grants nothing that may
    /// reach it: `[cache.shared] url` is set and `cache.shared` is not
    /// granted, so `Core\Cache::shared()` and `Core\RateLimit::consume` would
    /// both be refused at the door.
    /// `rule:config/cache-shared-is-the-grant-over-the-configured-store` is
    /// what makes that pair legible at boot — the two keys are in the same
    /// file, so the deployment that granted `net.connect` for its store's host
    /// hears about it while an operator is reading output rather than on the
    /// first request that touches the tier.
    ///
    /// An advisory and not a refusal, because the pair is not wrong on its
    /// own: a fleet may share one `nvs.toml` between an application that uses
    /// the tier and one that does not, and the second is a program with
    /// nothing to fix. What it may not be is *silent*, since the failure it
    /// otherwise produces arrives one deploy later and names only the member.
    pub const W_STORE_CONFIGURED_UNGRANTED: Code = Code::new("W1008").card(
        "`[cache.shared] url` names a store, but no application may use it, so \
         `Core\\Cache::shared()` will fail. Write `cache.shared = true` under `[capabilities]`.",
    );
    /// `[tls.client] keylog` is on, and this host is one where that is
    /// allowed: every outbound TLS session is appending its secrets to the
    /// named file.
    ///
    /// Announced at every start rather than once, because the state it
    /// describes is one somebody switched on for an afternoon's debugging and
    /// nobody reports having switched off — a capture left in place is the
    /// deployment's whole outbound traffic readable by anyone who can read a
    /// file. It is not a refusal here for the reason it is `E0640` elsewhere:
    /// `production` never reaches this, so the only host that sees it has
    /// already said it is being debugged.
    pub const W_TLS_KEYLOG_ON: Code = Code::new("W1009").card(
        "`[tls.client] keylog` is on, so the keys of every TLS connection are written to \
         a file. Anybody who can read that file can read the traffic. Remove the setting when \
         you have finished debugging.",
    );
    /// `[http.client.proxy] resolve` is `proxy`, so the destination's
    /// address is the proxy's question and no longer this deployment's.
    ///
    /// `rule:security/net-address-policy` is not being violated — the
    /// operator has said in configuration that the address question has
    /// moved — but it is a real narrowing: Novis judges the scheme, the
    /// `net.connect` host list and the tainted-URL check, and never learns
    /// an address to judge. Written at every start rather than once,
    /// because a deployment that has run this way for a year still owes
    /// today's log the sentence, and the alternative is an auditor reading
    /// a boot record that says the address policy is in force when what is
    /// in force is the proxy's.
    pub const W_PROXY_RESOLVES_THE_DESTINATION: Code = Code::new("W1010").card(
        "`[http.client.proxy] resolve` is `\"proxy\"`, so the proxy looks up the address of \
         every outbound host and Novis cannot check it. Write `resolve = \"local\"` if this \
         network can look up addresses itself.",
    );
    /// A `/** … */` block directly above a declaration, across no blank line
    /// (`rule:tooling/doc-comment-is-three-slashes`). The block is an ordinary
    /// comment and documents nothing — `nvs meta`, `nvs doc` and the editor's
    /// hover all read `///` alone — and that shape is the PHPDoc habit, so
    /// it is always a mistake here and never a note to the reader. A warning
    /// rather than a refusal because the program is correct; what is wrong is
    /// that its documentation is silently dropped, which is exactly what a
    /// reader who only sees `nvs check` say `no errors` cannot discover. The
    /// help names the one-token fix.
    pub const W_DOC_BLOCK_BEFORE_A_DECLARATION: Code = Code::new("W1011").card(
        "A `/** ... */` block is an ordinary comment, so it does not document the declaration \
         below it. Start each line of a doc comment with `///` instead.",
    );
    /// `{` directly followed by a class path — `{Page::TITLE}`,
    /// `{Money::format($c)}` — inside an html template
    /// (`rule:core-classes/html-template`). A brace hole begins with `$`, so
    /// this is text and is written to the page as written; the shape is a
    /// template habit, and a page that prints `{Page::TITLE}` is a program
    /// that runs and ships the wrong string. A warning rather than a refusal
    /// because the grammar is unchanged and the bytes may be what was meant;
    /// the help names `<?= Page::TITLE ?>`, the hole that takes the
    /// expression, and `\{` for a brace that should stay as text.
    pub const W_HTML_TEMPLATE_BRACE_BEFORE_A_CLASS_PATH: Code = Code::new("W1012").card(
        "In an html template, a `{` followed by a class name is printed as text. Write \
         `<?= Page::TITLE ?>` to print a value, or `\\{` for a brace you want to keep.",
    );
    /// A completion file that is not JSON, or has a field that is unknown or
    /// of the wrong form (`rule:ide/completion-files-offer-values-at-named-parameters`).
    /// The language server publishes it on the completion file, and the file
    /// contributes nothing until it is fixed, so no value is offered from a
    /// file that is half understood.
    pub const W_COMPLETION_FILE_INVALID: Code = Code::new("W1013").card(
        "This completion file is not valid, so none of its values are offered. The message \
         names the field that is wrong and the fields that are allowed there.",
    );
    /// A completion file's attachment names a set that no completion file in
    /// the workspace defines. The attachment still offers its own `values`.
    pub const W_COMPLETION_SET_MISSING: Code = Code::new("W1014").card(
        "No completion file defines this set. Define it under `sets` in a completion file, or \
         correct the name.",
    );
    /// A value's `location` names a file that does not exist. The value is
    /// still offered, and go-to-definition on it has no answer.
    pub const W_COMPLETION_LOCATION_MISSING: Code = Code::new("W1015").card(
        "The file this `location` names does not exist. The path is relative to the folder of \
         the completion file.",
    );
    /// A value that is not `deprecated` names a `replacement`, which is then
    /// ignored.
    pub const W_COMPLETION_REPLACEMENT_NOT_DEPRECATED: Code = Code::new("W1016").card(
        "A `replacement` is used only for a deprecated value. Add `\"deprecated\": true`, or \
         delete the `replacement`.",
    );
    /// An attachment names a method the class does not declare, or a parameter
    /// the method does not have, in `parameter` or in `when`, for a class the
    /// workspace index declares. No call can reach that attachment.
    pub const W_COMPLETION_MEMBER_MISSING: Code = Code::new("W1017").card(
        "The class does not declare this method, or the method has no parameter with this \
         name, so these values are never offered. Write the name as the class declares it.",
    );
    /// An attachment's parameter is declared with a type that is not a
    /// string, so no string argument can reach it.
    pub const W_COMPLETION_PARAMETER_NOT_A_STRING: Code = Code::new("W1018").card(
        "This parameter does not take a string, so its values are never offered. Attach them to \
         a parameter of type `string`.",
    );
    /// An attachment names a class the workspace index does not declare.
    /// Published as a hint, because the class may come from `vendor/`, which
    /// the index does not read.
    pub const W_COMPLETION_CLASS_UNDECLARED: Code = Code::new("W1019").card(
        "The workspace does not declare this class. If it comes from `vendor/`, this is \
         expected. Otherwise, check the class's whole name.",
    );
    /// A string literal at a parameter a completion file marks `strict`, whose
    /// text is none of the values that apply at the call. Reported by the
    /// language server only, in the program's file, and never by `nvs check`.
    pub const W_COMPLETION_VALUE_UNKNOWN: Code = Code::new("W1020").card(
        "A completion file lists the values this parameter takes, and this string is not one of \
         them. Choose a value from the list, or add this one to the completion file.",
    );
    /// A string literal equal to a value a completion file marks `deprecated`,
    /// among the values that apply at the call. Published by the language
    /// server only, as a hint with the `Deprecated` tag, and never by `nvs
    /// check`. When the value names a `replacement`, the hint carries the fix
    /// that writes it.
    pub const W_COMPLETION_VALUE_DEPRECATED: Code = Code::new("W1021").card(
        "A completion file says this value is deprecated. If the file names a replacement, the \
         quick fix changes the string to it.",
    );
    /// An operator mix whose grouping a reader misreads
    /// (`rule:expressions/misread-grouping-warns`): a `??` whose right side is
    /// an unparenthesized arithmetic or `.` expression, and a `?:` whose
    /// condition is one, or a `??`. The list is closed, because every case on
    /// it is valid code and a warning on valid code is only worth its noise
    /// where the misreading is common. A warning rather than a refusal because
    /// the grammar is unchanged and the parse may be what was meant; it carries
    /// two edits, parentheses around what the code does now, which a fix-all
    /// may apply, and the reading the author most likely meant, which only a
    /// person may choose. `nvs_syntax::parser::grouping` raises it.
    pub const W_MISREAD_GROUPING: Code = Code::new("W1022").card(
        "The operators here are applied in an order that is easy to misread. \
         `$n ?? 0 + 10` is `$n ?? (0 + 10)`, so the result is `5` when `$n` is `5`. \
         `\"n=\" . $ok ? \"x\" : \"y\"` tests `\"n=\" . $ok`, so the result is always `\"x\"`. \
         Add parentheses to show the order you mean.",
    );
    /// A condition whose type may be `null`
    /// (`rule:expressions/nullable-condition-lint`): `null` and a falsy value
    /// of the other half are both false, so `if ($s as ?int)` is false for a
    /// value that did not convert and for a valid `0`. A warning rather than a
    /// refusal because every type is a condition
    /// (`rule:expressions/truthy-positions`), and the hazard is a habit and
    /// not a type error. `nvs_types::expr::operators` raises it.
    pub const W_NULLABLE_CONDITION: Code = Code::new("W1023").card(
        "This condition can be `null`. `null` is false, and so are `0`, `\"\"` and `false`. \
         So `if ($s as ?int)` is false when `$s` is not a number, and also when it is `\"0\"`. \
         Write `!= null` to test whether there is a value: `if (($s as ?int) != null)`.",
    );
    /// A table header in one configuration file that the typed tree has no
    /// block for, with no key under it and no table under it that has a key
    /// (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`).
    /// The table is removed before the file is read, so it sets nothing. A
    /// warning rather than `E0601` because an empty header grants nothing and
    /// claims nothing: the unknown-key refusal exists so a typo cannot read as
    /// "granted nothing", and a header with nothing under it reads as nothing
    /// either way. A header with any key under it stays `E0601`.
    /// `nvs_config::file::parse_with_warnings` raises it.
    pub const W_EMPTY_UNKNOWN_BLOCK: Code = Code::new("W1024").card(
        "This block is not part of the configuration, and it contains no settings, so Novis \
         ignores it. Delete the line. A block that Novis does not know stops the start when it \
         contains a setting.",
    );
    /// A `Core\Router::dispatch` call whose gate cannot be a
    /// `Core\Router\Responder`, in a program with a route method that returns
    /// a value (`rule:routing/a-gate-answers-around-every-handler`). `dispatch`
    /// ignores that value, so the page the method built never reaches the
    /// response. A warning and never an error: a program may mean to ignore it,
    /// and it says so with a `respond` that does nothing or a `void` method.
    /// "Cannot be" is over every class the program declares, so the warning is
    /// never wrong about the program as written.
    /// `nvs_types::routes::check_responders` raises it.
    pub const W_DISPATCH_IGNORES_RETURNED_VALUE: Code = Code::new("W1025").card(
        "A route method returns a value, but the gate passed to `Core\\Router::dispatch` does \
         not implement `Core\\Router\\Responder`. So `dispatch` ignores the value, and it is not \
         sent in the response. Implement `Core\\Router\\Responder` in the gate class. Its \
         `respond` method receives the value and writes the response. If a route method writes \
         its own response, declare its return type `void`.",
    );
    /// A `use` line that no name in its namespace scope reads
    /// (`rule:statements/an-unused-use-is-a-warning`). A warning and never an
    /// error: the program runs as written, and the line is only noise. It
    /// carries one safe edit that deletes the line, so `nvs check --fix`
    /// removes it. `nvs_lsp::diagnostics::unused_uses` raises it, for `nvs
    /// check` and for the editor, which shows it faded.
    pub const W_UNUSED_USE: Code = Code::new("W1026").card(
        "This `use` line imports a name that the file never uses. The line has no effect, so \
         you can delete it. `nvs check --fix` deletes it for you.",
    );
    /// A route method that reads no request body itself, and reaches a
    /// `Core\Request::jsonAs` or `postAs` call through the methods it calls
    /// (`rule:routing/a-request-body-is-read-from-the-handlers-own-call`). The
    /// API document takes a body only from the handler's own call, so it has
    /// no `requestBody` for the route. A warning and never an error: the
    /// program runs as written, and only the document is incomplete.
    /// `nvs_types::routes::check_helper_reads` raises it.
    pub const W_BODY_READ_IN_HELPER: Code = Code::new("W1027").card(
        "This route method reads the request body in another method that it calls. The API \
         document only describes a body that the route method reads itself. So the document has \
         no `requestBody` for this route. Move the `Core\\Request::jsonAs` or \
         `Core\\Request::formFieldsAs` call into the route method.",
    );
    /// A key in `[settings]`, `[secrets]`, `[app.settings]` or `[app.secrets]`
    /// that no program the tree names reads with `Core\Config::setting` or
    /// `secret` (`rule:config/settings-and-secrets-are-a-programs-own-values`).
    /// A warning and never an error: a program the tree does not name may read
    /// it. `nvs config check` raises it, and never when a program computes a
    /// name.
    pub const W_SETTING_NOT_READ: Code = Code::new("W1028").card(
        "No program reads this setting or secret. The check looks at every program that the \
         configuration names in `[[app]]` and `[[server.mount]]`, and at each file given with \
         `--program`. Check the name for a typo. If another program reads it, give that program \
         with `--program`.",
    );
    /// A pattern literal that only the backtracking regex engine accepts, under
    /// `[regex] backtracking = "warn"` (`rule:core-classes/regex-compile-time-tiering`). The
    /// pattern compiles and runs under `[limits] max_regex_steps`.
    pub const W_REGEX_BACKTRACKING: Code = Code::new("W1029").card(
        "This pattern needs the backtracking regex engine. Lookaround and backreferences need \
         it. `[regex] backtracking` is `warn` in `nvs.toml`. The pattern compiles, and \
         `[limits] max_regex_steps` limits the work of each match. Write the pattern without \
         lookaround and without backreferences to remove the warning.",
    );
    /// A parameter with no `@param` line in a doc comment that already has an
    /// `@param` for another parameter
    /// (`rule:tooling/doc-comment-tags-are-closed-and-checked`). A comment with
    /// no `@param` at all is never warned. The fix inserts an empty
    /// `/// @param $name` line in signature order, and that line then shows
    /// `E0139` until its author writes the sentence.
    pub const W_DOC_PARAM_MISSING: Code = Code::new("W1030").card(
        "This doc comment describes some parameters with `@param`, but not this one. Add an \
         `@param` line for it, and write what it means.",
    );
    /// A `[[restrict]]` target in the application's `package.toml` that names no declaration
    /// of the program (`rule:packaging/a-restriction-limits-where-a-declaration-is-used`). The
    /// same target in a package's file is silent, since it often names code this program does
    /// not load.
    pub const W_RESTRICTION_NAMES_NOTHING: Code = Code::new("W1031").card(
        "The `use` of this `[[restrict]]` entry names no class, method, function or namespace \
         in this program, so the entry has no effect. Check the name for a typo. Delete the \
         entry if the code it named is gone.",
    );
    /// A function changes a value it was given from a property, and then reads
    /// that property, so the change copies the value
    /// (`rule:types/a-replaced-value-is-changed-in-place`). Raised only where
    /// the read can move above the change, and the fix moves it.
    /// `nvs_ir::copies::warnings` raises it from the ownership pass's table, for
    /// `nvs check` and for the editor.
    pub const W_COPY_READ_AFTER_CHANGE: Code = Code::new("W1032").card(
        "This function changes a value that came from a property. Later, it reads that property \
         again. The read must see the old value, so the change copies the whole value. Move the \
         read above the change. Then the value is changed in place, and nothing is copied.",
    );
    /// A function changes a value it was given from a property or a variable,
    /// and then a statement can throw an error, so the change copies the value
    /// (`rule:types/a-replaced-value-is-changed-in-place`). Raised only where
    /// that statement can move above the change, and the fix moves it.
    /// `nvs_ir::copies::warnings` raises it, as it raises `W1032`.
    pub const W_COPY_FAILURE_AFTER_CHANGE: Code = Code::new("W1033").card(
        "This function changes a value that it was given. A statement after the change can throw \
         an error. If it throws, the caller must still have the old value, so the change copies \
         the whole value. Move that statement above the change. Then the value is changed in \
         place, and nothing is copied.",
    );
    /// A form the table of cheaper forms has a faster one for, where the
    /// callback writes nothing but can throw an error
    /// (`rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named`).
    /// The faster form calls the callback for fewer entries, so the compiler
    /// does not choose it. `nvs_ir::forms::warnings` raises it from the
    /// recorded forms, for `nvs check` and for the editor, and the fix is an
    /// alternative a person picks.
    pub const W_CHEAPER_FORM: Code = Code::new("W1034").card(
        "This code calls a function for the entries of an array. A faster form gives the same \
         result and calls the function fewer times. The function can throw an error. So the \
         faster form may not throw an error that this code throws. Use the faster form if that \
         error does not matter for the calls it skips.",
    );
    /// Two calls of one reading `Core` member in one function, where the
    /// second does the work of the first again: a cache key read twice, a
    /// query parsed twice, or the file system asked twice about one path
    /// (`rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named`).
    /// Raised only where nothing between the two calls can change the result.
    /// `nvs_ir::rereads::warnings` raises it from the program's rows, for
    /// `nvs check` and for the editor. It has no fix, because the fix needs a
    /// variable name.
    pub const W_REPEATED_READ: Code = Code::new("W1035").card(
        "This code reads the same value twice. The second call does all the work of the first \
         call again. Keep the first result in a variable and use it both times. For several \
         facts about one file path, call `Core\\IO::metadata` once. For several query parameters, \
         call `Core\\Uri::parseQuery` once.",
    );
    /// A `Core\Str::slice` that cuts off a prefix or a suffix the code has
    /// just tested with `Core\Str::startsWith` or `endsWith`
    /// (`rule:expressions/a-prefix-or-suffix-cut-written-by-hand-warns`).
    /// `nvs_ir::hand_cuts::warnings` raises it from the written tree, for
    /// `nvs check` and for the editor, with a fix for one exact shape.
    pub const W_HAND_CUT: Code = Code::new("W1036").card(
        "This code tests a string with `Core\\Str::startsWith` or `Core\\Str::endsWith`. Then it \
         cuts the same prefix or suffix off with `Core\\Str::slice`. `Core\\Str::afterPrefix` and \
         `Core\\Str::beforeSuffix` do both in one call. They return the rest of the string, or \
         `null` when the string does not match.",
    );
    /// A `Core\Config::set` or `restore` whose name is a string template that no key on
    /// `nvs_config::settable::SETTABLE` can fit, whatever its holes contain
    /// (`rule:config/set-changes-only-listed-keys`). Each hole fits one or more characters that
    /// are not a dot. A warning, because the checker cannot see the value.
    pub const W_SET_NEVER_FITS: Code = Code::new("W1037").card(
        "This `Core\\Config::set` or `Core\\Config::restore` call builds the name of a setting \
         from a string template, such as `\"db.{$name}.driver\"`. No setting that a program may change has a name of this form, \
         so the call changes nothing. Check the name. The help line lists the \
         settings a program may change.",
    );
    /// A date pattern written as a literal that has the week-based year `Y` and
    /// no week of the year `w` (`rule:core-classes/a-time-class-builds-its-own-values`).
    /// `Y` is the year an ISO week belongs to, which differs from the calendar
    /// year `y` on a few days around New Year, so `YYYY-MM-dd` is almost always
    /// a typo for `yyyy-MM-dd`. A warning, because a program may mean it. A
    /// pattern built at run time is not checked.
    pub const W_WEEK_YEAR_WITHOUT_WEEK: Code = Code::new("W1038").card(
        "This date pattern has `Y`, the year of the ISO week, and no week number `w`. `Y` is \
         not always the calendar year. On 2024-12-30, `Y` is 2025, because that day is in week 1 \
         of 2025. Write `y` for the calendar year, as in `yyyy-MM-dd`.",
    );
    /// A string a property callback compares with its name parameter that no
    /// property of the class, of a class above it or of a class below it has
    /// (`rule:classes/a-compared-property-name-is-a-use`). `nvs_types::compared_names`
    /// raises it. A warning, because a class below may be declared in another
    /// program.
    pub const W_COMPARED_NAME_IS_NO_PROPERTY: Code = Code::new("W1039").card(
        "A method such as `initProperty` or `onPropertySet` gets the name of a property as its \
         first parameter. This method compares that parameter with a string. No property of the \
         class has that name. No class above it or below it has it either. So the comparison is \
         never true. Check the spelling of the name.",
    );
}
