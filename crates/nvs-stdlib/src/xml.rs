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
//! walk in any order and any number of times. [`nvs_core_xml_reader`] is the
//! stream half: it answers a walk that holds one node at a time, so a document
//! a program does not want to materialise is read by asking for the next node
//! until there is none. **No operation is available through both** — the two
//! doors share the node family and nothing else, so what a shape costs stays a
//! property of the door a program came in through, and a program picks the one
//! that fits how much of the document it needs at once. This paragraph is the
//! split's one statement; no member card re-argues it.
//!
//! Two things a walk deliberately does not carry, resolved toward the narrower
//! surface and recorded here because this is where the split is stated. A walk
//! never answers a `Document` node: that kind is a tree's root, and a reader
//! answers what it has read past rather than something holding the rest. And a
//! closing tag is not a node either, so [`nvs_core_xml_reader_depth`] is how a
//! program tells where an element ended — a depth no greater than an earlier
//! one means everything opened since has closed.
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
//! Per streaming walk: the document's own text, held as the caller handed it
//! over rather than copied, plus the node [`nvs_core_xml_reader_read`] last
//! answered and the names of the elements open around it — [`DEPTH_CEILING`]
//! of them at the very most. What a walk *builds* is one node, so reading a
//! document ten times larger costs the reader the same, which is the property
//! `rule:core-classes/xml-tree-and-stream` names and
//! `the_reader_holds_one_window_rather_than_the_document` measures.
//!
//! # Known gaps
//!
//! 1. **The writer is not here.** The reader is ([`READER`]), so a program that
//!    outgrows the tree can read a document a node at a time; writing one back
//!    out a node at a time is what is missing, and until it lands the stream
//!    half of § 17 reads and does not write.
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

use nvs_runtime::{Fault, NvsArray, NvsStr, ObjHeader, ThrownClass, Value};

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

/// `Core\Xml\Reader`'s fully-qualified name, for [`READER`] and for the return
/// position that carries one.
pub(crate) const READER_NAME: &str = r"Core\Xml\Reader";

/// `Core\Xml`'s registry rows — § 17's two front doors, one per shape: the
/// parse that materialises a whole document, and the reader that walks one a
/// node at a time. They answer the same family and share no operation, which is
/// `rule:core-classes/xml-tree-and-stream` and the paragraph in the module doc
/// above. The writer is this module's known gap 1.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "parse",
            names: &["document"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(NODE_NAME),
            symbol: "nvs_core_xml_parse",
            doc: Some(&PARSE_DOC),
        },
        CoreMethod {
            name: "reader",
            names: &["document"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(READER_NAME),
            symbol: "nvs_core_xml_reader",
            doc: Some(&READER_DOC),
        },
    ],
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

/// `Core\Xml\Reader`'s registry rows — the stream half of § 17, which is a walk
/// over a document and nothing else.
///
/// Two members, and the tree's are not among them: `read` advances the walk and
/// answers what it read, and `depth` says how deeply nested that was. That
/// disjointness is `rule:core-classes/xml-tree-and-stream` — the family is
/// shared, the operations are not — and it is why this is a second class rather
/// than a mode on [`CLASS`].
pub(crate) const READER: CoreClass = CoreClass {
    name: READER_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "read",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(NODE_NAME)),
            symbol: "nvs_core_xml_reader_read",
            doc: Some(&READ_DOC),
        },
        CoreMethod {
            name: "depth",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_xml_reader_depth",
            doc: Some(&DEPTH_DOC),
        },
    ],
    slots: &["document", "cursor", "depth", "stack", "open", "rooted"],
    constants: &[],
};

/// `Core\Xml::reader`'s reference card — `rule:core-api/reference-card`.
const READER_DOC: MethodDoc = MethodDoc {
    short: "Opens a walk over `$document` that holds one node at a time — replacing `XMLReader`. \
            A document a program does not want to materialise is read by asking for the next node \
            until there is none, and what the walk itself holds does not grow with how much of \
            the document is left. Nothing is read here: the first `read` is what reaches the \
            document's first character.",
    params: &[ParamDoc {
        name: "document",
        desc: "The document text, held as it was handed over rather than copied and read forward \
               from as the walk goes. No entity is resolved from anywhere, exactly as `parse` \
               resolves none.",
        shape: &[],
    }],
    ret: "A reader positioned before the first node.",
    errors: &[],
};

/// `Core\Xml\Reader::read`'s reference card — `rule:core-api/reference-card`.
const READ_DOC: MethodDoc = MethodDoc {
    short: "The next node of the walk, or `null` at the end of the document — the one operation \
            that advances a reader. An element arrives when its opening tag is read, carrying its \
            name and its attributes and no children, because nothing inside it has been read yet; \
            what is inside arrives as the nodes that follow. A closing tag is not a node, so \
            `depth` is how a program tells where one element ended and the next began.",
    params: &[],
    ret: "The node just read, of the family a parsed tree is made of — every kind but `Document`, \
          which is a tree's root and a walk has none. `null` once the document is finished, and \
          every string a node carries is `tainted`.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "The document is not well-formed where the walk has reached — the same refusals \
               `parse` makes, reported when a node reaches them rather than before the first node \
               is answered. The walk does not advance past one, so asking again reports the same \
               sentence.",
    }],
};

