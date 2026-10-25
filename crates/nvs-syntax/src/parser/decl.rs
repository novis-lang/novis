//! Declarations: classes, interfaces and enums with their members,
//! attributes, and the file-scope forms that sit alongside a statement without
//! being one.
//!
//! Classes ([`Parser::parse_class_decl`]), interfaces
//! ([`Parser::parse_interface_decl`]) and enums
//! ([`Parser::parse_enum_decl`]); their members — properties with PHP 8.4's
//! hooks, consts, methods; `#[...]` attribute groups
//! ([`Parser::parse_attribute_groups`], `rule:attributes/inert-metadata`); and `namespace`, `use`,
//! `autoload` (`rule:programs/autoload`) and the `type`-alias declaration (`rule:statements/nothing-gets-a-second-name`).
//!
//! `trait`, class-body `use TraitName, ...;` and `insteadof` are all
//! parse-time rejected (`E_TRAIT_NOT_SUPPORTED`,
//! [`Parser::report_trait_not_supported`]) rather than built into any AST node
//! — `rule:classes/no-traits`. A class's `implements` list instead grows an optional `by
//! $field` suffix per entry ([`Parser::parse_implements_clause`], `rule:classes/delegation-by-property`). A top-level `function`/`const` is the same shape of reject, one layer
//! out: `rule:classes/no-free-functions-or-constants` makes every function a method and every constant a class
//! constant, so both are parsed and diagnosed rather than left to fail.
//!
//! Part of [`super`]'s one `impl Parser`, split across this directory so a
//! session editing one layer of the grammar does not carry the rest in
//! context. Every item moved here unchanged; the methods are `pub(super)` so
//! they reach across these modules and no further, which is the reach they had
//! when `parser` was a single file.

use super::*;

/// Where a modifier list was written, which decides the modifiers it takes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ModifierSite {
    Class,
    Property,
    Constant,
    Method,
    Parameter,
}

impl ModifierSite {
    /// Whether a modifier belongs at this site at all, before any question
    /// of what else is written beside it. `lateinit` on a parameter is taken
    /// here and judged by `nvs-types`, which knows whether the parameter is a
    /// promoted one.
    fn takes(self, m: Modifier) -> bool {
        use Modifier as M;
        match self {
            Self::Class => matches!(m, M::Abstract | M::Final),
            Self::Property => !matches!(m, M::Abstract | M::Final),
            Self::Constant => matches!(m, M::Public | M::Protected | M::Private),
            Self::Method => matches!(
                m,
                M::Public | M::Protected | M::Private | M::Static | M::Abstract | M::Final
            ),
            Self::Parameter => !matches!(m, M::Static | M::Abstract | M::Final),
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Self::Class => "a class",
            Self::Property => "a property",
            Self::Constant => "a constant",
            Self::Method => "a method",
            Self::Parameter => "a parameter",
        }
    }
}

/// Two distinct modifiers that cannot be written on one declaration: two
/// read visibilities, two `(set)` visibilities, or `abstract` with `final`.
fn conflicts(a: Modifier, b: Modifier) -> bool {
    use Modifier as M;
    let visibility = |m| matches!(m, M::Public | M::Protected | M::Private);
    (visibility(a) && visibility(b))
        || matches!((a, b), (M::SetVisibility(_), M::SetVisibility(_)))
        || matches!((a, b), (M::Abstract, M::Final) | (M::Final, M::Abstract))
}

impl<'src, 'd> Parser<'src, 'd> {
    // ========================================================================
    // Attributes (`#[...]`)
    // ========================================================================

    /// Zero or more `#[...]` groups, in source order — the standard prefix
    /// of every declaration site (a class, a property, a method, a
    /// parameter, an enum case, ...).
    pub(super) fn parse_attribute_groups(&mut self) -> Vec<AttributeGroup> {
        let mut groups = Vec::new();
        while self.at(TokenKind::AttributeOpen) {
            groups.push(self.parse_attribute_group());
        }
        groups
    }

    /// One `#[...]` group.
    ///
    /// A malformed group is one error. When the group does not end at its
    /// `]`, "expected `]`" is reported only if nothing inside the group was
    /// reported already, and the tokens up to the group's own `]` are then
    /// skipped by [`Self::skip_to_attribute_close`]. Without the skip, the
    /// declaration after the group is parsed from the middle of the
    /// attribute, and every token left over is a further error.
    pub(super) fn parse_attribute_group(&mut self) -> AttributeGroup {
        let start = self.bump().span; // '#['
        let reported = self.diags.len();
        let mut attributes = vec![self.parse_attribute()];
        while self.eat(TokenKind::Comma).is_some() && !self.at(TokenKind::RBracket) {
            attributes.push(self.parse_attribute());
        }
        let close = match self.eat(TokenKind::RBracket) {
            Some(close) => close,
            None => {
                if self.diags.len() == reported {
                    self.error_expected("`]`");
                }
                self.skip_to_attribute_close()
            }
        };
        AttributeGroup {
            attributes,
            span: start.to(close),
        }
    }

