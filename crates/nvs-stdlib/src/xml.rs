//! `Core\Xml` — spec § 17's one API for documents, and the node family
//! `rule:core-classes/html-parsing` makes both parsers produce.
//!
//! # One node family, and it is closed at five
//!
//! [`Kind`] is element, text, comment, processing instruction and document, and
//! there is no sixth. `rule:core-classes/html-parsing` decided that the WHATWG
//! parser on `Core\Html` and the strict parser here materialise **the same
//! nodes**, so queries, traversal and the memory story are written once and
//! which door parsed a document does not change what a program can do with it.
//! That is the whole reason the two land in one goal rather than two, and it is
//! why the family is declared here — in the module the tree belongs to — rather
//! than beside either parser.
//!
//! What the family deliberately lacks is the rest of the DOM's inventory:
//! there is no attribute node, no CDATA node, no entity node, no notation node
//! and no document-fragment node. An attribute is a pair on the element that
//! carries it ([`nvs_core_xml_node_attributes`]); a CDATA section is text,
//! because the difference is a spelling of the same characters and a program
//! that could tell them apart would be reading the document's punctuation
//! rather than its content; the other three are the DTD's, and this parser
//! resolves no DTD at all.
//!
//! # The tree materialises and the stream does not
//!
//! Spec § 17 states the split outright, as "the one place in this file where
//! two shapes of the same subsystem coexist", so it is not read as an exception
//! to `rule:core-api/one-paradigm-per-operation`. [`nvs_core_xml_parse`] is the
//! tree half: it reads the whole document and answers a value a program can
//! walk in any order and any number of times. The streaming reader and writer
//! that replace `XMLReader` and `XMLWriter` hold one window instead, and **no
//! operation is available through both** — a program picks the shape that fits
//! how much of the document it needs at once, and pays for that shape only.
//! This paragraph is the split's one statement; no member card re-argues it.
//!
//! # What a parse refuses, by construction
//!
//! There is **no code in this module that resolves an entity**, and that is a
//! stronger statement than a flag defaulting to off. The five predefined
//! entities and numeric character references expand, because they are the
//! document's own characters written another way; every other reference is a
//! `ParseError` naming what was written. A document type declaration is refused
//! whole, so an external subset is never a thing that could have been fetched
//! and an internal subset defines nothing that could be expanded. `Core\Http\Client`
//! is how a program fetches something, and it parses the bytes it got back.
//!
//! The other bound is depth: [`DEPTH_CEILING`] caps how deeply elements may
//! nest, so a document engineered to be deep is refused rather than run at
//! whatever the native stack happens to be. `Core\Json`'s ceiling is the same
//! number for the same reason, and the parse loop here is iterative regardless.
//!
//! # Everything read out of a parsed tree is `tainted`
//!
//! A name, a text, a comment's content and every attribute value answer the
//! `tainted` form, whatever the argument to [`nvs_core_xml_parse`] was — the
//! ordinary rule that what a parser over untrusted bytes produces is untrusted,
//! and the same shape `Core\Jwt::verify` writes. So the parse parameter is
//! [`Qual::Neutral`] rather than [`Qual::Contagious`]: the answer's mark does
//! not depend on the argument's, because it is unconditional.
//!
//! # What this spends
//!
//! Per parse, per `rule:programs/memory-priority`: the materialised tree, which
//! is proportional to the document, attributed to the request that parsed it
//! and released with its arena. The parse itself holds the document once more
//! while it builds — a Rust tree of owned strings, dropped before the member
//! returns.
//!
//! # Known gaps
//!
//! 1. **The stream is not here.** The reader and writer the section above
//!    describes are unwritten, and until they are, a program that outgrows the
//!    tree has nowhere to go. The family they answer with is [`Kind`] and no
//!    second vocabulary.
//! 2. **Nothing serialises.** A tree can be walked and not written back out, so
//!    `Core\Html::sanitize`'s parse-walk-serialise round trip has two of its
//!    three steps.
//! 3. **A name is the name as written, prefix and all.** `<x:a/>` answers
//!    `x:a`, and no `xmlns` declaration is resolved to a namespace URI. A
//!    program comparing qualified names is comparing the document's own
//!    spelling, which is right for a document it controls and not enough for
//!    one it does not.
//! 4. **Whitespace between elements is text.** A pretty-printed document has a
//!    text node between every pair of siblings, exactly as the XML it is says
//!    it does. Dropping them would be a guess about which whitespace mattered,
//!    which `rule:errors/ambiguous-input-refused` is the general answer to.

use nvs_runtime::{Fault, NvsArray, NvsStr, ThrownClass, Value};

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, its node family, and where its symbols live
// ============================================================================

/// `Core\Xml`'s fully-qualified name, written once so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Xml";

/// `Core\Xml\Node`'s fully-qualified name, for [`NODE`] and for the return
/// position that carries one.
pub(crate) const NODE_NAME: &str = r"Core\Xml\Node";