/// `Core\Xml\Reader::depth`'s reference card — `rule:core-api/reference-card`.
const DEPTH_DOC: MethodDoc = MethodDoc {
    short: "How many elements are open around the node `read` last answered: `0` for the root \
            element and for anything written beside it, one more for each element it is nested \
            inside. This is the structure a walk carries, since a closing tag is not a node — a \
            depth no greater than an earlier one means every element opened since has closed.",
    params: &[],
    ret: "The depth of the node last answered, and `0` both before the first `read` and after the \
          one that answered `null`.",
    errors: &[],
};

/// [`READER`]'s slots, in declaration order.
const DOCUMENT_SLOT: usize = 0;
/// See [`DOCUMENT_SLOT`].
const CURSOR_SLOT: usize = 1;
/// See [`DOCUMENT_SLOT`].
const DEPTH_SLOT: usize = 2;
/// See [`DOCUMENT_SLOT`].
const STACK_SLOT: usize = 3;
/// See [`DOCUMENT_SLOT`].
const OPEN_SLOT: usize = 4;
/// See [`DOCUMENT_SLOT`].
const ROOTED_SLOT: usize = 5;

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
        "nvs_core_xml_reader" => (nvs_core_xml_reader as *const ()).cast(),
        "nvs_core_xml_reader_read" => (nvs_core_xml_reader_read as *const ()).cast(),
        "nvs_core_xml_reader_depth" => (nvs_core_xml_reader_depth as *const ()).cast(),
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

/// The names of the elements a streaming walk has open, over the slots that
/// hold them.
///
/// A stack in a Novis array rather than a Rust `Vec`, because a `Core`
/// instance's state is values Novis can already hold ([`crate::instance`]) and
/// a walk's position has to survive between two `read` calls. A pop leaves the
/// entry where it is and moves [`Self::depth`] instead, so the array grows to
/// the deepest the document reached and never past it: the next push overwrites
/// what a deeper element left behind. [`DEPTH_CEILING`] bounds that, which is
/// what makes the stack O(deepest) rather than O(document).
struct Open<'a> {
    /// The open names, of which the first [`Self::depth`] are live.
    names: &'a mut NvsArray,
    /// How many elements are open.
    depth: usize,
}

impl Open<'_> {
    /// How deeply nested whatever is read next will be.
    fn depth(&self) -> usize {
        self.depth
    }

    /// The name of the innermost open element, or `None` outside the root.
    fn innermost(&self) -> Option<String> {
        let at = i64::try_from(self.depth.checked_sub(1)?).ok()?;
        Some(self.names.get_index(at)?.as_text()?.to_owned())
    }

    /// Opens `name`.
    ///
    /// The write cannot move the array: the reader owns the only reference to
    /// its own stack, since no member hands it out, so
    /// [`NvsArray::set_index`]'s copy-on-write separation never fires and the
    /// handle stays the one the slot names.
    fn push(&mut self, name: &str) {
        let at = i64::try_from(self.depth).expect("`DEPTH_CEILING` is far under `i64::MAX`");
        self.names
            .set_index(at, Value::str(NvsStr::new(name.as_bytes())));
        self.depth += 1;
    }

    /// Closes the innermost open element and answers the name it opened as, or
    /// `None` where nothing is open.
    fn pop(&mut self) -> Option<String> {
        let name = self.innermost()?;
        self.depth -= 1;
        Some(name)
    }
}