    /// Skips the rest of a malformed attribute group and returns the span of
    /// the last token it consumed: the group's own `]` when there is one.
    ///
    /// Brackets, parentheses and braces are counted, so a `]` inside the
    /// payload does not end the group. Outside every bracket, a `;`, an
    /// unmatched closing bracket, or a keyword that starts a declaration also
    /// ends the skip, without being consumed. A group with no `]` at all then
    /// loses only its own tokens, and the declaration after it still parses.
    fn skip_to_attribute_close(&mut self) -> Span {
        let mut depth = 0usize;
        loop {
            match self.peek().kind {
                TokenKind::Eof => return self.last_span,
                TokenKind::RBracket if depth == 0 => return self.bump().span,
                TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::LBrace
                | TokenKind::AttributeOpen => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    if depth == 0 {
                        return self.last_span;
                    }
                    depth -= 1;
                }
                TokenKind::Semicolon if depth == 0 => return self.last_span,
                TokenKind::Keyword(
                    Keyword::Abstract
                    | Keyword::Class
                    | Keyword::Const
                    | Keyword::Enum
                    | Keyword::Final
                    | Keyword::Function
                    | Keyword::Interface
                    | Keyword::Private
                    | Keyword::Protected
                    | Keyword::Public
                    | Keyword::Readonly,
                ) if depth == 0 => return self.last_span,
                _ => {}
            }
            self.bump();
        }
    }

    /// One attribute — `rule:attributes/attach-sites-and-forms`'s named `Name(field: value, ...)` or bare
    /// `{ field: value, ... }`. Both carry the same payload, so the
    /// parenthesized list is parsed by the very function that parses an
    /// `rule:types/anonymous-object` anonymous object's fields: an attribute payload is that
    /// object written without its braces, not an argument list, so a
    /// positional argument is "expected a field name" where it is written
    /// rather than something a later pass has to refuse.
    ///
    /// The named form's name may be `Owner::Name`, an alias declared in the
    /// body of `Owner` (`rule:types/class-scoped-alias`), as it may in type
    /// position. A `::` with no name after it is one error, and the attribute
    /// is then kept as the bare form, so the checker reports nothing more about
    /// it.
    pub(super) fn parse_attribute(&mut self) -> Attribute {
        let start = self.peek().span;
        if self.at(TokenKind::LBrace) {
            let open = self.bump().span; // '{'
            let fields = self.parse_anon_object_fields(TokenKind::RBrace);
            let close = self.expect(TokenKind::RBrace, "`}`");
            let payload = open.to(close);
            return Attribute {
                name: None,
                member: None,
                fields,
                payload,
                span: payload,
            };
        }
        let name = self.parse_name();
        let mut named = true;
        let mut member = None;
        if self.eat(TokenKind::DoubleColon).is_some() {
            if Self::is_name_segment(self.peek().kind) {
                member = Some(self.bump().span);
            } else {
                self.error_expected("the name of a `type` alias after `::`");
                named = false;
            }
        }
        let written = member.map_or(name.span, |member| name.span.to(member));
        let (fields, payload) = if self.at(TokenKind::LParen) {
            let open = self.bump().span; // '('
            let fields = self.parse_anon_object_fields(TokenKind::RParen);
            let close = self.expect(TokenKind::RParen, "`)`");
            (fields, open.to(close))
        } else {
            (Vec::new(), written)
        };
        Attribute {
            name: named.then_some(name),
            member,
            fields,
            payload,
            span: start.to(self.last_span),
        }
    }

    // ========================================================================
    // Declaration modifiers
    // ========================================================================

    /// A modifier list at a site whose kind is known before the list is
    /// read — a class header or a parameter — read and checked in one call.
    pub(super) fn parse_modifiers(&mut self, site: ModifierSite) -> Vec<Modifier> {
        let written = self.parse_written_modifiers();
        self.check_modifiers(site, &written)
    }

    /// `E_BAD_MODIFIER`: a modifier written twice, a second visibility (or a
    /// second `(set)` visibility), `abstract` beside `final`, or a modifier
    /// `site` does not take. Each is reported where it is written and left
    /// out of the list returned, so no later pass reads a modifier the
    /// source was told to delete; the first of a conflicting pair is kept.
    pub(super) fn check_modifiers(
        &mut self,
        site: ModifierSite,
        written: &[WrittenModifier],
    ) -> Vec<Modifier> {
        let mut kept: Vec<Modifier> = Vec::with_capacity(written.len());
        for w in written {
            let text = self.file.span_text(w.span).unwrap_or_default();
            let problem = if kept.contains(&w.modifier) {
                Some(format!("`{text}` is written twice"))
            } else if !site.takes(w.modifier) {
                Some(format!("`{text}` is not allowed on {}", site.noun()))
            } else {
                kept.iter()
                    .find(|k| conflicts(**k, w.modifier))
                    .map(|other| match other {
                        Modifier::Abstract | Modifier::Final => format!(
                            "`abstract` and `final` cannot both be written on {}",
                            site.noun()
                        ),
                        _ => format!(
                            "{} has one visibility, and `{text}` is a second",
                            site.noun()
                        ),
                    })
            };
            match problem {
                Some(message) => self.diags.report(
                    Diagnostic::error(code::E_BAD_MODIFIER, message)
                        .with_primary(w.span, "delete this modifier"),
                ),
                None => kept.push(w.modifier),
            }
        }
        kept
    }

    /// Every modifier the parser knows, in any combination and any order —
    /// a class header, a property, a constant, a method and a parameter all
    /// call this one loop, and [`Self::check_modifiers`] then judges the list
    /// against the site it turned out to be at.
    ///
    /// Being the one loop is also what makes a file's modifier lists
    /// collectable without a second walk over the grammar: a parse that keeps
    /// positions ([`Parser::with_trivia`]) records each list here, in source
    /// order, and no other production writes one.
    pub(super) fn parse_written_modifiers(&mut self) -> Vec<WrittenModifier> {
        let mut written = Vec::new();
        self.covariant_before_modifier = false;
        loop {
            let at = self.peek().span;
            if self.at_covariant_before_modifier() {
                self.bump();
                self.covariant_before_modifier = true;
                self.diags.report(
                    Diagnostic::error(
                        code::E_BAD_MODIFIER,
                        "`covariant` is written just before the type",
                    )
                    .with_primary(at, "move this after the other modifiers")
                    .with_help("write `public covariant Dog $pet` or `inout covariant Dog $a`"),
                );
                continue;
            }
            let m = match self.peek().kind {
                TokenKind::Keyword(Keyword::Public) => {
                    self.parse_visibility_modifier(Visibility::Public)
                }
                TokenKind::Keyword(Keyword::Protected) => {
                    self.parse_visibility_modifier(Visibility::Protected)
                }
                TokenKind::Keyword(Keyword::Private) => {
                    self.parse_visibility_modifier(Visibility::Private)
                }
                TokenKind::Keyword(Keyword::Readonly) => {
                    self.bump();
                    Modifier::Readonly
                }
                TokenKind::Keyword(Keyword::Lateinit) => {
                    self.bump();
                    Modifier::Lateinit
                }
                TokenKind::Keyword(Keyword::Static) => {
                    self.bump();
                    Modifier::Static
                }
                TokenKind::Keyword(Keyword::Abstract) => {
                    self.bump();
                    Modifier::Abstract
                }
                TokenKind::Keyword(Keyword::Final) => {
                    self.bump();
                    Modifier::Final
                }
                _ => break,
            };
            written.push(WrittenModifier {
                modifier: m,
                span: at.to(self.last_span),
            });
        }
        if let Some(runs) = self.modifiers.as_mut()
            && !written.is_empty()
        {
            runs.push(written.clone());
        }
        written
    }

    /// The contextual `covariant` of a parameter or a property
    /// (`rule:classes/a-covariant-type-is-checked-where-a-value-enters`),
    /// eaten when it is written after the modifiers and the next token starts
    /// a type. Anywhere else the word is an identifier, so `covariant $x` is a
    /// parameter typed by a class named `covariant` and `->covariant()` is a
    /// method call.
    ///
    /// A `covariant` written in front of a modifier or `inout` was reported
    /// by [`Self::parse_written_modifiers`], which read it there, and counts
    /// here: the declaration is still covariant, so the checker reports
    /// nothing a second time about the same word.
    pub(super) fn eat_covariant(&mut self) -> bool {
        let misplaced = std::mem::take(&mut self.covariant_before_modifier);
        if !self.at_contextual("covariant") {
            return misplaced;
        }
        let next = self.peek_at(1).kind;
        let starts_type = Self::token_starts_type(next)
            || matches!(next, TokenKind::Minus | TokenKind::Keyword(Keyword::Class));
        if starts_type {
            self.bump();
        }
        starts_type || misplaced
    }

    /// `covariant` in front of a modifier or `inout`, which is the one place
    /// before its own that the word can be read as the keyword: an
    /// identifier followed by a modifier is no other construct.
    fn at_covariant_before_modifier(&mut self) -> bool {
        self.at_contextual("covariant")
            && matches!(
                self.peek_at(1).kind,
                TokenKind::Keyword(
                    Keyword::Public
                        | Keyword::Protected
                        | Keyword::Private
                        | Keyword::Readonly
                        | Keyword::Lateinit
                        | Keyword::Static
                        | Keyword::Abstract
                        | Keyword::Final
                        | Keyword::Inout
                )
            )
    }

    /// `public`/`protected`/`private`, optionally followed by PHP 8.4's
    /// asymmetric-visibility suffix `(set)` — `private(set)` etc. — which
    /// becomes [`Modifier::SetVisibility`] instead of the plain form.
    pub(super) fn parse_visibility_modifier(&mut self, v: Visibility) -> Modifier {
        self.bump();
        if self.eat(TokenKind::LParen).is_none() {
            return match v {
                Visibility::Public => Modifier::Public,
                Visibility::Protected => Modifier::Protected,
                Visibility::Private => Modifier::Private,
            };
        }
        if self.at_contextual("set") {
            self.bump();
        } else {
            self.error_expected("`set`");
        }
        self.expect(TokenKind::RParen, "`)`");
        Modifier::SetVisibility(v)
    }

    // ========================================================================
    // Shared declaration helpers
    // ========================================================================

    /// One unqualified declared name — a class, interface, trait, enum,
    /// method, constant or `type`-alias name. Unlike [`Self::parse_name`],
    /// this never admits a `\`-qualified path: nothing is ever declared
    /// under a path, only referred to by one. A keyword-shaped spelling is
    /// accepted, same as a member name after `->`/`::`, except `let` and
    /// `is`: they name nothing (`rule:statements/let-and-is-are-reserved`),
    /// so either is reported as `E0247` and kept as the name.
    pub(super) fn parse_decl_name(&mut self, what: &str) -> Name {
        let span = match self.peek().kind {
            TokenKind::Keyword(word @ (Keyword::Let | Keyword::Is)) => {
                let span = self.bump().span;
                self.report_reserved_for_future_use(word, span);
                span
            }
            kind if Self::is_name_segment(kind) => self.bump().span,
            _ => self.error_expected(what),
        };
        Name { span }
    }

    /// `extends`/`implements`'s comma-separated name list — shared by every
    /// declaration that has one.
    pub(super) fn parse_name_list(&mut self) -> Vec<Name> {
        let mut names = vec![self.parse_name()];
        while self.eat(TokenKind::Comma).is_some() {
            names.push(self.parse_name());
        }
        names
    }

    /// `rule:core-api/reserved-namespace`: `Core` is reserved for built-ins. Reports and keeps
    /// going.
    pub(super) fn check_reserved_core_namespace(&mut self, name: &Name) {
        let text = self.file.span_text(name.span).unwrap_or_default();
        let first_segment = text.split('\\').next().unwrap_or(text);
        if first_segment.eq_ignore_ascii_case("Core") {
            self.diags.report(
                Diagnostic::error(
                    code::E_RESERVED_CORE_NAMESPACE,
                    "`Core` is reserved for built-ins",
                )
                .with_primary(name.span, "not available to user code"),
            );
        }
    }

    // ========================================================================
    // `namespace`, `use`, `autoload`, `type` alias — file-scope declarations
    // ========================================================================

    pub(super) fn parse_namespace_decl(&mut self, start: Span) -> Stmt {
        self.bump(); // 'namespace'
        let name = if matches!(self.peek().kind, TokenKind::Ident | TokenKind::Backslash) {
            Some(self.parse_name())
        } else {
            None
        };
        if let Some(name) = &name {
            self.check_reserved_core_namespace(name);
        }
        let body = if self.at(TokenKind::LBrace) {
            let brace = self.peek().span;
            // The braced form is refused and then parsed anyway — the block's
            // declarations are still the ones the author wrote, so this file
            // reports what is wrong inside them in the same run. `rule:security/authority-is-the-enclosing-namespace` keys
            // authority on the enclosing namespace, and a file that is two
            // namespaces has an authority that depends on the line number.
            self.diags.report(
                Diagnostic::error(
                    code::E_BRACED_NAMESPACE_UNSUPPORTED,
                    "a `namespace` declaration is a statement, not a block",
                )
                .with_primary(brace, "a namespace is not opened here")
                .with_help(
                    "write `namespace X;` once, before any declaration, and put a second \
                     namespace in a second file",
                ),
            );
            Some(self.parse_block())
        } else {
            self.expect(TokenKind::Semicolon, "`;`");
            None
        };
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::NamespaceDecl(NamespaceDecl { span, name, body }),
        }
    }

    /// One file is one namespace, so a `namespace X;` statement after another
    /// namespace, or after a declaration it would not cover, is `E0243`. The
    /// braced form is reported where [`Self::parse_namespace_decl`] parses it.
    pub(super) fn check_one_namespace(&mut self, stmts: &[Stmt]) {
        let mut first: Option<(Span, bool)> = None;
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::NamespaceDecl(ns) if ns.body.is_none() => {
                    if let Some((at, is_namespace)) = first {
                        let (message, label) = if is_namespace {
                            (
                                "a file has only one namespace",
                                "the first namespace is here",
                            )
                        } else {
                            (
                                "a `namespace` statement comes after a declaration",
                                "this declaration comes first",
                            )
                        };
                        self.diags.report(
                            Diagnostic::error(code::E_BRACED_NAMESPACE_UNSUPPORTED, message)
                                .with_primary(stmt.span, "this namespace is not allowed here")
                                .with_secondary(at, label)
                                .with_help(
                                    "write `namespace X;` once, before any declaration, and put \
                                     a second namespace in a second file",
                                ),
                        );
                    } else {
                        first = Some((stmt.span, true));
                    }
                }
                StmtKind::NamespaceDecl(_) => {
                    first.get_or_insert((stmt.span, true));
                }
                StmtKind::ClassDecl(_)
                | StmtKind::InterfaceDecl(_)
                | StmtKind::EnumDecl(_)
                | StmtKind::TypeAliasDecl(_)
                | StmtKind::TopLevelFunction(_)
                | StmtKind::TopLevelConst(_) => {
                    first.get_or_insert((stmt.span, false));
                }
                _ => {}
            }
        }
    }

    /// `use App\Models\User;` — one import, one statement.
    ///
    /// Two PHP spellings of the same statement are refused rather than
    /// parsed: renaming (`as Other`) and the group form
    /// (`use App\Models\{User, Post};`), both refused by
    /// `rule:statements/nothing-gets-a-second-name`. Both are reported and then skipped, so the
    /// statement still yields a [`UseDecl`] for the path that was written and
    /// nothing downstream sees a half-parsed import.
    pub(super) fn parse_use_decl(&mut self, start: Span) -> Stmt {
        self.bump(); // 'use'
        if let Some(kind) = self.at_php_symbol_import() {
            self.refuse_symbol_import(kind);
        }
        let path = self.parse_name();
        if self.at(TokenKind::Backslash) && self.peek_at(1).kind == TokenKind::LBrace {
            self.recover_use_group(path.span);
        }
        if self.at(TokenKind::DoubleColon) {
            return self.refuse_member_import(start, path.span);
        }
        let alias = if self.eat_keyword(Keyword::As).is_some() {
            Some(self.expect(TokenKind::Ident, "an alias name"))
        } else {
            None
        };
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        if let Some(alias) = alias {
            self.diags.report(
                Diagnostic::error(
                    code::E_IMPORT_ALIAS_UNSUPPORTED,
                    "an import cannot be renamed",
                )
                .with_primary(alias, "rename not supported")
                .with_help("write its declared short name, or write the fully-qualified path"),
            );
        }
        Stmt {
            span,
            kind: StmtKind::UseDecl(UseDecl { span, path, alias }),
        }
    }

    /// `use Shop\Order::Status;`: an import names a class, interface or enum,
    /// never a member of one. A member import would give an enum in a class
    /// body a second short name (`rule:enums/class-scoped-enum`). The statement
    /// is one diagnostic and imports nothing, so the owner's own import beside
    /// it is no duplicate.
    fn refuse_member_import(&mut self, start: Span, owner: Span) -> Stmt {
        let colons = self.bump().span;
        let member = self.expect(TokenKind::Ident, "a member name");
        let owner_text = self.file.span_text(owner).unwrap_or_default().to_owned();
        let short = owner_text
            .rsplit('\\')
            .next()
            .unwrap_or_default()
            .to_owned();
        let member_text = self.file.span_text(member).unwrap_or_default().to_owned();
        self.diags.report(
            Diagnostic::error(
                code::E_IMPORT_ALIAS_UNSUPPORTED,
                "an import cannot name a member of a class",
            )
            .with_primary(colons.to(member), "a member of a class")
            .with_help(format!(
                "import the class with `use {owner_text};` and write `{short}::{member_text}`"
            )),
        );
        self.expect(TokenKind::Semicolon, "`;`");
        Stmt {
            span: start.to(self.last_span),
            kind: StmtKind::Error,
        }
    }

    /// Whether the `use` just consumed opens PHP's `use function Foo\bar;` or
    /// `use const Foo\BAZ;`, and which of the two.
    ///
    /// The tell is a second name segment following the keyword directly:
    /// `function` and `const` are ordinary segments like every other keyword
    /// spelling, so `use function\Foo;` imports a name out of a namespace
    /// called `function` and stays an ordinary import, while `use function
    /// Foo\bar;` puts two segments side by side, which no import can be.
    fn at_php_symbol_import(&mut self) -> Option<&'static str> {
        let kind = match self.peek().kind {
            TokenKind::Keyword(Keyword::Function) => "function",
            TokenKind::Keyword(Keyword::Const) => "const",
            _ => return None,
        };
        matches!(
            self.peek_at(1).kind,
            TokenKind::Ident | TokenKind::Keyword(_)
        )
        .then_some(kind)
    }

    /// Reports the `use function`/`use const` refusal and eats the keyword,
    /// leaving the caller on the path it introduced — which parses as the
    /// ordinary import it is written like, so nothing downstream sees a
    /// half-parsed statement.
    ///
    /// `rule:classes/no-free-functions-or-constants` is the whole reason: a
    /// function is a method and a constant is a class constant, so there is
    /// nothing either spelling could name, and the help says what to import
    /// instead.
    fn refuse_symbol_import(&mut self, kind: &str) {
        let keyword = self.bump().span;
        self.diags.report(
            Diagnostic::error(
                code::E_IMPORT_OF_FUNCTION_OR_CONST_UNSUPPORTED,
                format!("`use {kind}` imports a kind of name this language does not have"),
            )
            .with_primary(keyword, format!("no free {kind} to import"))
            .with_help(
                "every function is a method and every constant is a class constant. Import the \
                 class that declares it and write `Class::name`",
            ),
        );
    }

    /// Reports the group-use refusal and eats `\{ ... }`, leaving the caller
    /// at the `;` it was already going to expect.
    fn recover_use_group(&mut self, path: Span) {
        let open = self.bump().span; // '\'
        self.bump(); // '{'
        let mut depth = 1usize;
        let mut last = self.last_span;
        while depth > 0 && !self.at(TokenKind::Eof) {
            let tok = self.bump();
            match tok.kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth -= 1,
                _ => {}
            }
            last = tok.span;
        }
        self.diags.report(
            Diagnostic::error(
                code::E_IMPORT_GROUP_UNSUPPORTED,
                "an import cannot name a group of names",
            )
            .with_primary(open.to(last), "group import not supported")
            .with_help(
                "write one `use` statement per imported name, each ending in the short name it \
                 introduces",
            )
            .with_secondary(path, "the shared prefix"),
        );
    }

    /// `autoload 'Prefix' from 'a', 'b';` and `autoload discover 'glob';` —
    /// `rule:programs/autoload`'s two forms, spelled the way
    /// [`docs/reference/lang/10-programs.md`](/docs/reference/lang/10-programs.md)
    /// writes them.
    ///
    /// A file-scope form that is not a statement, exactly like
    /// [`Self::parse_use_decl`] beside it: it declares a rule the whole
    /// compilation reads, and reaches `parse_statement` only because that is
    /// where the token stream arrives. Whether it sits at a file's top level,
    /// and whether that file is reachable by `require` from the entry point,
    /// are `nvs_hir`'s checks — the parser can see neither.
    ///
    /// `discover` is **contextual**: an ordinary identifier everywhere else,
    /// meaning the second form only in this one position, so no existing
    /// member or class named `discover` stops compiling.
    pub(super) fn parse_autoload_decl(&mut self, start: Span) -> Stmt {
        self.bump(); // 'autoload'
        let kind = if self.at_contextual("discover") {
            self.bump();
            match self.parse_written_autoload_path("a quoted glob") {
                Some(glob) => AutoloadKind::Discover { glob },
                None => return self.recover_autoload_decl(start),
            }
        } else {
            let Some(prefix) = self.parse_written_autoload_path("a quoted namespace prefix") else {
                return self.recover_autoload_decl(start);
            };
            if self.at_contextual("from") {
                self.bump();
            } else {
                self.error_expected("`from`");
                return self.recover_autoload_decl(start);
            }
            let mut roots = Vec::new();
            loop {
                let Some(root) = self.parse_written_autoload_path("a quoted root path") else {
                    return self.recover_autoload_decl(start);
                };
                roots.push(root);
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            AutoloadKind::Prefix { prefix, roots }
        };
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::AutoloadDecl(AutoloadDecl { span, kind }),
        }
    }

    /// One quoted string inside an `autoload` declaration — its prefix, one
    /// of its roots, or its glob.
    ///
    /// Only a literal is accepted, the restriction `require`'s static
    /// resolution already carries (`rule:statements/require-is-the-only-inclusion-construct`,
    /// `rule:programs/autoload`) and for the same reason: the map is built at compile
    /// time, so a path assembled at run time could not contribute to it. A
    /// double-quoted spelling is read for its escapes and refused if it
    /// interpolates — the parser is the only place that is visible.
    ///
    /// The span returned covers the whole literal, quotes included, so
    /// `nvs_hir` decodes it with the same `cook_quoted` a `require` path goes
    /// through.
    fn parse_written_autoload_path(&mut self, what: &str) -> Option<Span> {
        let span = match self.peek().kind {
            TokenKind::SingleQuotedString => self.bump().span,
            TokenKind::DoubleQuoteOpen => {
                let open = self.bump().span;
                let (parts, close) = self.parse_string_body(TokenKind::DoubleQuoteClose);
                let span = open.to(close);
                if !parts.iter().all(|p| matches!(p, StringPart::Text(_))) {
                    self.report_autoload_not_written(span, format!("expected {what}"));
                    return None;
                }
                span
            }
            _ => {
                let span = self.peek().span;
                self.report_autoload_not_written(span, format!("expected {what}"));
                return None;
            }
        };
        // The other way a path stops being a literal, and the one a reader
        // would otherwise see reported as a missing `;`: a concatenation
        // whose first operand happens to be one.
        if self.at(TokenKind::Dot) {
            let dot = self.peek().span;
            self.report_autoload_not_written(dot, "a path cannot be built by concatenation");
            return None;
        }
        Some(span)
    }

    fn report_autoload_not_written(&mut self, span: Span, label: impl Into<String>) {
        self.diags.report(
            Diagnostic::error(
                code::E_AUTOLOAD_PATH_NOT_WRITTEN_DIRECTLY,
                "an `autoload` declaration needs strings written directly in the code",
            )
            .with_primary(span, label)
            .with_help(
                "write the path as a plain string. The path is read at compile time, relative \
                 to this file",
            ),
        );
    }

    /// Swallows the rest of a malformed `autoload` declaration through its
    /// `;`, so one unwritable path reports one diagnostic rather than also an
    /// "expected `;`" from whatever token the parser stopped on.
    fn recover_autoload_decl(&mut self, start: Span) -> Stmt {
        while !matches!(
            self.peek().kind,
            TokenKind::Semicolon | TokenKind::Eof | TokenKind::CloseTag
        ) {
            self.bump();
        }
        self.eat(TokenKind::Semicolon);
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Error,
        }
    }

    pub(super) fn parse_type_alias_decl(&mut self, start: Span) -> Stmt {
        let doc = self.take_doc_comment(start);
        let alias = self.parse_type_alias_body(start, doc);
        Stmt {
            span: alias.span,
            kind: StmtKind::TypeAliasDecl(alias),
        }
    }

    /// `type` is contextual at each of the sites an alias is written — file
    /// scope, and a class, interface or enum body — so the three tokens that
    /// tell the declaration from an identifier expression, a property or an
    /// enum case are asked for in one place. A `let` or `is` name is admitted
    /// here so that [`Self::parse_decl_name`] reports it as `E0247`.
    pub(super) fn at_type_alias(&mut self) -> bool {
        self.at_contextual("type")
            && matches!(
                self.peek_at(1).kind,
                TokenKind::Ident | TokenKind::Keyword(Keyword::Let | Keyword::Is)
            )
            && self.peek_at(2).kind == TokenKind::Equals
    }

    /// A modifier run or an attribute group written in front of a body's
    /// `type` is `E0133`. An alias is reachable wherever its owner's name is,
    /// so there is no visibility to write, and nothing downstream of the
    /// checker sees the name, so an attribute has nothing to attach to
    /// (`rule:types/type-alias`). Both runs sit between the member's start and
    /// the keyword, attributes first, so one span covers whichever was
    /// written. The declaration itself is kept: only its decoration is
    /// refused, the same discipline the `case` keyword in an enum body gets.
    fn refuse_decoration_on_a_type_alias(
        &mut self,
        attributes: &[AttributeGroup],
        modifiers: Option<Span>,
    ) {
        let Some(first) = attributes.first().map(|g| g.span).or(modifiers) else {
            return;
        };
        let last = modifiers
            .or_else(|| attributes.last().map(|g| g.span))
            .unwrap_or(first);
        self.diags.report(
            Diagnostic::error(
                code::E_TYPE_ALIAS_TAKES_NO_MODIFIER_OR_ATTRIBUTE,
                "a `type` alias takes no modifier and no attribute group",
            )
            .with_primary(first.to(last), "remove this")
            .with_help(
                "an alias can be used wherever its class can be used, so it has no visibility. \
                 An alias exists only at compile time, so an attribute on it has no effect",
            ),
        );
    }

    /// A modifier run in front of a body's `enum` is `E0133`, the code an
    /// alias's decoration gets. The enum's cases leave the class in every
    /// return value, so a visibility would hide the name and not the values
    /// (`rule:enums/class-scoped-enum`). The declaration itself is kept.
    fn refuse_modifiers_on_a_member_enum(&mut self, modifiers: Span) {
        self.diags.report(
            Diagnostic::error(
                code::E_TYPE_ALIAS_TAKES_NO_MODIFIER_OR_ATTRIBUTE,
                "an enum in a class body takes no modifier",
            )
            .with_primary(modifiers, "remove this")
            .with_help(
                "the enum's cases can be returned outside the class. A visibility would hide \
                 only the enum's name",
            ),
        );
    }

    /// The declaration from `type` through its `;`, shared by the file-scope
    /// form and the body member. `doc` is the run the caller already took: a
    /// member's hangs on its [`ClassMember`], so that site passes `None`.
    fn parse_type_alias_body(&mut self, start: Span, doc: Option<DocComment>) -> TypeAliasDecl {
        self.bump(); // 'type' (contextual — see `Self::at_type_alias`)
        let name = self.parse_decl_name("a type alias name");
        self.expect(TokenKind::Equals, "`=`");
        let ty = self.parse_type();
        self.expect(TokenKind::Semicolon, "`;`");
        TypeAliasDecl {
            span: start.to(self.last_span),
            doc,
            name,
            ty,
        }
    }

    // ========================================================================
    // Classes, interfaces, traits (`rule:classes/no-free-functions-or-constants`/4, `rule:classes/no-traits`)
    // ========================================================================

    pub(super) fn parse_class_decl(&mut self, start: Span) -> Stmt {
        self.finish_class_decl(start, Vec::new())
    }

    pub(super) fn finish_class_decl(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> Stmt {
        let doc = self.take_doc_comment(start);
        let modifiers = self.parse_modifiers(ModifierSite::Class);
        self.expect_keyword(Keyword::Class, "`class`");
        let name = self.parse_decl_name("a class name");
        let extends = if self.eat_keyword(Keyword::Extends).is_some() {
            Some(self.parse_name())
        } else {
            None
        };
        let implements = if self.eat_keyword(Keyword::Implements).is_some() {
            self.parse_implements_list()
        } else {
            Vec::new()
        };
        let members = self.parse_class_body();
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::ClassDecl(ClassDecl {
                span,
                doc,
                attributes,
                modifiers,
                name,
                extends,
                implements,
                members,
            }),
        }
    }

    /// `implements`'s comma-separated list, each entry optionally suffixed
    /// with `by $field` (`rule:classes/delegation-by-property`) — the class-only extension of
    /// [`Self::parse_name_list`], which every other `extends`/`implements`
    /// list (an interface's `extends`, an enum's rejected `implements`, an
    /// anonymous class's `implements`) still uses unchanged, since
    /// delegation is meaningless without a constructor to assign the target
    /// field.
    pub(super) fn parse_implements_list(&mut self) -> Vec<ImplementsClause> {
        let mut clauses = vec![self.parse_implements_clause()];
        while self.eat(TokenKind::Comma).is_some() {
            clauses.push(self.parse_implements_clause());
        }
        clauses
    }

    /// `Name ('<' T, ... '>')? ('by' '$'field)?`. `by` is a contextual
    /// keyword, the same shape [`Self::parse_type_alias_decl`]'s `type`
    /// already is — it has no other meaning as a bare identifier immediately
    /// after an `implements` name, so no reserved word was needed for it.
    ///
    /// The type-argument list is `rule:iteration/concrete-generic-implements`'s `implements Iterable<int>`,
    /// parsed by the same [`Self::parse_type_args`] a name in type position
    /// uses, so the two spellings cannot drift apart.
    pub(super) fn parse_implements_clause(&mut self) -> ImplementsClause {
        let name = self.parse_name();
        let (type_args, mut span) = self.parse_type_args(name.span);
        let by_field = if self.at_contextual("by") {
            self.bump();
            let field = self.expect(
                TokenKind::Variable,
                "the delegated-to property, `$property`",
            );
            span = span.to(field);
            Some(field)
        } else {
            None
        };
        ImplementsClause {
            name,
            type_args,
            by_field,
            span,
        }
    }

    pub(super) fn parse_interface_decl(&mut self, start: Span) -> Stmt {
        self.finish_interface_decl(start, Vec::new())
    }

    pub(super) fn finish_interface_decl(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> Stmt {
        let doc = self.take_doc_comment(start);
        self.bump(); // 'interface'
        let name = self.parse_decl_name("an interface name");
        let extends = if self.eat_keyword(Keyword::Extends).is_some() {
            self.parse_name_list()
        } else {
            Vec::new()
        };
        let members = self.parse_class_body();
        // `rule:classes/interfaces-declare-no-state`: the body grammar is the
        // class's, so a property parses here, and here is where it is refused
        // — at the declaration, the way an enum body refuses a method — rather
        // than by `nvs_types::layout`, which carries no property for an
        // interface and would leave the first read to fail in codegen.
        for member in &members {
            if matches!(member.kind, ClassMemberKind::Property(_)) {
                self.diags.report(
                    Diagnostic::error(
                        code::E_INTERFACE_PROPERTY_UNSUPPORTED,
                        "an interface declares no property",
                    )
                    .with_primary(member.span, "declared inside an interface")
                    .with_help(
                        "declare a method that every class must write instead. You can also \
                         declare a typed constant and read it as `static::NAME` in a default \
                         method. For shared state, write `implements I by $property;` in the class",
                    ),
                );
            }
        }
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::InterfaceDecl(InterfaceDecl {
                span,
                doc,
                attributes,
                name,
                extends,
                members,
            }),
        }
    }

    pub(super) fn parse_trait_decl(&mut self, start: Span) -> Stmt {
        self.finish_trait_decl(start, Vec::new())
    }

    /// `trait Name { ... }` — rejected outright (`rule:classes/no-traits`): there is no
    /// `TraitDecl` AST node left to build, so this still consumes the whole
    /// declaration (name through the closing brace, via
    /// [`Self::parse_class_body`], its members discarded) so a malformed
    /// trait body cannot desynchronize the parser, then reports
    /// [`Self::report_trait_not_supported`] and produces a plain
    /// [`StmtKind::Error`].
    pub(super) fn finish_trait_decl(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> Stmt {
        let _ = attributes;
        self.bump(); // 'trait'
        self.parse_decl_name("a trait name");
        self.parse_class_body();
        let span = start.to(self.last_span);
        self.report_trait_not_supported(span);
        Stmt {
            span,
            kind: StmtKind::Error,
        }
    }

    /// `rule:classes/no-traits`: `E_TRAIT_NOT_SUPPORTED` for any of the three
    /// removed constructs — a `trait` declaration, a class-body
    /// `use TraitName, ...;`, or an `insteadof` adaptation (which, with the
    /// whole adaptation-block grammar gone, can now only ever be encountered
    /// as part of the `use` block this same diagnostic already covers).
    pub(super) fn report_trait_not_supported(&mut self, span: Span) {
        self.diags.report(
            Diagnostic::error(code::E_TRAIT_NOT_SUPPORTED, "traits do not exist")
                .with_primary(span, "not supported")
                .with_help(
                    "for shared methods, write a default or private method in an interface. \
                     For shared state, write `implements Interface by $property;`",
                ),
        );
    }

    /// A `{ ... }` class/interface/trait body. Mirrors [`Self::parse_block`]'s
    /// force-progress guard exactly, for the same reason: a malformed member
    /// must not hang the parser.
    pub(super) fn parse_class_body(&mut self) -> Vec<ClassMember> {
        self.expect(TokenKind::LBrace, "`{`");
        let mut members = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            self.parse_class_member(&mut members);
            if self.peek().span == before && !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        members
    }

    pub(super) fn parse_class_member(&mut self, out: &mut Vec<ClassMember>) {
        let start = self.peek().span;
        // Before the attributes, because the `///` run is written above them
        // (`rule:tooling/doc-comment-attaches-to-the-next-declaration`), and
        // over the members this one declaration produces rather than the first,
        // because `public int $a, $b;` is one declaration and the run
        // documents it.
        let doc = self.take_doc_comment(start);
        let first = out.len();
        let attributes = self.parse_attribute_groups();
        // Only a class/interface/anonymous-class body redirects `var`; an
        // enum body reaches `parse_class_member_with_attrs` directly and
        // already reports `E_ENUM_MEMBER_UNSUPPORTED` for whatever it holds,
        // which is the one diagnostic `rule:core-api/written-visibility`'s scope leaves it.
        if self.at_keyword(Keyword::Var) {
            out.push(self.parse_class_body_var(start, attributes));
        } else {
            self.parse_class_member_with_attrs(start, attributes, out);
        }
        if let Some(doc) = doc {
            for member in &mut out[first..] {
                member.doc = Some(doc.clone());
            }
        }
    }

    /// One or more members can come from a single source construct — a
    /// property or constant may name several declarators at once
    /// (`public int $a, $b;`) — so this pushes into `out` rather than
    /// returning a single [`ClassMember`].
    pub(super) fn parse_class_member_with_attrs(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
        out: &mut Vec<ClassMember>,
    ) {
        if self.at_keyword(Keyword::Use) {
            out.push(self.parse_use_trait_member(start, attributes));
            return;
        }
        let written = self.parse_written_modifiers();
        let modifiers_span = written
            .first()
            .zip(written.last())
            .map(|(first, last)| first.span.to(last.span));
        if self.at_keyword(Keyword::Const) {
            let modifiers = self.check_modifiers(ModifierSite::Constant, &written);
            let consts = self.parse_const_body(&attributes, &modifiers);
            let span = start.to(self.last_span);
            for c in consts {
                out.push(ClassMember {
                    span,
                    doc: None,
                    kind: ClassMemberKind::Const(c),
                });
            }
            return;
        }
        if self.at_keyword(Keyword::Function) {
            let modifiers = self.check_modifiers(ModifierSite::Method, &written);
            let mut member = self.parse_method_member(start, attributes, modifiers);
            if let ClassMemberKind::Method(m) = &mut member.kind {
                self.refuse_static_constructor(m, &written);
            }
            out.push(member);
            return;
        }
        // Asked before `can_start_type`, which reads the contextual `type` as a
        // property's class-named type. Neither a modifier nor an attribute group
        // is part of an alias declaration, so one written in front of the
        // keyword is refused rather than parsed and dropped.
        if self.at_type_alias() {
            self.refuse_decoration_on_a_type_alias(&attributes, modifiers_span);
            let alias = self.parse_type_alias_body(start, None);
            out.push(ClassMember {
                span: start.to(self.last_span),
                doc: None,
                kind: ClassMemberKind::TypeAlias(alias),
            });
            return;
        }
        // `rule:enums/class-scoped-enum`: the body owns the enum, and its cases
        // leave the class in every value, so a modifier is refused and the
        // declaration kept. An attribute group stays on it, as on a
        // file-scope enum.
        if self.at_keyword(Keyword::Enum) {
            if let Some(span) = modifiers_span {
                self.refuse_modifiers_on_a_member_enum(span);
            }
            let decl = self.parse_enum_decl_body(start, attributes, None);
            out.push(ClassMember {
                span: start.to(self.last_span),
                doc: None,
                kind: ClassMemberKind::Enum(decl),
            });
            return;
        }
        // A class or interface stays a file-scope declaration
        // (`rule:classes/a-type-is-declared-at-file-scope`). It is parsed whole so
        // the refusal is one diagnostic, and nothing of it is kept.
        if self.at_keyword(Keyword::Class) || self.at_keyword(Keyword::Interface) {
            let decl = if self.at_keyword(Keyword::Class) {
                self.finish_class_decl(start, attributes)
            } else {
                self.finish_interface_decl(start, attributes)
            };
            self.diags.report(
                Diagnostic::error(
                    code::E_NESTED_TYPE_DECLARATION_UNSUPPORTED,
                    "a class or an interface is not declared in a class body",
                )
                .with_primary(decl.span, "declared inside another type")
                .with_help(
                    "move it to file scope. Only an `enum` or a `type` alias can be declared \
                     in a class body",
                ),
            );
            out.push(ClassMember {
                span: decl.span,
                doc: None,
                kind: ClassMemberKind::Error,
            });
            return;
        }
        if self.can_start_type() {
            let modifiers = self.check_modifiers(ModifierSite::Property, &written);
            let covariant = self.eat_covariant();
            self.parse_property_members(start, attributes, modifiers, covariant, out);
            return;
        }
        let span = self.error_expected("a class member");
        out.push(ClassMember {
            kind: ClassMemberKind::Error,
            span,
            doc: None,
        });
    }

    /// PHP's `var $x;` property form, which
    /// `rule:core-api/legacy-property-shapes-name-the-visibility` answers with the same `E_MISSING_VISIBILITY` a bare `int $x;`
    /// gets. It needs its own arm because `var` is
    /// `rule:types/var-inference`'s
    /// local-inference keyword and starts no type, so without this the
    /// declaration falls through to `expected a class member` — a message
    /// about the grammar, aimed at an author who wrote the one shape the
    /// diagnostic exists to redirect.
    ///
    /// Recovery consumes the rest of the declaration, so the body's loop
    /// resumes at the next member rather than re-reading `$x` as one.
    fn parse_class_body_var(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> ClassMember {
        let _ = attributes;
        let kw = self.peek().span;
        self.bump(); // 'var'
        while !matches!(
            self.peek().kind,
            TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof
        ) {
            self.bump();
        }
        let _ = self.eat(TokenKind::Semicolon);
        self.diags.report(
            Diagnostic::error(
                code::E_MISSING_VISIBILITY,
                "`var` is not a visibility, and a property declares its type",
            )
            .with_primary(kw, "write `public int $x;`")
            .with_fix(kw, "public int", "write a visibility and a type"),
        );
        ClassMember {
            span: start.to(self.last_span),
            doc: None,
            kind: ClassMemberKind::Error,
        }
    }

    /// `const (Type)? Name = expr (',' Name = expr)*;` — `const` and the
    /// trailing `;` are both consumed here, so this is the whole
    /// declaration regardless of whether it ends up wrapped as a class
    /// member or (with empty `modifiers`) rejected as a top-level `const`.
    pub(super) fn parse_const_body(
        &mut self,
        attributes: &[AttributeGroup],
        modifiers: &[Modifier],
    ) -> Vec<ConstMember> {
        let kw = self.bump().span; // 'const'
        let ty = if self.can_start_type() && !self.at_const_name_without_type() {
            Some(self.parse_type())
        } else {
            None
        };
        // A constant declares its type like every other binding. Only a
        // member declaration is reported here: a top-level `const` is already
        // refused whole (`E0216`) and one with no visibility is already
        // `E0122`, and a second code on the same line would name a third edit
        // for what is one rewrite.
        if ty.is_none() && !modifiers.is_empty() {
            self.diags.report(
                Diagnostic::error(
                    code::E_CONSTANT_WITHOUT_TYPE,
                    "a constant declares its type",
                )
                .with_primary(kw, "no type between `const` and the name")
                .with_help(
                    "write the type the value has — `public const int LIMIT = 9;`, \
                     `public const string NAME = \"limits\";`",
                ),
            );
        }
        let mut members = Vec::new();
        loop {
            let name = self.parse_decl_name("a constant name").span;
            self.expect(TokenKind::Equals, "`=`");
            let value = self.parse_expr();
            members.push(ConstMember {
                attributes: attributes.to_vec(),
                modifiers: modifiers.to_vec(),
                ty: ty.clone(),
                name,
                value,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::Semicolon, "`;`");
        members
    }

    /// Whether `const` is immediately followed by `Name '='` — PHP 8.3's
    /// untyped spelling — rather than a type. A bare `Ident` here is
    /// ambiguous with a class-name type atom; the deciding token is
    /// whether `=` follows it directly.
    pub(super) fn at_const_name_without_type(&mut self) -> bool {
        matches!(self.peek().kind, TokenKind::Ident) && self.peek_at(1).kind == TokenKind::Equals
    }

    /// `E_BAD_MODIFIER` for `static` on the method named `constructor`. `new`
    /// runs the constructor on the object it has just made, so a `static` one
    /// would run with no `$this` and could assign no property. The modifier is
    /// left out of the method, as [`Self::check_modifiers`] leaves out every
    /// modifier it reports.
    fn refuse_static_constructor(&mut self, m: &mut MethodMember, written: &[WrittenModifier]) {
        if self.file.span_text(m.name) != Some("constructor")
            || !m.modifiers.contains(&Modifier::Static)
        {
            return;
        }
        let Some(at) = written.iter().find(|w| w.modifier == Modifier::Static) else {
            return;
        };
        self.diags.report(
            Diagnostic::error(code::E_BAD_MODIFIER, "`static` is not allowed on a constructor")
                .with_primary(at.span, "delete this modifier")
                .with_help(
                    "`new` calls the constructor on the new object, so the constructor needs                      `$this`. To make objects in a static method, give that method another name                      and return `new self(...)` from it",
                ),
        );
        m.modifiers.retain(|&k| k != Modifier::Static);
    }

    pub(super) fn parse_method_member(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
        modifiers: Vec<Modifier>,
    ) -> ClassMember {
        let method = self.parse_method_body(attributes, modifiers);
        let span = start.to(self.last_span);
        ClassMember {
            span,
            doc: None,
            kind: ClassMemberKind::Method(method),
        }
    }

    /// `function '&'? name(params) (: ReturnType)? (block | ';')` —
    /// `function` itself consumed here, exactly like
    /// [`Self::parse_const_body`] consumes `const`. The `&` is PHP's
    /// by-reference *return*, which `rule:statements/ampersand-is-not-a-by-reference-marker` retires with no replacement
    /// — `inout` is a parameter mode, and a return hands back a value — so it
    /// is recognized only to be reported (E0237) and the AST keeps no
    /// variant for it.
    pub(super) fn parse_method_body(
        &mut self,
        attributes: Vec<AttributeGroup>,
        modifiers: Vec<Modifier>,
    ) -> MethodMember {
        self.bump(); // 'function'
        if let Some(amp) = self.eat(TokenKind::Amp) {
            self.report_by_reference_marker(
                amp,
                "a method returns a value, not a place — drop the `&`",
            );
        }
        let name = self.parse_decl_name("a method name").span;
        let params = self.parse_params();
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        // `rule:classes/a-constructor-return-carries-no-value` is asked
        // of the body, and the name it was declared with is the whole of what
        // decides it. A method that is not the constructor parks the flag by
        // passing `false` through the same call, so a method declared inside a
        // constructor's body does not inherit it.
        let constructor = self.ident_text(name) == "constructor";
        let body = if self.at(TokenKind::LBrace) {
            Some(self.in_callable_body(constructor, Self::parse_block))
        } else {
            self.expect(TokenKind::Semicolon, "`;`");
            None
        };
        MethodMember {
            attributes,
            modifiers,
            name,
            params,
            return_type,
            body,
        }
    }

    /// `Type '$'name (',' '$'name)* ';'`, or the single-declarator hooked
    /// form `Type '$'name '{' hooks '}'` (PHP 8.4 property hooks, feeding
    /// `PropertyObserver` — `rule:classes/property-observer`). A hooked property is never part of a
    /// comma list — real PHP requires it declared alone — so the hooked
    /// branch returns as soon as it is taken.
    pub(super) fn parse_property_members(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
        modifiers: Vec<Modifier>,
        covariant: bool,
        out: &mut Vec<ClassMember>,
    ) {
        let ty = self.parse_type();
        loop {
            let name = self.expect(TokenKind::Variable, "a property name");
            if self.at(TokenKind::LBrace) {
                let hooks = self.parse_property_hooks();
                // A hook runs when a property of an object is read or written,
                // and a `static` property is never reached through an object
                // (`rule:classes/property-hooks`).
                if modifiers.contains(&Modifier::Static)
                    && let Some(hook) = hooks.first()
                {
                    self.diags.report(
                        Diagnostic::error(
                            code::E_STATIC_PROPERTY_WITH_HOOK,
                            "a `static` property cannot have a `get` or `set` hook",
                        )
                        .with_primary(hook.span, "this hook would never run")
                        .with_help("remove `static`, or remove the hooks"),
                    );
                }
                // A hook runs code on every read or write, so a `readonly`
                // property with one is no longer one value written once.
                if modifiers.contains(&Modifier::Readonly)
                    && let Some(hook) = hooks.first()
                {
                    self.diags.report(
                        Diagnostic::error(
                            code::E_READONLY_PROPERTY_WITH_HOOK,
                            "a `readonly` property cannot have a `get` or `set` hook",
                        )
                        .with_primary(hook.span, "this hook runs on every read or write")
                        .with_help(
                            "remove `readonly`, or remove the hooks and compute the value in \
                             the constructor",
                        ),
                    );
                }
                let span = start.to(self.last_span);
                out.push(ClassMember {
                    span,
                    doc: None,
                    kind: ClassMemberKind::Property(PropertyMember {
                        attributes,
                        modifiers,
                        ty,
                        covariant,
                        name,
                        default: None,
                        hooks: Some(hooks),
                    }),
                });
                return;
            }
            let default = self.eat(TokenKind::Equals).map(|_| self.parse_expr());
            // `rule:classes/a-readonly-property-declares-no-default`:
            // `readonly` is one assignment, during construction, in the
            // declaring class's own constructor, so a declaration-site default
            // *is* that assignment and the property is a per-instance constant
            // — which `const` already spells. Refused at the value rather than
            // at the modifier, because that is what a fix removes or moves.
            if let Some(value) = default
                .as_ref()
                .filter(|_| modifiers.contains(&Modifier::Readonly))
            {
                self.diags.report(
                    Diagnostic::error(
                        code::E_READONLY_PROPERTY_WITH_DEFAULT,
                        "a `readonly` property declares no default",
                    )
                    .with_primary(value.span, "this would be the property's one assignment")
                    .with_help(
                        "assign it in the constructor, write `const` for a value known at \
                         the declaration, or drop `readonly`",
                    ),
                );
            }
            let span = start.to(self.last_span);
            out.push(ClassMember {
                span,
                doc: None,
                kind: ClassMemberKind::Property(PropertyMember {
                    attributes: attributes.clone(),
                    modifiers: modifiers.clone(),
                    ty: ty.clone(),
                    covariant,
                    name,
                    default,
                    hooks: None,
                }),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::Semicolon, "`;`");
    }

    /// `{ hook+ }` — PHP 8.4's property-hook block, kept exactly as PHP has
    /// it (`rule:classes/property-hooks`: this rule "adds no new syntax beyond an ordinary
    /// interface declaration"). Mirrors [`Self::parse_block`]'s
    /// force-progress guard.
    pub(super) fn parse_property_hooks(&mut self) -> Vec<PropertyHook> {
        self.expect(TokenKind::LBrace, "`{`");
        let mut hooks = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            let hook = self.parse_property_hook();
            // A property has one `get` hook and one `set` hook at most. The
            // second of a kind is reported and left out, so the first runs.
            if let Some(first) = hooks.iter().find(|h: &&PropertyHook| h.kind == hook.kind) {
                let kind = match hook.kind {
                    PropertyHookKind::Get => "get",
                    PropertyHookKind::Set => "set",
                };
                self.diags.report(
                    Diagnostic::error(
                        code::E_DUPLICATE_DECLARATION,
                        format!("this property declares the `{kind}` hook twice"),
                    )
                    .with_primary(hook.span, "declared again here")
                    .with_secondary(first.span, "first declared here")
                    .with_help(format!(
                        "a property has one `{kind}` hook. Delete one of the two hooks"
                    )),
                );
            } else {
                hooks.push(hook);
            }
            if self.peek().span == before && !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        hooks
    }

    pub(super) fn parse_property_hook(&mut self) -> PropertyHook {
        let start = self.peek().span;
        let attributes = self.parse_attribute_groups();
        if let Some(amp) = self.eat(TokenKind::Amp) {
            self.report_by_reference_marker(
                amp,
                "a hook returns a value, not a place — drop the `&`",
            );
        }
        let kind = if self.at_contextual("set") {
            self.bump();
            PropertyHookKind::Set
        } else {
            if self.at_contextual("get") {
                self.bump();
            } else {
                self.error_expected("`get` or `set`");
            }
            PropertyHookKind::Get
        };
        let param = if kind == PropertyHookKind::Set && self.at(TokenKind::LParen) {
            Some(self.parse_hook_param())
        } else {
            None
        };
        let body = if self.eat(TokenKind::FatArrow).is_some() {
            let e = self.parse_expr();
            self.expect(TokenKind::Semicolon, "`;`");
            Some(PropertyHookBody::Expr(Box::new(e)))
        } else if self.at(TokenKind::LBrace) {
            Some(PropertyHookBody::Block(
                self.in_callable_body(false, Self::parse_block),
            ))
        } else {
            self.expect(TokenKind::Semicolon, "`;`");
            None
        };
        let span = start.to(self.last_span);
        PropertyHook {
            span,
            attributes,
            kind,
            param,
            body,
        }
    }

    /// `'(' Type? '$'name ')'` — a `set` hook's parameter. Unlike an
    /// ordinary [`Self::parse_param`], the type may be omitted with no
    /// diagnostic: PHP 8.4 infers it from the property's own declared type.
    pub(super) fn parse_hook_param(&mut self) -> Param {
        let start = self.expect(TokenKind::LParen, "`(`");
        let attributes = self.parse_attribute_groups();
        let ty = if self.can_start_type() {
            Some(self.parse_type())
        } else {
            None
        };
        let name = self.expect(TokenKind::Variable, "the hook's parameter name");
        let close = self.expect(TokenKind::RParen, "`)`");
        Param {
            span: start.to(close),
            attributes,
            modifiers: Vec::new(),
            ty,
            inout: false,
            covariant: false,
            variadic: false,
            name,
            default: None,
        }
    }

    /// `use TraitName, ...; (';' | '{' ... '}')` inside a class body —
    /// rejected outright (`rule:classes/no-traits`), same shape as
    /// [`Self::finish_trait_decl`]: there is no `UseTraitMember` AST node
    /// left to build. The trait names and, if written, the whole `{ ... }`
    /// adaptation block (which is where an `insteadof`/`as` clause could
    /// ever appear) are consumed via [`Self::skip_balanced_braces`] without
    /// being interpreted, then this reports
    /// [`Self::report_trait_not_supported`] and produces a plain
    /// [`ClassMemberKind::Error`]. Nothing in PHP's grammar, or any rule,
    /// gives this an attribute position, so `attributes` (parsed uniformly
    /// by the caller before dispatching on `use`) is simply unused here.
    pub(super) fn parse_use_trait_member(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> ClassMember {
        let _ = attributes;
        self.bump(); // 'use'
        self.parse_name_list();
        if self.at(TokenKind::LBrace) {
            self.skip_balanced_braces();
        } else {
            self.expect(TokenKind::Semicolon, "`;`");
        }
        let span = start.to(self.last_span);
        self.report_trait_not_supported(span);
        ClassMember {
            span,
            doc: None,
            kind: ClassMemberKind::Error,
        }
    }

    /// Consumes a `{ ... }` block without interpreting its contents, tracking
    /// nested braces so a well-formed skip still lands past the matching
    /// close — used only where `rule:classes/no-traits` has removed a construct's grammar
    /// (a trait `use` block's `insteadof`/`as` adaptations) but a bare
    /// "consume tokens until this closes" is still needed to keep the parser
    /// from desynchronizing.
    pub(super) fn skip_balanced_braces(&mut self) {
        self.expect(TokenKind::LBrace, "`{`");
        let mut depth = 1usize;
        while depth > 0 && !self.at(TokenKind::Eof) {
            match self.peek().kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth -= 1,
                _ => {}
            }
            self.bump();
        }
    }

    // ========================================================================
    // Enums (`rule:enums/closed-integer-type`)
    // ========================================================================

    pub(super) fn parse_enum_decl(&mut self, start: Span) -> Stmt {
        self.finish_enum_decl(start, Vec::new())
    }

    pub(super) fn finish_enum_decl(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> Stmt {
        let doc = self.take_doc_comment(start);
        let decl = self.parse_enum_decl_body(start, attributes, doc);
        Stmt {
            span: decl.span,
            kind: StmtKind::EnumDecl(decl),
        }
    }

    /// The declaration from `enum` through its closing brace, shared by the
    /// file-scope form and the class-body member. `doc` is the run the caller
    /// already took: a member's hangs on its [`ClassMember`], so that site
    /// passes `None`.
    fn parse_enum_decl_body(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
        doc: Option<DocComment>,
    ) -> EnumDecl {
        self.bump(); // 'enum'
        let name = self.parse_decl_name("an enum name");
        let backing = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_enum_backing_type())
        } else {
            None
        };
        let implements = if self.eat_keyword(Keyword::Implements).is_some() {
            self.parse_name_list()
        } else {
            Vec::new()
        };
        if let (Some(first), Some(last)) = (implements.first(), implements.last()) {
            let span = first.span.to(last.span);
            self.diags.report(
                Diagnostic::error(
                    code::E_ENUM_IMPLEMENTS_UNSUPPORTED,
                    "an enum cannot implement an interface",
                )
                .with_primary(
                    span,
                    "an enum declares only cases and an optional backing type",
                )
                .with_help("write a `static` method on another class that takes the enum"),
            );
        }
        let (cases, members) = self.parse_enum_body();
        EnumDecl {
            span: start.to(self.last_span),
            doc,
            attributes,
            name,
            backing,
            implements,
            cases,
            members,
        }
    }

    /// The `: Type` backing-type clause, parsed with the full `rule:types/grammar`
    /// grammar — only the one rejection `rule:enums/no-class-machinery` names explicitly
    /// (`string`) is checked here; that the result is otherwise exactly
    /// `int` or `uint` is a later check, not the parser's.
    pub(super) fn parse_enum_backing_type(&mut self) -> Type {
        let ty = self.parse_type();
        if matches!(ty.kind, TypeKind::Atom(TypeAtom::String)) {
            self.diags.report(
                Diagnostic::error(
                    code::E_ENUM_STRING_BACKING_UNSUPPORTED,
                    "an enum cannot be backed by `string`",
                )
                .with_primary(ty.span, "only `int`/`uint` back an enum")
                .with_help("use `: int` or `: uint`, or omit the backing type"),
            );
        }
        ty
    }

    /// An enum body mixes cases (bare names) with the `type` aliases the enum
    /// owns and, if the input is malformed, member-shaped constructs that
    /// `rule:enums/no-class-machinery` rejects outright — a method, a property,
    /// a constant, a trait `use`. All three are parsed, since attributes may
    /// precede any of them and only the token after them tells them apart;
    /// mirrors [`Self::parse_block`]'s force-progress guard.
    pub(super) fn parse_enum_body(&mut self) -> (Vec<EnumCase>, Vec<ClassMember>) {
        self.expect(TokenKind::LBrace, "`{`");
        let mut cases = Vec::new();
        let mut members = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            // A case takes its `///` run like any other declaration, and so
            // does an alias the enum owns. The rejected-member arm below does
            // not: what it parsed is refused outright, so there is no node for
            // a run to hang on — and taking it here is already what keeps it
            // from being reported as documenting nothing.
            let doc = self.take_doc_comment(before);
            let attributes = self.parse_attribute_groups();
            if matches!(self.peek().kind, TokenKind::Keyword(Keyword::Case)) {
                // PHP's `case Hearts = 1;`. The case itself is fine — only its
                // spelling is wrong — so the keyword and the trailing `;` are
                // consumed and the case is kept, which is what keeps this to
                // one diagnostic instead of the `E0220` cascade the shape used
                // to produce. Same discipline as `rule:iteration/for-init-refusals`'s `for` header:
                // refuse the spelling, keep the declaration.
                let keyword = self.peek().span;
                self.bump();
                self.diags.report(
                    Diagnostic::error(
                        code::E_PHP_ENUM_CASE_UNSUPPORTED,
                        "an enum case is not written with `case`",
                    )
                    .with_primary(keyword, "remove this keyword")
                    .with_help(
                        "an enum body is a list of cases like `Name = 1,`, separated by commas. \
                         It has no `case` keyword and no `;`",
                    ),
                );
                cases.push(self.finish_enum_case(before, doc, attributes));
                if self.eat(TokenKind::Semicolon).is_none() {
                    self.eat(TokenKind::Comma);
                }
            } else if self.at_type_alias() {
                // Ahead of the case arm, which would otherwise read the
                // contextual `type` as the case's own name.
                self.refuse_decoration_on_a_type_alias(&attributes, None);
                let alias = self.parse_type_alias_body(before, None);
                members.push(ClassMember {
                    span: alias.span,
                    doc,
                    kind: ClassMemberKind::TypeAlias(alias),
                });
            } else if matches!(self.peek().kind, TokenKind::Ident)
                || (matches!(self.peek().kind, TokenKind::Keyword(_))
                    && matches!(
                        self.peek_at(1).kind,
                        TokenKind::Comma | TokenKind::Equals | TokenKind::RBrace
                    ))
            {
                // A keyword spelling is a case when only a case could follow it:
                // `default,` is one and `public function` is not. A mis-cased
                // `default` then gets the casing check's one `must be PascalCase`
                // rather than a member refusal aimed at the line above it.
                cases.push(self.finish_enum_case(before, doc, attributes));
                self.eat(TokenKind::Comma);
            } else {
                let first = members.len();
                self.parse_class_member_with_attrs(before, attributes, &mut members);
                // A modifier run in front of an enum's own `type` reaches here,
                // because only the token after the run tells a member from a
                // case. The alias is a member the enum is allowed to hold and
                // `E0133` has already refused the run, so the refusal below
                // would be a second diagnostic about the same declaration.
                let alias_only = members.len() > first
                    && members[first..]
                        .iter()
                        .all(|m| matches!(m.kind, ClassMemberKind::TypeAlias(_)));
                let an_enum = members.len() > first
                    && matches!(members[first].kind, ClassMemberKind::Enum(_));
                if an_enum {
                    // `rule:enums/class-scoped-enum`: a case list does not own
                    // another one. The enum is refused and not kept, so nothing
                    // after the parser meets an enum whose owner is an enum.
                    members.truncate(first);
                    let span = before.to(self.last_span);
                    self.diags.report(
                        Diagnostic::error(
                            code::E_ENUM_MEMBER_UNSUPPORTED,
                            "an enum is not declared in an enum body",
                        )
                        .with_primary(span, "not a case")
                        .with_help("declare it in a class or interface body, or at file scope"),
                    );
                } else if !alias_only {
                    let span = before.to(self.last_span);
                    self.diags.report(
                        Diagnostic::error(
                            code::E_ENUM_MEMBER_UNSUPPORTED,
                            "an enum declares only cases and an optional backing type",
                        )
                        .with_primary(span, "not a case")
                        .with_help("move this to a separate class"),
                    );
                }
            }
            if self.peek().span == before && !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        (cases, members)
    }

    pub(super) fn finish_enum_case(
        &mut self,
        start: Span,
        doc: Option<DocComment>,
        attributes: Vec<AttributeGroup>,
    ) -> EnumCase {
        let name = self.parse_decl_name("a case name");
        let value = self.eat(TokenKind::Equals).map(|_| self.parse_expr());
        let span = start.to(self.last_span);
        EnumCase {
            span,
            doc,
            attributes,
            name,
            value,
        }
    }

    // ========================================================================
    // The statement-shaped rejects: a top-level `function`/`const`
    // (`rule:classes/no-free-functions-or-constants`)
    // ========================================================================

    /// `#[...]` groups precede a class/interface/trait/enum declaration, a
    /// rejected top-level `function`/`const`, or nothing this parser
    /// recognizes yet — decided by the keyword that follows them.
    pub(super) fn parse_attributed_decl_stmt(&mut self, start: Span) -> Stmt {
        let attributes = self.parse_attribute_groups();
        match self.peek().kind {
            TokenKind::Keyword(Keyword::Abstract | Keyword::Final | Keyword::Class) => {
                self.finish_class_decl(start, attributes)
            }
            TokenKind::Keyword(Keyword::Interface) => self.finish_interface_decl(start, attributes),
            TokenKind::Keyword(Keyword::Trait) => self.finish_trait_decl(start, attributes),
            TokenKind::Keyword(Keyword::Enum) => self.finish_enum_decl(start, attributes),
            TokenKind::Keyword(Keyword::Const) => {
                self.parse_toplevel_const_reject(start, attributes)
            }
            TokenKind::Keyword(Keyword::Function) if self.at_named_function_decl() => {
                self.parse_toplevel_function_reject(start, attributes)
            }
            _ => {
                self.error_expected("a declaration after `#[...]`");
                self.parse_statement()
            }
        }
    }

    /// Whether `function` at the current position starts a rejected
    /// top-level declaration (`function foo() { ... }`) rather than an
    /// anonymous `function` used as a bare expression statement
    /// (`function () { ... };`) — decided by whether a name, not `(`,
    /// follows, skipping an optional by-reference `&`.
    pub(super) fn at_named_function_decl(&mut self) -> bool {
        let idx = if self.peek_at(1).kind == TokenKind::Amp {
            2
        } else {
            1
        };
        Self::is_name_segment(self.peek_at(idx).kind)
    }

    pub(super) fn parse_toplevel_function_reject(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> Stmt {
        let method = self.parse_method_body(attributes, Vec::new());
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(
                code::E_TOPLEVEL_FUNCTION_UNSUPPORTED,
                "a function must be a method",
            )
            .with_primary(span, "not inside any class")
            .with_help("move it into a class as a `public static function`"),
        );
        Stmt {
            span,
            kind: StmtKind::TopLevelFunction(method),
        }
    }

    pub(super) fn parse_toplevel_const_reject(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> Stmt {
        let consts = self.parse_const_body(&attributes, &[]);
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(
                code::E_TOPLEVEL_CONST_UNSUPPORTED,
                "a constant must belong to a class",
            )
            .with_primary(span, "not inside any class")
            .with_help("declare it as a `public const` in the class it belongs to"),
        );
        Stmt {
            span,
            kind: StmtKind::TopLevelConst(consts),
        }
    }
}