/// `Core\Xml\NodeKind`'s fully-qualified name, for [`KIND`] and for the return
/// position that carries one.
pub(crate) const KIND_NAME: &str = r"Core\Xml\NodeKind";

/// `Core\Xml`'s registry rows — the tree half of § 17, which is the parse and
/// nothing else. The streaming half is this module's known gap 1.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[CoreMethod {
        name: "parse",
        names: &["document"],
        params: &[CoreTy::Text(Qual::Neutral)],
        defaults: &[],
        return_ty: CoreTy::Instance(NODE_NAME),
        symbol: "nvs_core_xml_parse",
        doc: Some(&PARSE_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Xml::parse`'s reference card — `rule:core-api/reference-card`.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads a whole XML document and answers its document node — replacing `DOMDocument::load`, \
            `simplexml_load_string` and `xml_parse`, none of which agree about what malformed input \
            means. This one refuses it: a document that is not well-formed throws, and nothing is \
            repaired, recovered or guessed. `Core\\Html::parse` is the opposite contract on the same \
            node family, because the WHATWG algorithm has no failure mode.",
    params: &[ParamDoc {
        name: "document",
        desc: "The document text. No entity is resolved from anywhere: the five predefined entities \
               and numeric character references expand, and every other reference is refused.",
        shape: &[],
    }],
    ret: "The document node, whose children are the root element and any comments or processing \
          instructions written beside it. Every string reachable through it is `tainted`, whatever \
          this argument was.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "The document is not well-formed — a tag that never closes or closes as something \
               else, more than one root element, an attribute written twice, a reference this \
               parser will not resolve, a document type declaration, or elements nested deeper \
               than the ceiling.",
    }],
};

/// [`NODE`]'s slots, in declaration order.
const KIND_SLOT: usize = 0;
/// See [`KIND_SLOT`].
const NAME_SLOT: usize = 1;
/// See [`KIND_SLOT`].
const TEXT_SLOT: usize = 2;
/// See [`KIND_SLOT`].
const ATTRIBUTES_SLOT: usize = 3;
/// See [`KIND_SLOT`].
const CHILDREN_SLOT: usize = 4;

/// `rule:core-classes/html-parsing`'s one node family, as the value a program
/// holds — five members over five slots and no static member at all, because a
/// node is only ever produced by a parse.
///
/// Every member is a slot read: the document has been read by the time a node
/// exists, so there is nothing left to compute and nothing left to fail. Which
/// slots carry anything depends on the node's [`Kind`], and each member's card
/// says which — a text node has no attributes and an element carries no text of
/// its own, both of which are answers rather than errors.
pub(crate) const NODE: CoreClass = CoreClass {
    name: NODE_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "kind",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(KIND_NAME),
            symbol: "nvs_core_xml_node_kind",
            doc: Some(&KIND_DOC),
        },
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_xml_node_name",
            doc: Some(&NAME_DOC),
        },
        CoreMethod {
            name: "text",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_xml_node_text",
            doc: Some(&TEXT_DOC),
        },
        CoreMethod {
            name: "attributes",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::TaintedStr),
            symbol: "nvs_core_xml_node_attributes",
            doc: Some(&ATTRIBUTES_DOC),
        },
        CoreMethod {
            name: "children",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(NODE_NAME)),
            symbol: "nvs_core_xml_node_children",
            doc: Some(&CHILDREN_DOC),
        },
    ],
    slots: &["kind", "name", "text", "attributes", "children"],
    constants: &[],
};

/// `Core\Xml\Node::kind`'s reference card — `rule:core-api/reference-card`.
const KIND_DOC: MethodDoc = MethodDoc {
    short: "Which of the five kinds of node this is — the question every walk over a tree asks \
            first, and the one a program answers with a comparison rather than with a check for \
            which other members are empty.",
    params: &[],
    ret: "A `Core\\Xml\\NodeKind` case. The set is closed, so a `match` over it is exhaustive.",
    errors: &[],
};

/// `Core\Xml\Node::name`'s reference card — `rule:core-api/reference-card`.
const NAME_DOC: MethodDoc = MethodDoc {
    short: "The name this node was written under — an element's tag name, or a processing \
            instruction's target. Empty for a text node, a comment and the document, which have \
            no name to carry rather than an unknown one.",
    params: &[],
    ret: "The name exactly as the document spelled it, prefix included: `<x:a/>` answers `x:a`, \
          because no namespace declaration is resolved. `tainted`, as everything read out of a \
          parsed tree is.",
    errors: &[],
};

/// `Core\Xml\Node::text`'s reference card — `rule:core-api/reference-card`.
const TEXT_DOC: MethodDoc = MethodDoc {
    short: "The character data this node carries itself — a text node's characters, a comment's \
            content, a processing instruction's data. Empty for an element and for the document, \
            whose characters belong to their text children: this is the node's own text and never \
            a walk over its descendants, so what it costs is a slot read.",
    params: &[],
    ret: "The characters, with the five predefined entities and any character references already \
          expanded and a CDATA section read as the text it spells. `tainted`, as everything read \
          out of a parsed tree is.",
    errors: &[],
};

/// `Core\Xml\Node::attributes`'s reference card — `rule:core-api/reference-card`.
const ATTRIBUTES_DOC: MethodDoc = MethodDoc {
    short: "This element's attributes, in the order they were written — replacing `DOMElement`'s \
            attribute nodes with the pairs they always were. Empty for every other kind of node.",
    params: &[],
    ret: "One entry per attribute, keyed by the name as written and holding the value with its \
          references expanded. `tainted`, as everything read out of a parsed tree is.",
    errors: &[],
};

/// `Core\Xml\Node::children`'s reference card — `rule:core-api/reference-card`.
const CHILDREN_DOC: MethodDoc = MethodDoc {
    short: "This node's children, in document order — the document's are its root element and \
            whatever comments and processing instructions sit beside it, an element's are its \
            content. Empty for a text node, a comment and a processing instruction, which are \
            leaves.",
    params: &[],
    ret: "One `Core\\Xml\\Node` per child, including the text nodes a pretty-printed document has \
          between its elements: whitespace in an XML document is content, and dropping it would be \
          a guess about which of it mattered.",
    errors: &[],
};

/// `rule:core-classes/html-parsing`'s node family, as the closed enum a program
/// compares against.
///
/// The integers are each case's own constant, per `CoreEnum::cases`, and they
/// are ABI: [`Kind::ordinal`] writes one into a node's [`KIND_SLOT`] and
/// `Core\Xml\Node::kind` reads it straight back out, so a case is appended and
/// never inserted. `element_text_comment_processing_instruction_and_document_are_the_whole_family`
/// holds this roster and [`Kind::ALL`] together.
pub(crate) const KIND: CoreEnum = CoreEnum {
    name: KIND_NAME,
    cases: &[
        ("Element", 0),
        ("Text", 1),
        ("Comment", 2),
        ("ProcessingInstruction", 3),
        ("Document", 4),
    ],
    doc: Some(&KIND_ENUM_DOC),
};

/// [`KIND`]'s reference card — `rule:core-api/reference-card`.
const KIND_ENUM_DOC: EnumDoc = EnumDoc {
    short: "What a node in a parsed document is. The set is closed at five and both parsers \
            produce it, so a walk written against one door works unchanged through the other.",
    cases: &[
        CaseDoc {
            name: "Element",
            desc: "A tag and its content — the only kind that carries attributes or children of \
                   more than one kind.",
        },
        CaseDoc {
            name: "Text",
            desc: "Character data, including what a CDATA section spelled: the two are the same \
                   characters written differently.",
        },
        CaseDoc {
            name: "Comment",
            desc: "A `<!-- … -->`, carried rather than dropped, because a document's comments are \
                   part of what it says.",
        },
        CaseDoc {
            name: "ProcessingInstruction",
            desc: "A `<?target data?>`, whose target is the node's name and whose data is its \
                   text. The XML declaration is not one of these.",
        },
        CaseDoc {
            name: "Document",
            desc: "The root of what a parse answers with — never a child of anything, and the one \
                   node a program is handed rather than reaching.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_xml_parse" => (nvs_core_xml_parse as *const ()).cast(),
        "nvs_core_xml_node_kind" => (nvs_core_xml_node_kind as *const ()).cast(),
        "nvs_core_xml_node_name" => (nvs_core_xml_node_name as *const ()).cast(),
        "nvs_core_xml_node_text" => (nvs_core_xml_node_text as *const ()).cast(),
        "nvs_core_xml_node_attributes" => (nvs_core_xml_node_attributes as *const ()).cast(),
        "nvs_core_xml_node_children" => (nvs_core_xml_node_children as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The node family
// ============================================================================

/// One of the five kinds of node — [`KIND`]'s cases, as the Rust half that
/// builds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A tag and its content.
    Element,
    /// Character data, however it was spelled.
    Text,
    /// A `<!-- … -->`.
    Comment,
    /// A `<?target data?>`.
    ProcessingInstruction,
    /// What a parse answers with.
    Document,
}

impl Kind {
    /// The whole family, in [`KIND`]'s own order — the roster a walk over "every
    /// kind there is" iterates, so neither half can grow a case the other lacks.
    pub(crate) const ALL: [Self; 5] = [
        Self::Element,
        Self::Text,
        Self::Comment,
        Self::ProcessingInstruction,
        Self::Document,
    ];

    /// This case's own integer, which is what a node's slot holds and what
    /// [`KIND`]'s row of the same name registers.
    pub(crate) fn ordinal(self) -> i64 {
        match self {
            Self::Element => 0,
            Self::Text => 1,
            Self::Comment => 2,
            Self::ProcessingInstruction => 3,
            Self::Document => 4,
        }
    }
}

/// The two halves of the family are the same length, checked here rather than
/// in a test because a `const` assertion cannot be forgotten to run. Which
/// case sits at which integer is
/// `element_text_comment_processing_instruction_and_document_are_the_whole_family`'s,
/// since that is a question about ordering and not about size.
const _: () = assert!(KIND.cases.len() == Kind::ALL.len());

/// One node of the tree a parse built, before it becomes a [`NODE`] instance.
///
/// A Rust tree first and a Novis one second, so the parse can be written as
/// ordinary code over owned strings and the whole of it is dropped before the
/// member returns. What each field means per [`Kind`] is on the member cards
/// above, which are what a program reads.
#[derive(Debug)]
struct Parsed {
    /// Which of the five this is.
    kind: Kind,
    /// An element's tag name or a processing instruction's target.
    name: String,
    /// The character data this node carries itself.
    text: String,
    /// An element's attributes, in written order.
    attributes: Vec<(String, String)>,
    /// This node's children, in document order.
    children: Vec<Parsed>,
}

impl Parsed {
    /// An empty node of `kind` — every field is filled by the reader that
    /// produced it, and the ones a kind does not use stay empty.
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            name: String::new(),
            text: String::new(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// A text node carrying `text` — the one node two readers build, since a
    /// CDATA section and ordinary character data are the same node.
    fn text_node(text: String) -> Self {
        let mut node = Self::new(Kind::Text);
        node.text = text;
        node
    }
}

// ============================================================================
// The parser
// ============================================================================

/// How deeply elements may nest before a document is refused.
///
/// `Core\Json::decode`'s ceiling is the same number for the same reason: a
/// document engineered to be deep costs work before any value exists, and a
/// bound that is stated is one a caller can reason about. The parse loop here
/// is iterative, so this is a policy rather than a guard over the native stack.
pub(crate) const DEPTH_CEILING: usize = 1024;

/// What a document type declaration gets, and why it is not a setting.
///
/// Written once because it is the whole of `Core\Xml`'s answer to XXE: there is
/// no branch anywhere in this module that resolves an entity, so there is
/// nothing for a flag to have defaulted to off.
const DOCTYPE_REFUSAL: &str = "a document type declaration is not read at all — no entity is \
                               resolved from an external subset, from an internal subset or from \
                               the network, so there is nothing a `<!DOCTYPE …>` could mean here";

/// Whether `ch` is XML's whitespace — the four characters, and not `char`'s
/// Unicode answer, which would accept a no-break space as tag separation.
fn is_space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\r' | '\n')
}

/// Whether `ch` may open a name.
fn is_name_start(ch: char) -> bool {
    ch == '_' || ch == ':' || ch.is_alphabetic()
}

/// Whether `ch` may continue a name.
fn is_name_char(ch: char) -> bool {
    is_name_start(ch) || ch == '-' || ch == '.' || ch.is_ascii_digit()
}

/// The character a numeric reference named, or why it is not one.
///
/// # Errors
///
/// A sentence for a reference that names no character at all, or one XML has no
/// way to write: a surrogate half, a `NUL`, or a control character other than
/// the three whitespace ones.
fn numeric(code: Option<u32>, body: &str) -> Result<char, String> {
    let refused = format!("`&{body};` names no character a document may hold");
    let Some(ch) = code.and_then(char::from_u32) else {
        return Err(refused);
    };
    if ch.is_control() && !matches!(ch, '\t' | '\r' | '\n') {
        return Err(refused);
    }
    Ok(ch)
}

/// A reader over one document's text, holding a byte cursor into it.
///
/// Every method reads forward from the cursor and every one of them either
/// advances it or reports why it stopped, so the walk cannot loop: a document
/// is refused the moment nothing matches, rather than skipped past.
#[derive(Debug)]
struct Reader<'a> {
    /// The whole document.
    src: &'a str,
    /// How far into it the walk has read, in bytes, always at a character
    /// boundary because every advance is by a character's own width.
    pos: usize,
}

impl<'a> Reader<'a> {
    /// A reader positioned at the start of `src`.
    fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    /// What has not been read yet.
    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    /// A short excerpt of what is next, for a message that has to show where
    /// the document stopped making sense.
    fn here(&self) -> String {
        self.rest().chars().take(16).collect()
    }

    /// Consumes `lead` if it is next, and reports whether it was.
    fn eat(&mut self, lead: &str) -> bool {
        if self.rest().starts_with(lead) {
            self.pos += lead.len();
            true
        } else {
            false
        }
    }

    /// Consumes any run of whitespace.
    fn skip_space(&mut self) {
        let left = self.rest().trim_start_matches(is_space);
        self.pos = self.src.len() - left.len();
    }

    /// Everything up to the next `close`, consuming the closer too.
    ///
    /// # Errors
    ///
    /// A sentence naming `what` for a document that ends first.
    fn take_until(&mut self, close: &str, what: &str) -> Result<&'a str, String> {
        let rest = self.rest();
        let end = rest
            .find(close)
            .ok_or_else(|| format!("{what} is never closed by `{close}`"))?;
        self.pos += end + close.len();
        Ok(&rest[..end])
    }

    /// One name at the cursor.
    ///
    /// # Errors
    ///
    /// A sentence naming `what` and showing what was written instead.
    fn name(&mut self, what: &str) -> Result<String, String> {
        let rest = self.rest();
        let mut end = 0;
        for (at, ch) in rest.char_indices() {
            let ok = if at == 0 {
                is_name_start(ch)
            } else {
                is_name_char(ch)
            };
            if !ok {
                break;
            }
            end = at + ch.len_utf8();
        }
        if end == 0 {
            return Err(format!(
                "{what} needs a name, and `{}` is not one",
                self.here()
            ));
        }
        self.pos += end;
        Ok(rest[..end].to_owned())
    }

    /// Expands the reference at the cursor onto `out`.
    ///
    /// **The whole of what this module resolves.** Five names and the two
    /// numeric forms, all of which are the document's own characters written
    /// another way; anything else is refused rather than looked up, because
    /// there is nowhere here to look one up.
    ///
    /// # Errors
    ///
    /// A sentence for a reference that is unclosed, that names an entity this
    /// parser will not resolve, or that is a bare `&`.
    fn reference(&mut self, out: &mut String) -> Result<(), String> {
        let rest = &self.rest()[1..];
        let end = rest.find(';').unwrap_or(rest.len());
        let body = &rest[..end];
        if end == rest.len()
            || body.is_empty()
            || body.contains(|ch: char| is_space(ch) || ch == '<')
        {
            return Err(format!(
                "`{}` opens a reference that is never closed by `;` — a literal ampersand is \
                 written `&amp;`",
                self.here()
            ));
        }
        let ch = if let Some(hex) = body.strip_prefix("#x").or_else(|| body.strip_prefix("#X")) {
            numeric(u32::from_str_radix(hex, 16).ok(), body)?
        } else if let Some(decimal) = body.strip_prefix('#') {
            numeric(decimal.parse::<u32>().ok(), body)?
        } else {
            match body {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                _ => {
                    return Err(format!(
                        "`&{body};` is not one of the five predefined entities, and no entity is \
                         resolved from a document type declaration, an internal subset or the \
                         network"
                    ));
                }
            }
        };
        out.push(ch);
        self.pos += 1 + end + 1;
        Ok(())
    }

    /// One attribute value, quotes and references consumed.
    ///
    /// # Errors
    ///
    /// A sentence naming `attr` for a value that is unquoted, never closed, or
    /// holds a literal `<`.
    fn attribute_value(&mut self, attr: &str) -> Result<String, String> {
        let quote = if self.eat("\"") {
            '"'
        } else if self.eat("'") {
            '\''
        } else {
            return Err(format!(
                "attribute `{attr}`'s value must be quoted, and `{}` is not",
                self.here()
            ));
        };
        let mut out = String::new();
        loop {
            let Some(ch) = self.rest().chars().next() else {
                return Err(format!("attribute `{attr}`'s value is never closed"));
            };
            if ch == quote {
                self.pos += ch.len_utf8();
                return Ok(out);
            }
            if ch == '<' {
                return Err(format!(
                    "attribute `{attr}`'s value holds a literal `<`, which is written `&lt;`"
                ));
            }
            if ch == '&' {
                self.reference(&mut out)?;
                continue;
            }
            out.push(ch);
            self.pos += ch.len_utf8();
        }
    }

    /// One start tag, and whether it closed itself.
    ///
    /// # Errors
    ///
    /// A sentence for a tag with no name, an attribute with no value, a value
    /// [`Self::attribute_value`] refuses, a name written twice, or a tag the
    /// document ends inside.
    fn start_tag(&mut self) -> Result<(Parsed, bool), String> {
        self.pos += '<'.len_utf8();
        let mut node = Parsed::new(Kind::Element);
        node.name = self.name("an opening tag")?;
        loop {
            let spaced = self.rest().starts_with(is_space);
            self.skip_space();
            if self.eat("/>") {
                return Ok((node, true));
            }
            if self.eat(">") {
                return Ok((node, false));
            }
            if self.rest().is_empty() {
                return Err(format!("`<{}` is never closed", node.name));
            }
            if !spaced {
                return Err(format!(
                    "`<{}` needs a space before `{}`",
                    node.name,
                    self.here()
                ));
            }
            let attr = self.name("an attribute")?;
            self.skip_space();
            if !self.eat("=") {
                return Err(format!(
                    "attribute `{attr}` on `{}` needs a value",
                    node.name
                ));
            }
            self.skip_space();
            let value = self.attribute_value(&attr)?;
            if node.attributes.iter().any(|(seen, _)| *seen == attr) {
                return Err(format!(
                    "attribute `{attr}` is written twice on `{}`",
                    node.name
                ));
            }
            node.attributes.push((attr, value));
        }
    }

    /// One comment.
    ///
    /// # Errors
    ///
    /// A sentence for a comment the document ends inside, or one holding `--`,
    /// which XML forbids because it cannot be told from the closer.
    fn comment(&mut self) -> Result<Parsed, String> {
        self.pos += "<!--".len();
        let body = self.take_until("-->", "a comment")?;
        if body.contains("--") {
            return Err("a comment holds `--`, which XML does not allow inside one".to_owned());
        }
        let mut node = Parsed::new(Kind::Comment);
        node.text = body.to_owned();
        Ok(node)
    }

    /// One processing instruction.
    ///
    /// # Errors
    ///
    /// A sentence for one the document ends inside, one with no target, or one
    /// whose target is `xml`, which is the declaration rather than an
    /// instruction and may only open the document.
    fn processing_instruction(&mut self) -> Result<Parsed, String> {
        self.pos += "<?".len();
        let target = self.name("a processing instruction")?;
        if target.eq_ignore_ascii_case("xml") {
            return Err(
                "`<?xml …?>` is the declaration, so it may only be the first thing in a document"
                    .to_owned(),
            );
        }
        let mut node = Parsed::new(Kind::ProcessingInstruction);
        node.name = target;
        if self.eat("?>") {
            return Ok(node);
        }
        if !self.rest().starts_with(is_space) {
            return Err(format!(
                "processing instruction `{}` needs a space before its data",
                node.name
            ));
        }
        self.skip_space();
        node.text = self
            .take_until("?>", "a processing instruction")?
            .to_owned();
        Ok(node)
    }

    /// One CDATA section, as the text node it is.
    ///
    /// # Errors
    ///
    /// A sentence for a section the document ends inside.
    fn cdata(&mut self) -> Result<Parsed, String> {
        self.pos += "<![CDATA[".len();
        let body = self.take_until("]]>", "a CDATA section")?;
        Ok(Parsed::text_node(body.to_owned()))
    }

    /// The character data up to the next `<`, references expanded.
    ///
    /// # Errors
    ///
    /// Whatever [`Self::reference`] refused, or a literal `]]>`, which XML
    /// forbids in content because it cannot be told from a CDATA closer.
    fn chardata(&mut self) -> Result<Parsed, String> {
        let mut out = String::new();
        loop {
            let rest = self.rest();
            if rest.starts_with("]]>") {
                return Err(
                    "`]]>` is written in character data, where it can only be a CDATA section's \
                     closer"
                        .to_owned(),
                );
            }
            let Some(ch) = rest.chars().next() else { break };
            if ch == '<' {
                break;
            }
            if ch == '&' {
                self.reference(&mut out)?;
                continue;
            }
            out.push(ch);
            self.pos += ch.len_utf8();
        }
        Ok(Parsed::text_node(out))
    }

    /// The XML declaration, if the document opens on one, consumed and
    /// discarded — it is not a node, which is why the family has no case for it.
    ///
    /// # Errors
    ///
    /// A sentence for a declaration the document ends inside.
    fn declaration(&mut self) -> Result<(), String> {
        let rest = self.rest();
        if rest.starts_with("<?xml") && rest["<?xml".len()..].starts_with(is_space) {
            self.take_until("?>", "the XML declaration")?;
        }
        Ok(())
    }

    /// One element and everything under it, iteratively.
    ///
    /// The open elements are an explicit stack rather than the native one, so
    /// [`DEPTH_CEILING`] is what bounds nesting and a document engineered to be
    /// deep is refused rather than run at whatever the thread's stack happens
    /// to be.
    ///
    /// # Errors
    ///
    /// A sentence for a tag that never closes, one that closes as something
    /// else, content the document ends inside, or nesting past the ceiling.
    fn tree(&mut self) -> Result<Parsed, String> {
        let mut open: Vec<Parsed> = Vec::new();
        loop {
            let rest = self.rest();
            let node = if rest.starts_with("</") {
                self.pos += "</".len();
                let name = self.name("a closing tag")?;
                self.skip_space();
                if !self.eat(">") {
                    return Err(format!("`</{name}` is never closed"));
                }
                let Some(node) = open.pop() else {
                    return Err(format!("`</{name}>` closes an element nothing opened"));
                };
                if node.name != name {
                    return Err(format!(
                        "`<{}>` is closed by `</{name}>`, and an element closes as what it opened \
                         as",
                        node.name
                    ));
                }
                node
            } else if rest.starts_with("<!--") {
                self.comment()?
            } else if rest.starts_with("<![CDATA[") {
                self.cdata()?
            } else if rest.starts_with("<!DOCTYPE") {
                return Err(DOCTYPE_REFUSAL.to_owned());
            } else if rest.starts_with("<!") {
                return Err(format!(
                    "`{}` is not something a document may hold",
                    self.here()
                ));
            } else if rest.starts_with("<?") {
                self.processing_instruction()?
            } else if rest.starts_with('<') {
                let (node, closed) = self.start_tag()?;
                if !closed {
                    if open.len() >= DEPTH_CEILING {
                        return Err(format!(
                            "elements nest deeper than {DEPTH_CEILING}, which is as deep as a \
                             document may be"
                        ));
                    }
                    open.push(node);
                    continue;
                }
                node
            } else if rest.is_empty() {
                let unclosed = open.last().map_or_else(
                    || "a document needs a root element".to_owned(),
                    |node| format!("`<{}>` is never closed", node.name),
                );
                return Err(unclosed);
            } else {
                self.chardata()?
            };
            match open.last_mut() {
                Some(parent) => parent.children.push(node),
                None => return Ok(node),
            }
        }
    }

    /// The whole document, prolog and trailer included.
    ///
    /// # Errors
    ///
    /// A sentence for anything that is not one well-formed document: no root
    /// element, a second one, a document type declaration, or text outside the
    /// root.
    fn document(&mut self) -> Result<Parsed, String> {
        let mut root = Parsed::new(Kind::Document);
        self.declaration()?;
        let mut rooted = false;
        loop {
            self.skip_space();
            let rest = self.rest();
            if rest.is_empty() {
                break;
            }
            if rest.starts_with("<!--") {
                let node = self.comment()?;
                root.children.push(node);
            } else if rest.starts_with("<!DOCTYPE") {
                return Err(DOCTYPE_REFUSAL.to_owned());
            } else if rest.starts_with("<?") {
                let node = self.processing_instruction()?;
                root.children.push(node);
            } else if rest.starts_with("</") {
                return Err(format!(
                    "`{}` closes an element nothing opened",
                    self.here()
                ));
            } else if rest.starts_with('<') {
                if rooted {
                    return Err(
                        "a document has one root element, and this is a second one".to_owned()
                    );
                }
                rooted = true;
                let node = self.tree()?;
                root.children.push(node);
            } else {
                return Err(format!(
                    "`{}` is character data outside the root element",
                    self.here()
                ));
            }
        }
        if !rooted {
            return Err("a document needs a root element".to_owned());
        }
        Ok(root)
    }
}

/// `document`, parsed — the one entry into the reader above.
///
/// # Errors
///
/// One sentence saying what is not well-formed about it. There is no partial
/// answer: `rule:errors/ambiguous-input-refused` is why a document that does
/// not parse produces nothing rather than as much of a tree as could be built.
fn parse(document: &str) -> Result<Parsed, String> {
    Reader::new(document).document()
}

/// One [`Parsed`] subtree as the [`NODE`] instance a program holds.
///
/// Recursive, and bounded by [`DEPTH_CEILING`] because that is what the reader
/// already refused past — so the depth here is the document's, not an
/// attacker's choice.
fn instance_of(node: Parsed) -> Value {
    let mut children = NvsArray::new();
    for child in node.children {
        children.append(instance_of(child));
    }
    let mut attributes = NvsArray::new();
    for (name, value) in node.attributes {
        attributes.set(
            NvsStr::new(name.as_bytes()),
            Value::str(NvsStr::new(value.as_bytes())),
        );
    }
    crate::instance::build(
        &NODE,
        [
            Value::int(node.kind.ordinal()),
            Value::str(NvsStr::new(node.name.as_bytes())),
            Value::str(NvsStr::new(node.text.as_bytes())),
            Value::array(attributes),
            Value::array(children),
        ],
    )
}

/// One of a receiver's slots, **retained** — the caller takes over the
/// reference this answers with, since `crate::instance::slot` borrows.
///
/// # Errors
///
/// A `Fault::fatal` if the receiver is not one of [`NODE`]'s instances, which
/// the compile-time signature rules out.
fn held(receiver: Value, index: usize, member: &str) -> Result<Value, Fault> {
    let object = crate::instance::receiver(receiver, &NODE, member)?;
    let value = crate::instance::slot(object, index);
    #[expect(
        unsafe_code,
        reason = "the receiver owns the reference this borrowed read returned, \
                  so the caller needs one of its own"
    )]
    unsafe {
        value.retain();
    }
    Ok(value)
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Xml::parse(string $document): Core\Xml\Node` — replacing
    /// `DOMDocument::load`, `simplexml_load_string` and `xml_parse`.
    ///
    /// The refusal is the point: malformed XML throws here, where the WHATWG
    /// parser on `Core\Html` cannot, and one API flipping between the two under
    /// a flag is what `rule:core-classes/html-parsing` retired.
    fn nvs_core_xml_parse(_ctx, args: [1]) {
        // unreachable from source: the parameter is `CoreTy::Text`, so anything
        // that is not a `string` is `E0401` at the call site.
        let Some(document) = args[0].as_text() else {
            return Err(Fault::fatal(format!(
                "Core\\Xml::parse expected a `string`, got tag {}",
                args[0].tag_byte()
            )));
        };
        match parse(document) {
            Ok(tree) => Ok(instance_of(tree)),
            // The sentence is the reader's and the period is here, so no
            // message inside it has to remember to end like one.
            Err(why) => Err(Fault::thrown_as(
                ThrownClass::Parse,
                format!("{NAME}::parse(): {why}."),
            )),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `$node->kind(): Core\Xml\NodeKind` — which of the five this is.
    ///
    /// An enum answers as its ordinal, exactly as a user-declared enum does
    /// (`rule:enums/closed-integer-type`), and the slot already holds it.
    fn nvs_core_xml_node_kind(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &NODE, "kind")?;
        Ok(crate::instance::slot(receiver, KIND_SLOT))
    }
}