impl<'a> Reader<'a> {
    /// A reader positioned at the start of `src`.
    fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    /// A reader positioned `pos` bytes into `src` — a streaming walk resuming
    /// where the call before it stopped, where [`Self::new`] is the
    /// whole-document entry.
    fn at(src: &'a str, pos: usize) -> Self {
        debug_assert!(
            src.is_char_boundary(pos),
            "a cursor this module wrote is where one of its own reads stopped"
        );
        Self { src, pos }
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

    /// The next node of a streaming walk and how deeply nested it is, or `None`
    /// at the end of the document.
    ///
    /// The grammar [`Self::document`] and [`Self::tree`] accept, read one node
    /// at a time: `open` is the stack `tree` keeps on the Rust stack, living in
    /// the reader's own slots instead so that it survives between calls, and
    /// `rooted` is `document`'s own flag. A closing tag is not a node, so this
    /// walks past one rather than answering it, and the depth it answers is the
    /// node's own — the count of elements open *around* it, before an opening
    /// tag pushes its own.
    ///
    /// # Errors
    ///
    /// Every refusal [`Self::document`] makes, reported when the walk reaches
    /// it rather than before the first node is answered.
    fn step(
        &mut self,
        open: &mut Open<'_>,
        rooted: &mut bool,
    ) -> Result<Option<(Parsed, usize)>, String> {
        if self.pos == 0 {
            self.declaration()?;
        }
        loop {
            // Outside the root element, whitespace is not content — the one
            // place the two differ, and `document`'s own rule.
            if open.depth() == 0 {
                self.skip_space();
            }
            let rest = self.rest();
            if rest.is_empty() {
                if let Some(name) = open.innermost() {
                    return Err(format!("`<{name}>` is never closed"));
                }
                if !*rooted {
                    return Err("a document needs a root element".to_owned());
                }
                return Ok(None);
            }
            if rest.starts_with("</") {
                let Some(opened) = ({
                    self.pos += "</".len();
                    let name = self.name("a closing tag")?;
                    self.skip_space();
                    if !self.eat(">") {
                        return Err(format!("`</{name}` is never closed"));
                    }
                    open.pop().map(|opened| (opened, name))
                }) else {
                    return Err(format!(
                        "`{}` closes an element nothing opened",
                        self.here()
                    ));
                };
                let (opened, name) = opened;
                if opened != name {
                    return Err(format!(
                        "`<{opened}>` is closed by `</{name}>`, and an element closes as what it \
                         opened as"
                    ));
                }
                continue;
            }
            let depth = open.depth();
            if rest.starts_with("<!--") {
                return Ok(Some((self.comment()?, depth)));
            }
            if rest.starts_with("<!DOCTYPE") {
                return Err(DOCTYPE_REFUSAL.to_owned());
            }
            if rest.starts_with("<![CDATA[") && depth > 0 {
                return Ok(Some((self.cdata()?, depth)));
            }
            if rest.starts_with("<!") && depth > 0 {
                return Err(format!(
                    "`{}` is not something a document may hold",
                    self.here()
                ));
            }
            if rest.starts_with("<?") {
                return Ok(Some((self.processing_instruction()?, depth)));
            }
            if rest.starts_with('<') {
                if depth == 0 && *rooted {
                    return Err(
                        "a document has one root element, and this is a second one".to_owned()
                    );
                }
                let (node, closed) = self.start_tag()?;
                *rooted = true;
                if !closed {
                    if depth >= DEPTH_CEILING {
                        return Err(format!(
                            "elements nest deeper than {DEPTH_CEILING}, which is as deep as a \
                             document may be"
                        ));
                    }
                    open.push(&node.name);
                }
                return Ok(Some((node, depth)));
            }
            if depth == 0 {
                return Err(format!(
                    "`{}` is character data outside the root element",
                    self.here()
                ));
            }
            return Ok(Some((self.chardata()?, depth)));
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

/// A count of this module's own, as the `uint` a slot holds.
fn counted(count: usize) -> Value {
    Value::uint(u64::try_from(count).expect("a count taken off a document is under `u64::MAX`"))
}

/// Slot `index` of a reader, as the count it holds.
///
/// # Errors
///
/// A `Fault::fatal` for a slot holding anything else, which only a bug in this
/// module can produce: a reader's slots are written by [`nvs_core_xml_reader`]
/// and by [`read`] and by nothing else.
fn count_slot(receiver: *mut ObjHeader, index: usize, member: &str) -> Result<usize, Fault> {
    let held = crate::instance::slot(receiver, index);
    held.as_uint()
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| wrong_slot(member, index, held))
}

/// The fault for a reader slot holding something [`read`] did not write there.
fn wrong_slot(member: &str, index: usize, held: Value) -> Fault {
    Fault::fatal(format!(
        "{READER_NAME}::{member} found tag {} in its `{}` slot",
        held.tag_byte(),
        READER.slots[index]
    ))
}

/// [`nvs_core_xml_reader_read`]'s body: one step of a walk, over the slots that
/// hold its position.
///
/// A reader's whole state is those slots — the document's own text, a cursor
/// into it, the stack of open names, and the two counts — so it is an ordinary
/// `Core` value with no native allocation behind it and nothing to free when it
/// is released. What this builds per call is one node.
///
/// # Errors
///
/// A `ParseError` for a document that is not well-formed where the walk has
/// reached, and a `Fault::fatal` for a slot this module wrote wrong. A refusal
/// writes no slot at all, so asking again reads from the same place and reports
/// the same sentence rather than skipping the node it refused.
fn read(receiver: Value) -> Result<Value, Fault> {
    let object = crate::instance::receiver(receiver, &READER, "read")?;
    let held = crate::instance::slot(object, DOCUMENT_SLOT);
    let document = held
        .as_text()
        .ok_or_else(|| wrong_slot("read", DOCUMENT_SLOT, held))?;
    let held = crate::instance::slot(object, ROOTED_SLOT);
    let mut rooted = held
        .as_bool()
        .ok_or_else(|| wrong_slot("read", ROOTED_SLOT, held))?;
    let held = crate::instance::slot(object, STACK_SLOT);
    let stack = held
        .array_ptr()
        .ok_or_else(|| wrong_slot("read", STACK_SLOT, held))?;
    let mut stack = crate::arr::borrowed(stack);
    let mut open = Open {
        names: &mut stack,
        depth: count_slot(object, OPEN_SLOT, "read")?,
    };

    let mut scan = Reader::at(document, count_slot(object, CURSOR_SLOT, "read")?);
    let stepped = scan.step(&mut open, &mut rooted).map_err(|why| {
        Fault::thrown_as(
            ThrownClass::Parse,
            // The sentence is the reader's and the period is here, exactly as
            // the parse's is, so no message has to remember to end like one.
            format!("{READER_NAME}::read(): {why}."),
        )
    })?;
    let depth = open.depth();

    crate::instance::set_slot(object, CURSOR_SLOT, counted(scan.pos));
    crate::instance::set_slot(object, OPEN_SLOT, counted(depth));
    crate::instance::set_slot(object, ROOTED_SLOT, Value::bool(rooted));
    let Some((node, at)) = stepped else {
        crate::instance::set_slot(object, DEPTH_SLOT, counted(0));
        return Ok(Value::null());
    };
    crate::instance::set_slot(object, DEPTH_SLOT, counted(at));
    Ok(instance_of(node))
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

nvs_runtime::nvs_helper! {
    /// `Core\Xml::reader(string $document): Core\Xml\Reader` — replacing
    /// `XMLReader::xml` and `xml_parser_create`.
    ///
    /// Nothing is read here. The reader holds the document's own text, a cursor
    /// at its start, an empty stack and the two counts, and the first
    /// [`nvs_core_xml_reader_read`] is what reaches the first character —
    /// which is why a malformed document is refused by the walk rather than by
    /// this.
    fn nvs_core_xml_reader(_ctx, args: [1]) {
        // unreachable from source: the parameter is `CoreTy::Text`, so anything
        // that is not a `string` is `E0401` at the call site.
        if args[0].as_text().is_none() {
            return Err(Fault::fatal(format!(
                "Core\\Xml::reader expected a `string`, got tag {}",
                args[0].tag_byte()
            )));
        }
        let document = args[0];
        #[expect(
            unsafe_code,
            reason = "the argument is borrowed from the caller's frame, so the \
                      slot that outlives this call needs a reference of its own"
        )]
        unsafe {
            document.retain();
        }
        Ok(crate::instance::build(
            &READER,
            [
                document,
                counted(0),
                counted(0),
                Value::array(NvsArray::new()),
                counted(0),
                Value::bool(false),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$reader->read(): ?Core\Xml\Node` — the next node of the walk, or `null`
    /// at the end of the document.
    fn nvs_core_xml_reader_read(_ctx, args: [1]) {
        read(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `$reader->depth(): uint` — how deeply nested the node
    /// [`nvs_core_xml_reader_read`] last answered was.
    ///
    /// A slot read, like every member on a node: the walk wrote the depth when
    /// it answered, so there is nothing here to recompute and nothing to fail.
    fn nvs_core_xml_reader_depth(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &READER, "depth")?;
        Ok(crate::instance::slot(receiver, DEPTH_SLOT))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsStr, OutputSink, Tag, Value, call};

    use super::{
        ATTRIBUTES_SLOT, CHILDREN_SLOT, CLASS, DOCTYPE_REFUSAL, KIND, KIND_NAME, KIND_SLOT, Kind,
        NAME, NAME_SLOT, NODE, NODE_NAME, Parsed, TEXT_SLOT, parse,
    };
    use crate::registry::{CLASSES, CoreClass, CoreTy, Qual};

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

    /// What a reference may resolve to is a closed vocabulary, and the closure
    /// is the whole mitigation.
    ///
    /// `rule:core-classes/xml-refuses-by-construction`: the five predefined
    /// entities and numeric character references expand because they are the
    /// document's own characters written another way, and every other name —
    /// including the ones a document type declaration would have introduced and
    /// the HTML spellings a program reaches for out of habit — is refused by one
    /// sentence naming the three places nothing is resolved from. The sweep is
    /// what makes this a claim about the parser rather than about a default:
    /// there is no name that answers differently, and [`CLASS`] carries no
    /// second parameter a flag could have occupied.
    #[test]
    fn an_external_entity_is_not_a_code_path_rather_than_a_flag_defaulting_to_off() {
        for (reference, expanded) in [
            ("&amp;", "&"),
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&quot;", "\""),
            ("&apos;", "'"),
            ("&#65;", "A"),
            ("&#x41;", "A"),
        ] {
            let tree = parse(&format!("<a>{reference}</a>"))
                .expect("a predefined entity is the document's own character");
            let mut seen = Vec::new();
            walk(&tree, &mut seen);
            assert!(
                seen.contains(&format!("Text::{expanded}")),
                "{reference} is one of the seven that expand, and it answered {seen:?}"
            );
        }

        for name in ["xxe", "file", "sp", "nbsp", "copy", "lol1", "ent"] {
            let err = parse(&format!("<a>&{name};</a>")).expect_err(
                "no entity is resolved from anywhere, so no name outside the five works",
            );
            assert_eq!(
                err,
                format!(
                    "`&{name};` is not one of the five predefined entities, and no entity is \
                     resolved from a document type declaration, an internal subset or the network"
                ),
                "every unknown name is refused by the same sentence, because none of them is looked \
                 up anywhere"
            );
        }

        // The other half of "rather than a flag": no signature on this class
        // has anywhere to put one. Every door takes the document's text and
        // nothing else, with no defaults, so there is no argument a caller
        // could pass and no default a deployment could have changed — and the
        // sweep is over the rows rather than over the parse alone, because a
        // flag added to the stream would resolve entities just as thoroughly.
        for member in CLASS.methods {
            assert!(
                member
                    .params
                    .iter()
                    .all(|param| matches!(param, CoreTy::Text(Qual::Neutral))),
                "`Core\\Xml::{}` takes something that is not document text, which is where a \
                 setting would go",
                member.name
            );
            assert!(
                member.defaults.is_empty(),
                "a default is a setting once there is a parameter to hang it on"
            );
        }
    }

    /// A document type declaration stops at its own token, whatever it names.
    ///
    /// `rule:core-classes/xml-refuses-by-construction` refuses the declaration
    /// **whole**, and the assertion that says so is that all five spellings
    /// below — bare, `SYSTEM` over the network, `SYSTEM` over the filesystem,
    /// `PUBLIC`, and an internal subset declaring an external entity — answer
    /// the *same* sentence in both positions a declaration can appear. A parser
    /// that read the identifier far enough to decide would have to differ
    /// somewhere across that table.
    #[test]
    fn a_dtd_naming_an_external_subset_is_refused_rather_than_fetched() {
        for dtd in [
            "<!DOCTYPE a>",
            "<!DOCTYPE a SYSTEM \"http://example.invalid/a.dtd\">",
            "<!DOCTYPE a SYSTEM \"file:///etc/passwd\">",
            "<!DOCTYPE a PUBLIC \"-//W3C//DTD XHTML 1.0//EN\" \"http://www.w3.org/TR/xhtml1.dtd\">",
            "<!DOCTYPE a [<!ENTITY x SYSTEM \"file:///etc/passwd\">]>",
        ] {
            for document in [format!("{dtd}<a/>"), format!("<a>{dtd}</a>")] {
                let err = parse(&document)
                    .expect_err("a declaration is not something a document may hold");
                assert_eq!(
                    err, DOCTYPE_REFUSAL,
                    "`{document}` is refused by the identifier never being read, so the sentence \
                     cannot depend on what it named"
                );
            }
        }
    }

    /// The billion-laughs document is closed at its declaration, not metered.
    ///
    /// `rule:core-classes/xml-refuses-by-construction`: an expansion needs an
    /// internal subset to declare its entities in, and there is none, so there
    /// is no expansion factor for a ratio and ceiling to bound. Both sides are
    /// asserted, because either alone would pass against a parser that read the
    /// subset — with the declaration the parse stops at `<!DOCTYPE`, and with it
    /// stripped the first reference is refused as a name nothing defines.
    #[test]
    fn a_billion_laughs_document_is_refused_at_its_doctype_rather_than_bounded() {
        const SUBSET: &str = concat!(
            "<!DOCTYPE lolz [",
            "<!ENTITY lol \"lol\">",
            "<!ENTITY lol1 \"&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;\">",
            "<!ENTITY lol2 \"&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;\">",
            "]>"
        );

        let bomb = format!("{SUBSET}<lolz>&lol2;</lolz>");
        assert_eq!(
            parse(&bomb).expect_err("the declaration is refused before anything is expanded"),
            DOCTYPE_REFUSAL,
            "the bomb stops at its declaration, so no bound is reached and none is reported"
        );

        let err = parse("<lolz>&lol2;</lolz>")
            .expect_err("a reference to a name nothing defines is refused on its own");
        assert!(
            err.starts_with("`&lol2;` is not one of the five predefined entities"),
            "with the subset gone the reference is still unresolvable, and answered {err}"
        );
    }

    /// Whether a signature's type mentions `class` anywhere inside it.
    ///
    /// Exhaustive with the leaves grouped rather than swept up by a `_`,
    /// because this backs an assertion about an *absence*: a composite variant
    /// added later must be a build error here rather than a hole the sweep
    /// walks straight past. Same shape, and for the same reason, as
    /// `Core\Ast`'s.
    fn mentions(ty: &CoreTy, class: &str) -> bool {
        match ty {
            CoreTy::Instance(name)
            | CoreTy::ShapeOfCallables(name)
            | CoreTy::Written(name)
            | CoreTy::Enum(name)
            | CoreTy::EnumCase(name, _)
            | CoreTy::Var(name) => *name == class,
            CoreTy::Array(inner)
            | CoreTy::Nullable(inner)
            | CoreTy::Iterated(inner)
            | CoreTy::Variadic(inner) => mentions(inner, class),
            CoreTy::InstanceAt(name, args) => {
                *name == class || args.iter().any(|arg| mentions(arg, class))
            }
            CoreTy::Union(members) => members.iter().any(|member| mentions(member, class)),
            CoreTy::CallableSig(params, ret) => {
                params.iter().any(|param| mentions(param, class)) || mentions(ret, class)
            }
            CoreTy::Options(options) => options.iter().any(|option| mentions(&option.ty, class)),
            CoreTy::Shape(arms) => arms
                .iter()
                .any(|arm| arm.iter().any(|field| mentions(&field.ty, class))),
            CoreTy::Bool
            | CoreTy::Int
            | CoreTy::Uint
            | CoreTy::Float
            | CoreTy::Decimal
            | CoreTy::Str
            | CoreTy::Bytes
            | CoreTy::Text(_)
            | CoreTy::Blob(_)
            | CoreTy::SecretBytes
            | CoreTy::SecretBlob(_)
            | CoreTy::TaintedStr
            | CoreTy::TaintedBytes
            | CoreTy::SecretTaintedStr
            | CoreTy::Void
            | CoreTy::Mixed
            | CoreTy::Callable
            | CoreTy::Entry
            | CoreTy::IntLiteral(_) => false,
        }
    }

    /// Whether every place `ty` can carry text carries it in the `tainted`
    /// form.
    ///
    /// Exhaustive for the same reason [`mentions`] is: the assertion is that
    /// there is no unmarked text position anywhere in a node's answers, and a
    /// composite variant that appeared later would otherwise be a position the
    /// sweep never looked at.
    fn every_text_position_is_tainted(ty: &CoreTy) -> bool {
        match ty {
            CoreTy::Str
            | CoreTy::Bytes
            | CoreTy::Text(_)
            | CoreTy::Blob(_)
            | CoreTy::SecretBytes
            | CoreTy::SecretBlob(_) => false,
            CoreTy::Array(inner)
            | CoreTy::Nullable(inner)
            | CoreTy::Iterated(inner)
            | CoreTy::Variadic(inner) => every_text_position_is_tainted(inner),
            CoreTy::InstanceAt(_, args) => args.iter().all(every_text_position_is_tainted),
            CoreTy::Union(members) => members.iter().all(every_text_position_is_tainted),
            CoreTy::CallableSig(params, ret) => {
                params.iter().all(every_text_position_is_tainted)
                    && every_text_position_is_tainted(ret)
            }
            CoreTy::Options(options) => options
                .iter()
                .all(|option| every_text_position_is_tainted(&option.ty)),
            CoreTy::Shape(arms) => arms.iter().all(|arm| {
                arm.iter()
                    .all(|field| every_text_position_is_tainted(&field.ty))
            }),
            CoreTy::Instance(_)
            | CoreTy::ShapeOfCallables(_)
            | CoreTy::Written(_)
            | CoreTy::Enum(_)
            | CoreTy::EnumCase(_, _)
            | CoreTy::Var(_)
            | CoreTy::Bool
            | CoreTy::Int
            | CoreTy::Uint
            | CoreTy::Float
            | CoreTy::Decimal
            | CoreTy::TaintedStr
            | CoreTy::TaintedBytes
            | CoreTy::SecretTaintedStr
            | CoreTy::Void
            | CoreTy::Mixed
            | CoreTy::Callable
            | CoreTy::Entry
            | CoreTy::IntLiteral(_) => true,
        }
    }

    /// Every string a program can read out of a parsed tree is `tainted`, and
    /// the mark does not depend on what was parsed.
    ///
    /// `rule:security/tainted-sources` applied to a parser: what comes out of
    /// one over bytes a program did not write is untrusted, whatever the
    /// argument was. The sweep is over the whole instance roster rather than
    /// over the three members that answer text today, so a sixth member
    /// answering an unmarked `string` fails here — which is the shape a
    /// namespace or a doctype accessor would arrive in.
    ///
    /// The parameter's [`Qual::Neutral`] is the second half and not a detail:
    /// [`Qual::Contagious`] would make the answer's mark depend on the
    /// argument's, so a tree parsed from a literal would come back unmarked.
    /// A qualifier is erased before codegen, so these rows are the whole claim
    /// — there is nothing at run time for a case to look at.
    #[test]
    fn every_string_read_out_of_a_parsed_tree_is_tainted() {
        for member in NODE.members() {
            assert!(
                every_text_position_is_tainted(&member.return_ty),
                "{NODE_NAME}::{} answers text that is not `tainted`, and a parsed tree has no \
                 unmarked half",
                member.name
            );
        }

        assert!(
            matches!(CLASS.methods[0].params[0], CoreTy::Text(Qual::Neutral)),
            "the parse's parameter is neutral, so the answer is tainted unconditionally rather \
             than contagiously"
        );
    }

    /// Whether a type reaches nothing but the tree, its text and its closed
    /// kind — a whitelist, so a variant added later fails closed.
    fn is_inert(ty: &CoreTy) -> bool {
        match ty {
            CoreTy::Instance(name) => *name == NODE_NAME,
            CoreTy::Enum(name) => *name == KIND_NAME,
            CoreTy::Array(inner) => is_inert(inner),
            CoreTy::TaintedStr => true,
            _ => false,
        }
    }

    /// A parsed tree is inert data, on
    /// `rule:tooling/reflection-and-source-parsing-are-core-features`'s rule
    /// for the AST — the same property, shown the same three ways, because the
    /// two trees are the same kind of thing.
    ///
    /// 1. **No member anywhere in `Core` accepts a node or the class.** The
    ///    sweep is over the whole registry, because a door that ran a tree
    ///    would be declared next to whatever ran it rather than here.
    /// 2. **A walk reaches nothing but the tree.** Every member a node has
    ///    answers a node, its text or its kind, so no amount of walking
    ///    produces a value of another class.
    /// 3. **The values are ordinary data at run time too**, over a real parse:
    ///    every slot is an integer, text or an array of those, and there is no
    ///    closure, callable or resource anywhere in it.
    #[test]
    fn a_parsed_tree_has_no_path_back_into_execution() {
        for class in CLASSES {
            for member in class.members() {
                for param in member.params {
                    assert!(
                        !mentions(param, NODE_NAME) && !mentions(param, NAME),
                        "{}::{} takes a parsed tree, which would be the path back into execution \
                         this closes",
                        class.name,
                        member.name
                    );
                }
            }
        }

        for member in NODE.members() {
            assert!(
                is_inert(&member.return_ty),
                "{NODE_NAME}::{} answers something that is neither the tree, its text nor its \
                 kind, so walking the tree reaches outside it",
                member.name
            );
        }

        let source = Value::str(NvsStr::new(b"<?xml version=\"1.0\"?><a k=\"v\">t<b/></a>"));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree = call(super::nvs_core_xml_parse, &mut ctx, &[source])
            .expect("that document is well-formed");
        let mut seen = 0usize;
        assert_data(tree, &mut seen);
        // Document, the element, its text and the empty element — the walk
        // reached every one, so the property was checked over a tree rather
        // than over its root.
        assert_eq!(seen, 4, "every node of the parsed document was inspected");

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference `parse` answered with, and the tree's own \
                      references are the nodes' own"
        )]
        unsafe {
            tree.release();
            source.release();
        }
    }

    /// Asserts that `node` is a [`NODE`] holding nothing but data, and
    /// recurses into its children, counting what it inspected.
    fn assert_data(node: Value, seen: &mut usize) {
        *seen += 1;
        let receiver = node.obj_ptr().expect("a node is an object");
        assert_eq!(
            crate::instance::slot(receiver, KIND_SLOT).tag(),
            Some(Tag::Int),
            "a node's kind is the case's ordinal"
        );
        for (slot, what) in [(NAME_SLOT, "name"), (TEXT_SLOT, "text")] {
            assert_eq!(
                crate::instance::slot(receiver, slot).tag(),
                Some(Tag::Str),
                "a node's {what} is text"
            );
        }
        for (slot, each) in [(ATTRIBUTES_SLOT, Some(Tag::Str)), (CHILDREN_SLOT, None)] {
            let held = crate::instance::slot(receiver, slot);
            assert_eq!(held.tag(), Some(Tag::Array), "a node holds its own array");
            let array = held.array_ptr().expect("the slot is an array");
            let array = crate::arr::borrowed(array);
            let mut from = 0usize;
            while let Some(at) = array.next_slot(from) {
                let value = array
                    .value_at(at)
                    .expect("next_slot only names live entries");
                match each {
                    Some(tag) => assert_eq!(value.tag(), Some(tag), "an attribute value is text"),
                    None => assert_data(value, seen),
                }
                from = at + 1;
            }
        }
    }

    /// Releases the one reference this test module owns to `value`.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "a member answers with a reference of its own, and a test that keeps none \
                      has to give it back"
        )]
        unsafe {
            value.release();
        }
    }

    /// The stream answers the tree's own node family, one node per call, in
    /// document order.
    ///
    /// `rule:core-classes/xml-tree-and-stream`: the two shapes share the
    /// vocabulary and share no operation, so what is worth asserting is that
    /// they **agree** — one document asked of both, and the walk sees the nodes
    /// the tree holds, in the order the tree holds them. A stream that grew a
    /// kind of its own, or that answered a subtree where the tree answers a
    /// node, fails here rather than in a program. The document is
    /// `element_text_comment_processing_instruction_and_document_are_the_whole_family`'s,
    /// so the family reached is that test's roster and not a second one.
    #[test]
    fn the_reader_answers_stage_twos_node_family_one_node_at_a_time() {
        const DOCUMENT: &str =
            "<?xml version=\"1.0\"?><!--a--><?work do?><a k=\"v\">t<b/><![CDATA[c]]></a>";

        let mut expected = Vec::new();
        walk(
            &parse(DOCUMENT).expect("a well-formed document holding every kind"),
            &mut expected,
        );
        // The document node is the tree's root, and a walk has no root: a
        // reader answers what it has read past rather than something that holds
        // the rest.
        assert_eq!(expected.remove(0), "Document::");

        let source = Value::str(NvsStr::new(DOCUMENT.as_bytes()));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let reader = call(super::nvs_core_xml_reader, &mut ctx, &[source])
            .expect("a reader reads nothing until it is asked");
        let mut seen = Vec::new();
        let mut depths = Vec::new();
        loop {
            let node = call(super::nvs_core_xml_reader_read, &mut ctx, &[reader])
                .expect("that document is well-formed");
            if node.tag() == Some(Tag::Null) {
                break;
            }
            let object = node.obj_ptr().expect("a node is an object");
            let at = usize::try_from(
                crate::instance::slot(object, KIND_SLOT)
                    .as_int()
                    .expect("a node's kind is its case's ordinal"),
            )
            .expect("a case's integer is its own index");
            let text = |slot| {
                crate::instance::slot(object, slot)
                    .as_text()
                    .expect("a node's name and text are text")
                    .to_owned()
            };
            seen.push(format!(
                "{}:{}:{}",
                KIND.cases[at].0,
                text(NAME_SLOT),
                text(TEXT_SLOT)
            ));

            // One node, never a subtree: an element arrives when its opening
            // tag is read, so nothing inside it has been read yet.
            let children = crate::instance::slot(object, CHILDREN_SLOT);
            let children =
                crate::arr::borrowed(children.array_ptr().expect("a node holds its own array"));
            assert!(
                children.is_empty(),
                "a walk answers one node and not what is under it"
            );

            depths.push(
                call(super::nvs_core_xml_reader_depth, &mut ctx, &[reader])
                    .expect("a depth is a slot read")
                    .as_uint()
                    .expect("a depth is a count"),
            );
            dropped(node);
        }

        assert_eq!(
            seen, expected,
            "the walk and the tree see the same nodes, in the same order"
        );
        assert_eq!(
            depths,
            vec![0, 0, 0, 1, 1, 1],
            "the comment, the instruction and the root element are written at the top of the \
             document, and the root's own content one deeper"
        );

        // The end of a document is a state and not a refusal, so asking again
        // answers it again — and the depth goes with the node that is no longer
        // there.
        let ended = call(super::nvs_core_xml_reader_read, &mut ctx, &[reader])
            .expect("the end of a document is not a refusal");
        assert_eq!(ended.tag(), Some(Tag::Null));
        assert_eq!(
            call(super::nvs_core_xml_reader_depth, &mut ctx, &[reader])
                .expect("a depth is a slot read")
                .as_uint(),
            Some(0),
            "no node, no depth"
        );

        dropped(reader);
        dropped(source);
    }

    /// The two shapes share the node family and share no operation.
    ///
    /// `rule:core-classes/xml-tree-and-stream` is written as a disjointness,
    /// and the mistake it forbids is a concrete one: `XMLReader` grew `name`,
    /// `value` and `getAttribute` beside the DOM's, so a program reading a
    /// document through the stream learned a second vocabulary for the same
    /// questions and the memory story stopped being a property of the door it
    /// came in through. So no class of the stream half may declare a member the
    /// family declares — a walk reads a node through the value it was answered
    /// with — and none of them may answer a *list* of nodes, which is a
    /// materialised subtree wearing the stream's name.
    ///
    /// The stream half is read off the registry rather than listed here, so a
    /// class registered under `Core\Xml\` beside the family joins this sweep by
    /// existing.
    #[test]
    fn no_operation_is_available_through_both_the_tree_and_the_stream() {
        let stream: Vec<&'static CoreClass> = CLASSES
            .iter()
            .filter(|class| class.name.starts_with(r"Core\Xml\") && class.name != NODE_NAME)
            .collect();
        assert!(
            !stream.is_empty(),
            "the stream half of § 17 is registered, so there is something to be disjoint from"
        );

        for class in &stream {
            for member in class.members() {
                assert!(
                    !NODE.members().any(|shared| shared.name == member.name),
                    "{}::{} is a member the node family already declares, so one question has \
                     two spellings and which shape a program picked stopped being invisible",
                    class.name,
                    member.name
                );
                assert!(
                    !matches!(member.return_ty, CoreTy::Array(CoreTy::Instance(_))),
                    "{}::{} answers a list of instances, which is a materialised subtree \
                     answered through the door that exists not to materialise one",
                    class.name,
                    member.name
                );
            }
        }

        // The doors themselves: every row on `Core\Xml` opens exactly one of
        // the two shapes, and both shapes have one. A row answering something
        // that is neither is a third shape nobody decided on.
        let mut opened: Vec<&str> = Vec::new();
        for member in CLASS.methods {
            let shape = match member.return_ty {
                CoreTy::Instance(name) if name == NODE_NAME => "tree",
                CoreTy::Instance(name) if stream.iter().any(|class| class.name == name) => "stream",
                _ => panic!(
                    "`Core\\Xml::{}` answers neither the tree nor the stream",
                    member.name
                ),
            };
            opened.push(shape);
        }
        assert!(
            opened.contains(&"tree") && opened.contains(&"stream"),
            "§ 17 is two shapes and both are reachable: {opened:?}"
        );
    }

    /// A walk's own footprint is one node, and not the document.
    ///
    /// `rule:core-classes/xml-tree-and-stream`'s defining property for the
    /// stream half, measured rather than described: the same walk over a
    /// document ten times larger holds no more, where materialising it holds
    /// ten times as much. `nvs_runtime::budget`'s counters are per thread, so
    /// this reads what this test allocated and nothing another test is doing
    /// beside it; the baseline is taken with the document's own text already
    /// allocated, because what is being asked is what each *shape* adds for a
    /// program that already has the bytes.
    #[test]
    fn the_reader_holds_one_window_rather_than_the_document() {
        /// A document of `items` sibling elements, each carrying an attribute,
        /// a child element and text — three nodes a tree holds and three a walk
        /// sees one at a time.
        fn document(items: usize) -> String {
            let mut out = String::from("<order>");
            for _ in 0..items {
                out.push_str("<item sku=\"a\"><name>widget</name></item>");
            }
            out.push_str("</order>");
            out
        }

        /// What materialising `text` holds, and the most a walk over it holds
        /// at any one moment.
        fn cost(text: &str) -> (isize, isize) {
            let source = Value::str(NvsStr::new(text.as_bytes()));
            let mut ctx = Ctx::new(OutputSink::Sink);
            let base = nvs_runtime::budget::live_bytes();

            let tree = call(super::nvs_core_xml_parse, &mut ctx, &[source])
                .expect("the document is well-formed");
            let materialised = nvs_runtime::budget::live_bytes() - base;
            dropped(tree);

            let reader = call(super::nvs_core_xml_reader, &mut ctx, &[source])
                .expect("a reader reads nothing until it is asked");
            let mut peak = 0isize;
            loop {
                let node = call(super::nvs_core_xml_reader_read, &mut ctx, &[reader])
                    .expect("the document is well-formed");
                if node.tag() == Some(Tag::Null) {
                    break;
                }
                peak = peak.max(nvs_runtime::budget::live_bytes() - base);
                dropped(node);
            }
            dropped(reader);
            dropped(source);
            (materialised, peak)
        }

        // The first document through either shape pays for what this core
        // builds once and keeps — the leaked class descriptors among it — so it
        // is measured and thrown away rather than charged to a document.
        let _warm = cost(&document(2));
        let (small_tree, small_walk) = cost(&document(8));
        let (large_tree, large_walk) = cost(&document(80));

        assert!(
            large_tree > small_tree * 5,
            "a tree is proportional to the document: {small_tree} bytes for the small one, \
             {large_tree} for the one ten times its size"
        );
        assert!(
            large_walk <= small_walk + 1024,
            "a walk holds one window, so ten times the document costs it nothing more: \
             {small_walk} bytes against {large_walk}"
        );
        assert!(
            large_walk * 8 < large_tree,
            "the two shapes are different jobs rather than two spellings of one: the walk held \
             {large_walk} bytes where the tree held {large_tree}"
        );
    }
}