nvs_runtime::nvs_helper! {
    /// `$node->name(): tainted string` — an element's tag name or a processing
    /// instruction's target.
    fn nvs_core_xml_node_name(_ctx, args: [1]) {
        held(args[0], NAME_SLOT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `$node->text(): tainted string` — the character data this node carries
    /// itself.
    fn nvs_core_xml_node_text(_ctx, args: [1]) {
        held(args[0], TEXT_SLOT, "text")
    }
}

nvs_runtime::nvs_helper! {
    /// `$node->attributes(): array<tainted string>` — this element's
    /// attributes, in written order.
    fn nvs_core_xml_node_attributes(_ctx, args: [1]) {
        held(args[0], ATTRIBUTES_SLOT, "attributes")
    }
}

nvs_runtime::nvs_helper! {
    /// `$node->children(): array<Core\Xml\Node>` — this node's children, in
    /// document order.
    fn nvs_core_xml_node_children(_ctx, args: [1]) {
        held(args[0], CHILDREN_SLOT, "children")
    }
}

#[cfg(test)]
mod tests {
    use super::{KIND, Kind, Parsed, parse};

    /// A kind's name, read out of the registered roster rather than out of a
    /// second table here — so a walk asserted below is asserted against the
    /// spelling a program compares with.
    fn case(kind: Kind) -> &'static str {
        let at = usize::try_from(kind.ordinal()).expect("a case's integer is its own index");
        KIND.cases[at].0
    }

    /// Every kind of node one document holds, as `kind:name:text` per node,
    /// depth-first — what a walk sees, in one string a case can assert on.
    fn walk(node: &Parsed, into: &mut Vec<String>) {
        into.push(format!("{}:{}:{}", case(node.kind), node.name, node.text));
        for child in &node.children {
            walk(child, into);
        }
    }

    /// The whole family is five, both halves agree on which five, and one
    /// document reaches every one of them.
    ///
    /// `rule:core-classes/html-parsing` makes both parsers produce one node
    /// family, so the roster is a claim rather than an implementation detail:
    /// a sixth kind added on either side without the other is what this fails
    /// on, and stage 4's parser is checked against the same [`Kind::ALL`].
    #[test]
    fn element_text_comment_processing_instruction_and_document_are_the_whole_family() {
        assert_eq!(Kind::ALL.len(), 5, "the family is closed at five");
        let ordinals: Vec<i64> = Kind::ALL.iter().map(|kind| kind.ordinal()).collect();
        let registered: Vec<i64> = KIND.cases.iter().map(|(_, at)| *at).collect();
        assert_eq!(
            ordinals, registered,
            "the Rust family and the registered enum are one roster, case for case"
        );

        let tree =
            parse("<?xml version=\"1.0\"?><!--a--><?work do?><a k=\"v\">t<b/><![CDATA[c]]></a>")
                .expect("a well-formed document holding every kind");
        let mut seen = Vec::new();
        walk(&tree, &mut seen);
        assert_eq!(
            seen,
            vec![
                "Document::".to_owned(),
                "Comment::a".to_owned(),
                "ProcessingInstruction:work:do".to_owned(),
                "Element:a:".to_owned(),
                "Text::t".to_owned(),
                "Element:b:".to_owned(),
                "Text::c".to_owned(),
            ],
            "one document reaches every kind, and produces no other"
        );

        // The set is reached, not merely listed: every case of the family
        // appears in the walk above, which is what makes the roster a claim
        // about the parser rather than about the enum alone.
        for kind in Kind::ALL {
            assert!(
                seen.iter().any(|node| node.starts_with(case(kind))),
                "{} is in the family and no document produces one",
                case(kind)
            );
        }
    }
}
