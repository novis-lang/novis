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
//! until there is none. [`nvs_core_xml_writer`] is that half from the other
//! side, building a document a node at a time out of state that is the
//! writer's rather than the caller's — so an unclosed element is refused where
//! it was written instead of reaching a reader. **No operation is available
//! through both** — the doors share the node family and nothing else, so what a
//! shape costs stays a property of the door a program came in through, and a
//! program picks the one that fits how much of the document it needs at once.
//! This paragraph is the split's one statement; no member card re-argues it.
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
//! returns. That tree is many times the size of a document of small nodes, so
//! the parse asks the request's budget once per node and once per attribute
//! and stops at the memory limit's `FATAL` rather than build past it.
//!
//! Per streaming walk: the document's own text, held as the caller handed it
//! over rather than copied, plus the node [`nvs_core_xml_reader_read`] last
//! answered and the names of the elements open around it — [`DEPTH_CEILING`]
//! of them at the very most. What a walk *builds* is one node, so reading a
//! document ten times larger costs the reader the same, which is the property
//! `rule:core-classes/xml-tree-and-stream` names and
//! `the_reader_holds_one_window_rather_than_the_document` measures.
//!
//! # The way back out of a tree
//!
//! [`nvs_core_xml_node_source`] writes a node's subtree back out as document
//! text, so a program that walked a tree does not have to replay it into a
//! [`WRITER`] a call at a time. That is still not a path from a tree *into* the
//! writer: the writer is written to and never handed a tree, which is what
//! `a_parsed_tree_has_no_path_back_into_execution` holds, and what this member
//! answers is text.
//!
//! It writes **XML rules whichever door parsed the tree**, because a node
//! carries no memory of which parser built it and a flag to pick between the
//! two is the ambiguity `rule:core-classes/html-parsing` retired. Two things
//! follow, and both are that rule's *serialization follows the door* clause
//! read from this side. Every element is written with an end tag rather than as
//! `<a/>`, where [`crate::html`]'s serialiser leaves a void element with no end
//! tag at all: the two spellings are one document to an XML reader, so which is
//! written is this serialiser's own choice, and the long one is the one that
//! does not become an unclosed open tag swallowing the rest of the text when
//! something that is not an XML parser reads it. And a tree holding something
//! XML cannot spell — a name that is not a name, `--` inside a comment, `?>`
//! inside a processing instruction — is refused rather than written, where the
//! WHATWG algorithm recovered from all three on the way in.
//!
//! None of that makes this member a way to produce HTML. What it writes is XML,
//! read by an XML reader; a program that wants markup out of a document it
//! parsed wants `Core\Html::sanitize`, which is the launderer
//! `rule:core-classes/html-sanitize` specifies and the only member that answers
//! `Core\Html\Markup`.
//!
//! **Whitespace between elements is text**, and that is what the document says
//! rather than a hole in the tree. A pretty-printed document has a text node
//! between every pair of siblings, exactly as the XML it is says it does, and
//! dropping them would be a guess about which whitespace mattered — the guess
//! `rule:errors/ambiguous-input-refused` is the general answer to.
//!
//! # A name is as written, and what its prefix means sits beside it
//!
//! [`nvs_core_xml_node_name`] answers `x:a`: the document's own spelling,
//! prefix and all, which is what [`nvs_core_xml_node_source`] has to write back
//! out. What the prefix *means* is [`nvs_core_xml_node_namespace_uri`] — the
//! URI the nearest enclosing `xmlns:x` bound it to, the default `xmlns` for a
//! name written without a prefix, and `null` where nothing bound it or where
//! the binding is the empty one that undeclares. The `xml` prefix answers
//! [`XML_NAMESPACE`], which no document may rebind.
//!
//! **Every element resolves its own while it is being built**, and carries the
//! answer. A node holds its children and no parent, so an element has no way
//! back up to the declaration that covers it, and a parent link would be a
//! reference cycle in a refcounted tree — but both doors hold the scope on the
//! way down, [`instance_of`] as the declarations it carries into the subtree it
//! is building and a reader as the ones it keeps per open element beside their
//! names. So a lookup is a slot read, and the two doors answer alike.
//!
//! **What it spends**, per `rule:programs/memory-priority`: one slot on every
//! node, and one string per element that is in a namespace — O(nodes in the
//! document), beside a tree whose names and text already are, and released
//! with it.

use std::collections::HashSet;

use nvs_runtime::{Fault, NvsArray, NvsStr, ObjHeader, ThrownClass, Value};

use crate::registry::{
    CaseDoc, ClassDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual,
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

/// `Core\Xml\Writer`'s fully-qualified name, for [`WRITER`] and for the return
/// position that carries one.
pub(crate) const WRITER_NAME: &str = r"Core\Xml\Writer";

/// `Core\Xml`'s registry rows — § 17's three front doors, two of them the
/// stream half's: the parse that materialises a whole document, the reader that
/// walks one a node at a time, and the writer that builds one a node at a time.
/// They answer the same family and share no operation, which is
/// `rule:core-classes/xml-tree-and-stream` and the paragraph in the module doc
/// above.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
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
        CoreMethod {
            name: "writer",
            names: &[],
            params: &[CoreTy::Options(WRITER_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(WRITER_NAME),
            symbol: "nvs_core_xml_writer",
            doc: Some(&WRITER_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Xml`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reads and writes XML documents. `parse` reads a whole document into a tree of \
            `Core\\Xml\\Node` objects. `reader` returns the nodes of a document one at a time and \
            does not build a tree. `writer` builds a new document one node at a time, and escapes \
            every text for you.",
};

/// `Core\Xml::parse`'s reference card — `rule:core-api/reference-card`.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads a whole XML document and returns its document node. It replaces PHP's \
            `DOMDocument::load`, `simplexml_load_string` and `xml_parse`. A document that is not \
            well-formed throws a `ParseError`. Nothing is repaired or guessed. \
            `Core\\Html::parse` returns the same kind of tree, and repairs broken HTML instead of \
            throwing.",
    params: &[ParamDoc {
        name: "document",
        desc: "The document text. The five predefined entities, such as `&amp;`, and numeric \
               character references, such as `&#65;`, are expanded. Any other entity reference \
               throws a `ParseError`. Nothing is loaded from a file or from the network.",
        shape: &[],
    }],
    ret: "The document node. Its children are the root element and any comments or processing \
          instructions next to it. Every string you read from the tree is `tainted`, which means \
          it came from outside the program. This is true even when `$document` was not tainted.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "The document is not well-formed. For example: a tag is never closed or is closed \
               with a different name, there is more than one root element, an attribute is \
               written twice, or an entity reference is not one of the predefined ones. A \
               document type declaration also throws, and so do elements nested more than 1024 \
               levels deep.",
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
/// See [`KIND_SLOT`].
const NAMESPACE_SLOT: usize = 5;

/// `rule:core-classes/html-parsing`'s one node family, as the value a program
/// holds — the questions a walk asks, over the slots a parse filled, and no
/// static member at all, because a node is only ever produced by a parse.
///
/// Every member but one is a slot read: the document has been read by the time
/// a node exists, so there is nothing left to compute and nothing left to fail.
/// Which slots carry anything depends on the node's [`Kind`], and each member's
/// card says which — a text node has no attributes and an element carries no
/// text of its own, both of which are answers rather than errors. The exception
/// is [`nvs_core_xml_node_source`], which walks the subtree and writes it back
/// out, and is the one that can refuse.
pub(crate) const NODE: CoreClass = CoreClass {
    name: NODE_NAME,
    doc: Some(&NODE_CARD),
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
            name: "namespaceUri",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_xml_node_namespace_uri",
            doc: Some(&NAMESPACE_URI_DOC),
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
        CoreMethod {
            name: "source",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_xml_node_source",
            doc: Some(&SOURCE_DOC),
        },
    ],
    slots: &[
        "kind",
        "name",
        "text",
        "attributes",
        "children",
        "namespace",
    ],
    constants: &[],
};

/// `Core\Xml\Node`'s class card — `rule:core-api/reference-card`.
const NODE_CARD: ClassDoc = ClassDoc {
    short: "One node of a document tree. `Core\\Xml::parse` and `Core\\Html::parse` return the \
            document node, and you reach the other nodes through `children`. A node is an element, \
            a text, a comment, a processing instruction or the document itself, and `kind` tells \
            you which. Every string you read from a node is `tainted`, because it came from \
            outside the program.",
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
          which is the name a serialiser writes back out. What the prefix means is \
          `namespaceUri`. `tainted`, as everything read out of a parsed tree is.",
    errors: &[],
};

/// `Core\Xml\Node::namespaceUri`'s reference card — `rule:core-api/reference-card`.
const NAMESPACE_URI_DOC: MethodDoc = MethodDoc {
    short: "The namespace this element's name is in — the URI the nearest enclosing `xmlns:x` \
            bound its prefix to, or the one an `xmlns` bound names written without a prefix to. \
            Resolved against the declarations in scope where the element sits, so an inner \
            declaration shadows an outer one, and `name` stays the spelling the document wrote. \
            The `xml` prefix answers the URI the XML specification fixes it to, which no document \
            may rebind.",
    params: &[],
    ret: "The namespace URI, `tainted` as everything read out of a parsed tree is. `null` for a \
          node that is not an element, for an element no declaration covers, and for one under an \
          `xmlns=\"\"` that undeclared the default namespace — three answers rather than errors, \
          because a document is free to use no namespace at all.",
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
    short: "Returns the attributes of this element, in the order the document wrote them. It \
            replaces PHP's `DOMElement::getAttribute` and the `attributes` property. For a node that \
            is not an element, the array is empty.",
    params: &[],
    ret: "An array keyed by attribute name. The name is written as in the document, with its \
          prefix, such as `xml:lang`. Entities such as `&amp;` are expanded in the value. A tab or \
          a line break written inside the value becomes a space, as XML requires. The values are \
          `tainted`.",
    errors: &[],
};

/// `Core\Xml\Node::children`'s reference card — `rule:core-api/reference-card`.
const CHILDREN_DOC: MethodDoc = MethodDoc {
    short: "Returns the children of this node, in the order of the document. For the document \
            node, this is the root element and any comment or processing instruction beside it. \
            For an element, it is everything between its start tag and its end tag. A text, a \
            comment and a processing instruction have no children, so the array is empty.",
    params: &[],
    ret: "An array with one `Core\\Xml\\Node` per child. The spaces and line breaks between two \
          elements are text nodes, and they are in the array too.",
    errors: &[],
};

/// `Core\Xml\Node::source`'s reference card — `rule:core-api/reference-card`.
const SOURCE_DOC: MethodDoc = MethodDoc {
    short: "This node and everything under it, written back out as document text — the way out of \
            a walk, for a program that read a tree, decided something about it and wants the \
            document again without replaying it into a `Core\\Xml\\Writer` a call at a time. XML \
            rules, whichever door parsed the tree: every element is written with an end tag, and a \
            tree holding something XML cannot spell is refused here rather than written as \
            something a reader would read back differently.",
    params: &[],
    ret: "The subtree as text, with no XML declaration in front of it — a parse leaves none \
          behind, so writing one would be inventing the version and encoding it claims. Text and \
          attribute values are escaped, so nothing a document carried can come back out as markup. \
          `tainted`, as everything read out of a parsed tree is.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The tree holds something no XML document can spell: a name or a target that is not \
               a name, a comment holding `--` or ending in `-`, a processing instruction whose \
               data holds `?>`, or a character a document has no way to write. `Core\\Html::parse` \
               recovers from all four rather than failing, so this is where one door's recovery \
               stops being the other door's output.",
    }],
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
    doc: None,
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
    slots: &[
        "document", "cursor", "depth", "stack", "open", "rooted", "scopes",
    ],
    constants: &[],
};

/// `Core\Xml::reader`'s reference card — `rule:core-api/reference-card`.
const READER_DOC: MethodDoc = MethodDoc {
    short: "Returns a reader that goes through `$document` one node at a time. It replaces PHP's \
            `XMLReader`. Call `read` until it returns `null`. The reader does not build a tree. \
            It keeps the current node and the names of the elements that are open. This call \
            reads nothing yet. The first `read` reads the first node.",
    params: &[ParamDoc {
        name: "document",
        desc: "The document text. The reader does not copy it. Entity references are handled \
               the same way as in `Core\\Xml::parse`.",
        shape: &[],
    }],
    ret: "A reader. Its first `read` returns the first node of the document.",
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
/// See [`DOCUMENT_SLOT`].
const SCOPES_SLOT: usize = 6;

/// `Core\Xml\Writer`'s registry rows — the other half of the stream, which
/// builds a document a node at a time and never holds one to walk.
///
/// **Every one of these is a write, and none of them is a question**, which is
/// the disjointness `rule:core-classes/xml-tree-and-stream` asks for read from
/// the writing side: the tree's members answer what a node holds, and nothing
/// here answers anything until the document is finished. `content` is the
/// member the family would have called `text`, renamed rather than shared —
/// reading a node's text and writing character data are two operations, and a
/// spelling they had in common is exactly what the rule forbids.
///
/// PHP spells every construct twice, as a `start_`/`end_` pair and as a
/// `write_` shortcut. `rule:core-api/one-paradigm-per-operation` keeps one of
/// each, chosen by the construct: **the pair where it can contain other nodes**
/// — the document and an element — and **the single call where it cannot** — an
/// attribute, a comment, a CDATA section, a processing instruction and the
/// document type declaration.
pub(crate) const WRITER: CoreClass = CoreClass {
    name: WRITER_NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "startDocument",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_xml_writer_start_document",
            doc: Some(&START_DOCUMENT_DOC),
        },
        CoreMethod {
            name: "endDocument",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_xml_writer_end_document",
            doc: Some(&END_DOCUMENT_DOC),
        },
        CoreMethod {
            name: "startElement",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_xml_writer_start_element",
            doc: Some(&START_ELEMENT_DOC),
        },
        CoreMethod {
            name: "endElement",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_xml_writer_end_element",
            doc: Some(&END_ELEMENT_DOC),
        },
        CoreMethod {
            name: "content",
            names: &["text"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_xml_writer_content",
            doc: Some(&CONTENT_DOC),
        },
        CoreMethod {
            name: "attribute",
            names: &["name", "value"],
            params: &[CoreTy::Text(Qual::Launder), CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_xml_writer_attribute",
            doc: Some(&ATTRIBUTE_DOC),
        },
        CoreMethod {
            name: "comment",
            names: &["text"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_xml_writer_comment",
            doc: Some(&COMMENT_DOC),
        },
        CoreMethod {
            name: "cdata",
            names: &["text"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_xml_writer_cdata",
            doc: Some(&CDATA_DOC),
        },
        CoreMethod {
            name: "instruction",
            names: &["target", "data"],
            params: &[CoreTy::Text(Qual::Launder), CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_xml_writer_instruction",
            doc: Some(&INSTRUCTION_DOC),
        },
        CoreMethod {
            name: "doctype",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_xml_writer_doctype",
            doc: Some(&DOCTYPE_DOC),
        },
    ],
    slots: &[
        "written",
        "stack",
        "open",
        "tag",
        "attributes",
        "indent",
        "mixed",
        "state",
        "rooted",
    ],
    constants: &[],
};

/// `Core\Xml::writer`'s options — `xmlwriter_set_indent` and
/// `xmlwriter_set_indent_string` as the one thing they are.
///
/// A bag rather than two calls that mutate the writer, per
/// `rule:core-api/shape-rules` R3, which is also what removes the state PHP has
/// in which indenting is on with nothing to indent with. The default is the
/// empty string and it means no indenting at all, so the one option carries
/// both questions and there is no second one to disagree with it.
///
/// [`Qual::Neutral`] because a writer carries no qualifier at all: what this
/// answers is an object, and the option is checked to be whitespace before
/// anything holds it, so there is nothing an argument's mark could travel into.
const WRITER_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "indent",
    ty: CoreTy::Text(Qual::Neutral),
    default: Const::Str(""),
}];

/// `Core\Xml::writer`'s reference card — `rule:core-api/reference-card`.
const WRITER_DOC: MethodDoc = MethodDoc {
    short: "Returns a writer that builds an XML document one node at a time. It replaces PHP's \
            `XMLWriter`. The writer remembers which elements are open, so `endElement` needs no \
            name. Ending the document while an element is still open throws a `LogicError`. \
            Every method that takes text escapes it, so you never escape text \
            yourself.",
    params: &[ParamDoc {
        name: "indent",
        desc: "The text for one level of indentation, such as two spaces. The default is `\"\"`, \
               which means no indentation. It may contain only whitespace. The writer never adds \
               indentation next to text content, because that would change the text.",
        shape: &[],
    }],
    ret: "A new writer with an empty document. Call `startDocument` first.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`indent` contains a character that is not whitespace.",
    }],
};

/// `Core\Xml\Writer::startDocument`'s reference card —
/// `rule:core-api/reference-card`.
const START_DOCUMENT_DOC: MethodDoc = MethodDoc {
    short: "Opens the document, writing its XML declaration. A document is the outermost of the \
            two pairs, so this comes before every other write and `endDocument` closes it.",
    params: &[],
    ret: "Nothing; the declaration is written and the writer will accept content.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The document is already open, or is already finished.",
    }],
};

/// `Core\Xml\Writer::endDocument`'s reference card —
/// `rule:core-api/reference-card`.
const END_DOCUMENT_DOC: MethodDoc = MethodDoc {
    short: "Closes the document and answers it. This is where the writer refuses an unbalanced \
            tree rather than emitting one: an element still open here is an error, not a \
            document, so there is no arrangement of calls that produces text a parser would \
            refuse. Reading the document does not raise PHP's question of whether reading it also \
            empties the buffer, because what comes back is a value.",
    params: &[],
    ret: "The whole document as written, plain rather than `tainted`: every member that took \
          character data escaped it and every name was checked against XML's own, so nothing a \
          caller handed over survives as markup. Writing anything afterwards is refused.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "An element is still open, no root element was written, or the document was never \
               opened or is already finished.",
    }],
};

/// `Core\Xml\Writer::startElement`'s reference card —
/// `rule:core-api/reference-card`.
const START_ELEMENT_DOC: MethodDoc = MethodDoc {
    short: "Opens an element, which is the other of the two pairs: everything written until its \
            `endElement` is inside it. Attributes go on it until the first thing that is not one, \
            and whether it is written as `<a></a>` or `<a/>` is settled by whether anything was.",
    params: &[ParamDoc {
        name: "name",
        desc: "The element's name, qualified prefix and all — a qualified name is a name, so \
               there is no second member for a document that uses them. It has to be a name XML \
               can write, and is refused rather than escaped when it is not.",
        shape: &[],
    }],
    ret: "Nothing; the element is open and is what the next writes go into.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The name is not a name XML can write, a second root element was started, elements \
               are nested deeper than the ceiling, or the document is not open.",
    }],
};

/// `Core\Xml\Writer::endElement`'s reference card —
/// `rule:core-api/reference-card`.
const END_ELEMENT_DOC: MethodDoc = MethodDoc {
    short: "Closes the innermost open element. The name is not an argument, because the writer \
            knows what is open — which is the whole of why a mismatched close is not a shape this \
            API has.",
    params: &[],
    ret: "Nothing; an element nothing was written into is closed as `<a/>`, and one that holds \
          something as `</a>`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "Nothing is open, or the document is not open.",
    }],
};

/// `Core\Xml\Writer::content`'s reference card — `rule:core-api/reference-card`.
const CONTENT_DOC: MethodDoc = MethodDoc {
    short: "Writes character data into the open element, escaped. This is the writer's escape \
            point: `&`, `<` and `>` become references here, so an injection is not reachable by \
            forgetting a call, and there is no member that writes markup a caller assembled — \
            `rule:security/launderers-are-sink-named` is why a generic one would not be added.",
    params: &[ParamDoc {
        name: "text",
        desc: "The characters to write. `tainted` text is accepted and laundered for this one \
               sink, an XML document, because what reaches the document is the escaped form and \
               nothing a caller writes here can become markup.",
        shape: &[],
    }],
    ret: "Nothing; the open element now holds character data, and the writer stops indenting \
          inside it, since whitespace beside text changes what a document says.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "No element is open, the text holds a character XML cannot write, or the document \
               is not open.",
    }],
};

/// `Core\Xml\Writer::attribute`'s reference card —
/// `rule:core-api/reference-card`.
const ATTRIBUTE_DOC: MethodDoc = MethodDoc {
    short: "Writes one attribute on the element that was just opened. A single call rather than a \
            pair, because an attribute holds a value and cannot contain nodes — the pair PHP has \
            exists only to let text be written between its halves.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The attribute's name, qualified prefix and all, and refused when it is not a \
                   name XML can write.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "Its value, escaped into the quotes it is written between — the quote itself \
                   and the whitespace a parser would otherwise fold included, so what comes back \
                   out of a parse is what was written. `tainted` text is laundered for this sink \
                   exactly as `content`'s is.",
            shape: &[],
        },
    ],
    ret: "Nothing; the attribute is on the open start tag.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "No start tag is still taking attributes, the element already carries an attribute \
               of that name, the name is not a name XML can write, the value holds a character \
               XML cannot write, or the document is not open.",
    }],
};

/// `Core\Xml\Writer::comment`'s reference card — `rule:core-api/reference-card`.
const COMMENT_DOC: MethodDoc = MethodDoc {
    short: "Writes a comment, as one call: a comment's content is text, so there is nothing for a \
            pair to contain.",
    params: &[ParamDoc {
        name: "text",
        desc: "The comment's content. A comment is the one place XML has no escape grammar for, \
               so a `--` inside it or a trailing `-` is refused rather than rewritten — \
               `rule:errors/ambiguous-input-refused` is the general shape of that answer.",
        shape: &[],
    }],
    ret: "Nothing; the comment is written where the writer stands, inside the open element or \
          beside the root.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The text holds `--`, ends with `-`, holds a character XML cannot write, or the \
               document is not open.",
    }],
};

/// `Core\Xml\Writer::cdata`'s reference card — `rule:core-api/reference-card`.
const CDATA_DOC: MethodDoc = MethodDoc {
    short: "Writes character data as a CDATA section. One call, because a CDATA section is an \
            escaping choice about text and is written with the text it is a choice about — a \
            parse answers the same `Text` node either way.",
    params: &[ParamDoc {
        name: "text",
        desc: "The characters to write. A CDATA section has no escape grammar inside it, so a \
               `]]>` in the text is refused rather than split across two sections.",
        shape: &[],
    }],
    ret: "Nothing; the open element now holds character data, exactly as `content` leaves it.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "No element is open, the text holds `]]>` or a character XML cannot write, or the \
               document is not open.",
    }],
};

/// `Core\Xml\Writer::instruction`'s reference card —
/// `rule:core-api/reference-card`.
const INSTRUCTION_DOC: MethodDoc = MethodDoc {
    short: "Writes a processing instruction, as one call: it is a target and its data, both text, \
            with nothing to nest inside it.",
    params: &[
        ParamDoc {
            name: "target",
            desc: "What the instruction is addressed to. `xml` in any casing is refused, because \
                   that target is the XML declaration `startDocument` already wrote.",
            shape: &[],
        },
        ParamDoc {
            name: "data",
            desc: "The instruction's data, written as it stands — an instruction has no escape \
                   grammar, so a `?>` inside it is refused. The empty string writes the target \
                   alone.",
            shape: &[],
        },
    ],
    ret: "Nothing; the instruction is written where the writer stands.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The target is not a name XML can write or is `xml`, the data holds `?>` or a \
               character XML cannot write, or the document is not open.",
    }],
};

/// `Core\Xml\Writer::doctype`'s reference card — `rule:core-api/reference-card`.
const DOCTYPE_DOC: MethodDoc = MethodDoc {
    short: "Writes a document type declaration naming `$name`, before the root element. Naming a \
            document type is not resolving one: this takes no external identifier and no internal \
            subset, so nothing it writes declares an entity or points at one. It is written for a \
            reader outside Novis, because `Core\\Xml::parse` and `Core\\Xml::reader` refuse a \
            `<!DOCTYPE …>` whole — a document carrying one is the one thing this class writes and \
            will not read back.",
    params: &[ParamDoc {
        name: "name",
        desc: "The document type's name, which is the root element's name in every document that \
               a validator would accept.",
        shape: &[],
    }],
    ret: "Nothing; the declaration is written above the root element.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The root element is already open or written, the name is not a name XML can write, \
               or the document is not open.",
    }],
};

/// [`WRITER`]'s slots, in declaration order: the chunks of the document written
/// so far …
const WRITTEN_SLOT: usize = 0;
/// … the names of the elements open around the writer …
const OPEN_NAMES_SLOT: usize = 1;
/// … how many of those names are live …
const OPEN_COUNT_SLOT: usize = 2;
/// … whether a start tag is written and still able to take attributes …
const TAG_SLOT: usize = 3;
/// … the names that tag already carries …
const TAG_NAMES_SLOT: usize = 4;
/// … what one level of nesting is indented by …
const INDENT_SLOT: usize = 5;
/// … the depth at which character data was written into an element still open,
/// or `0` for none …
const MIXED_SLOT: usize = 6;
/// … which of [`BEFORE`], [`WRITING`] and [`FINISHED`] the writer is in …
const STATE_SLOT: usize = 7;
/// … and whether the root element has been written.
const ROOT_SLOT: usize = 8;

/// [`STATE_SLOT`] before `startDocument`, when the writer holds nothing.
const BEFORE: usize = 0;
/// [`STATE_SLOT`] while the document is open and takes writes.
const WRITING: usize = 1;
/// [`STATE_SLOT`] once `endDocument` has answered, after which nothing is
/// written and nothing is answered again.
const FINISHED: usize = 2;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_xml_parse" => (nvs_core_xml_parse as *const ()).cast(),
        "nvs_core_xml_node_kind" => (nvs_core_xml_node_kind as *const ()).cast(),
        "nvs_core_xml_node_name" => (nvs_core_xml_node_name as *const ()).cast(),
        "nvs_core_xml_node_namespace_uri" => (nvs_core_xml_node_namespace_uri as *const ()).cast(),
        "nvs_core_xml_node_text" => (nvs_core_xml_node_text as *const ()).cast(),
        "nvs_core_xml_node_attributes" => (nvs_core_xml_node_attributes as *const ()).cast(),
        "nvs_core_xml_node_children" => (nvs_core_xml_node_children as *const ()).cast(),
        "nvs_core_xml_node_source" => (nvs_core_xml_node_source as *const ()).cast(),
        "nvs_core_xml_reader" => (nvs_core_xml_reader as *const ()).cast(),
        "nvs_core_xml_reader_read" => (nvs_core_xml_reader_read as *const ()).cast(),
        "nvs_core_xml_reader_depth" => (nvs_core_xml_reader_depth as *const ()).cast(),
        "nvs_core_xml_writer" => (nvs_core_xml_writer as *const ()).cast(),
        "nvs_core_xml_writer_start_document" => {
            (nvs_core_xml_writer_start_document as *const ()).cast()
        }
        "nvs_core_xml_writer_end_document" => {
            (nvs_core_xml_writer_end_document as *const ()).cast()
        }
        "nvs_core_xml_writer_start_element" => {
            (nvs_core_xml_writer_start_element as *const ()).cast()
        }
        "nvs_core_xml_writer_end_element" => (nvs_core_xml_writer_end_element as *const ()).cast(),
        "nvs_core_xml_writer_content" => (nvs_core_xml_writer_content as *const ()).cast(),
        "nvs_core_xml_writer_attribute" => (nvs_core_xml_writer_attribute as *const ()).cast(),
        "nvs_core_xml_writer_comment" => (nvs_core_xml_writer_comment as *const ()).cast(),
        "nvs_core_xml_writer_cdata" => (nvs_core_xml_writer_cdata as *const ()).cast(),
        "nvs_core_xml_writer_instruction" => (nvs_core_xml_writer_instruction as *const ()).cast(),
        "nvs_core_xml_writer_doctype" => (nvs_core_xml_writer_doctype as *const ()).cast(),
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
///
/// **This type is the boundary `rule:core-classes/html-parsing`'s one-node-family
/// clause is enforced at.** [`crate::html`]'s WHATWG parse builds these and
/// nothing else, so the two doors cannot drift into two families: a node HTML
/// could produce that XML could not would have to be a sixth [`Kind`], and
/// there is no sixth. It is `pub(crate)` for exactly that one caller.
#[derive(Debug)]
pub(crate) struct Parsed {
    /// Which of the five this is.
    pub(crate) kind: Kind,
    /// An element's tag name or a processing instruction's target.
    pub(crate) name: String,
    /// The character data this node carries itself.
    pub(crate) text: String,
    /// An element's attributes, in written order.
    pub(crate) attributes: Vec<(String, String)>,
    /// This node's children, in document order.
    pub(crate) children: Vec<Parsed>,
}

impl Parsed {
    /// An empty node of `kind` — every field is filled by the reader that
    /// produced it, and the ones a kind does not use stay empty.
    pub(crate) fn new(kind: Kind) -> Self {
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
    if forbidden(ch) {
        return Err(refused);
    }
    Ok(ch)
}

/// Whether XML has no way to write `ch`: a control character other than the
/// three whitespace ones. This is the one test for it — a reference naming one
/// is refused by [`numeric`], a literal one by [`refused_literal`]'s callers, and a
/// writer that would emit one by [`unwritable`] — so the parser never accepts
/// a character the writers refuse to write back out.
fn forbidden(ch: char) -> bool {
    ch.is_control() && !matches!(ch, '\t' | '\r' | '\n')
}

/// The sentence for a literal [`forbidden`] character in the document.
fn refused_literal(ch: char) -> String {
    format!(
        "the document holds U+{:04X}, which XML does not allow",
        u32::from(ch)
    )
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
    /// The namespace declarations each of those elements wrote, at the same
    /// index as its name and as the attributes it wrote them as — the scope a
    /// node read next sits in, which a tree door instead carries down the
    /// [`Building`] stack.
    scopes: &'a mut NvsArray,
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

    /// Opens `node`, keeping its name and the namespace declarations it wrote.
    ///
    /// The write cannot move either array: the reader owns the only reference
    /// to its own stack, since no member hands it out, so
    /// [`NvsArray::set_index`]'s copy-on-write separation never fires and the
    /// handle stays the one the slot names.
    fn push(&mut self, node: &Parsed) {
        let at = i64::try_from(self.depth).expect("`DEPTH_CEILING` is far under `i64::MAX`");
        self.names
            .set_index(at, Value::str(NvsStr::new(node.name.as_bytes())));
        let mut declarations = NvsArray::new();
        for (attribute, value) in &node.attributes {
            if declared(attribute).is_some() {
                declarations.set(
                    NvsStr::new(attribute.as_bytes()),
                    Value::str(NvsStr::new(value.as_bytes())),
                );
            }
        }
        self.scopes.set_index(at, Value::array(declarations));
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

/// What [`Reader::afford`] refuses with. It never reaches a program as a
/// `ParseError`: [`nvs_core_xml_parse`] reports it as the memory-limit `FATAL`
/// `rule:errors/on-limit` makes every breach.
const UNAFFORDABLE: &str = "the document does not fit in the request's memory limit";

impl<'a> Reader<'a> {
    /// Refuses once the request holds more than its memory limit allows.
    ///
    /// The tree a parse builds is Rust memory, which the allocator counts and
    /// never refuses, so without this a document of many small nodes grows the
    /// request far past its ceiling before the first Novis allocation notices.
    /// Asked once per node and once per attribute, and each ask is one
    /// thread-local compare.
    fn afford(&self) -> Result<(), String> {
        match nvs_runtime::affordable(Some(size_of::<Parsed>()), "Core\\Xml::parse") {
            Ok(_) => Ok(()),
            Err(_) => Err(UNAFFORDABLE.to_owned()),
        }
    }

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
        if let Some(ch) = rest[..end].chars().find(|&ch| forbidden(ch)) {
            return Err(refused_literal(ch));
        }
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

    /// One attribute value, quotes and references consumed, and normalised as
    /// XML 1.0 § 3.3.3 says a value with no declared type is: a tab, a line
    /// feed or a carriage return written literally reads as one space, and a
    /// `\r\n` pair as one space rather than two. A character reference is not
    /// normalised, so `&#9;` stays a tab — which is how [`quoted`] writes one
    /// back out.
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
            if forbidden(ch) {
                return Err(refused_literal(ch));
            }
            self.pos += ch.len_utf8();
            if ch == '\r' && self.rest().starts_with('\n') {
                self.pos += 1;
            }
            out.push(if matches!(ch, '\t' | '\n' | '\r') {
                ' '
            } else {
                ch
            });
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
        /// How many attributes a tag may carry before [`Self::start_tag`]
        /// checks for a repeated name with a set instead of a scan.
        const SCANNED: usize = 16;
        self.pos += '<'.len_utf8();
        let mut node = Parsed::new(Kind::Element);
        node.name = self.name("an opening tag")?;
        // A tag with few attributes is checked for a repeated name by scanning
        // them; past `SCANNED` the names also go into a set, so a tag written
        // with a great many attributes costs linear time and not quadratic.
        // What that spends is one copy of each name of a wide tag, freed when
        // the tag is read.
        let mut names: Option<HashSet<String>> = None;
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
            self.afford()?;
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
            let twice = if node.attributes.len() < SCANNED {
                node.attributes.iter().any(|(seen, _)| *seen == attr)
            } else {
                let names = names.get_or_insert_with(|| {
                    node.attributes
                        .iter()
                        .map(|(seen, _)| seen.clone())
                        .collect()
                });
                !names.insert(attr.clone())
            };
            if twice {
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
            if forbidden(ch) {
                return Err(refused_literal(ch));
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
            self.afford()?;
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
                    open.push(&node);
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
            self.afford()?;
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

/// The namespace the XML specification binds the `xml` prefix to for good: a
/// document may neither declare it nor rebind it, so an `xml:lang` is in this
/// namespace wherever it is written and whatever is in scope around it.
pub(crate) const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// The prefix `attribute` declares a namespace for — `""` where it is the
/// `xmlns` that binds names written without one, the prefix itself for an
/// `xmlns:x`, and `None` for an attribute that declares nothing.
fn declared(attribute: &str) -> Option<&str> {
    match attribute.strip_prefix("xmlns") {
        Some("") => Some(""),
        Some(rest) => rest.strip_prefix(':').filter(|prefix| !prefix.is_empty()),
        None => None,
    }
}

/// The namespace `name` is in under `scope`, whose entries are the declarations
/// that cover it as `(prefix, uri)` pairs, outermost first.
///
/// The innermost declaration of a prefix wins, which is what makes a nested
/// `xmlns` a rebinding rather than a conflict. An empty URI is the `xmlns=""`
/// that undeclares the default namespace, and answers as no namespace rather
/// than as one whose name is the empty string.
fn resolved(scope: &[(String, String)], name: &str) -> Option<String> {
    let prefix = name.split_once(':').map_or("", |(prefix, _)| prefix);
    if prefix == "xml" {
        return Some(XML_NAMESPACE.to_owned());
    }
    scope
        .iter()
        .rev()
        .find(|(declared, _)| declared.as_str() == prefix)
        .map(|(_, uri)| uri.clone())
        .filter(|uri| !uri.is_empty())
}

/// One node part-way through becoming a [`NODE`] instance: the node itself with
/// its children taken off it, the ones still to be built, the ones already
/// built, and what the scope it was opened in said about its name.
///
/// [`instance_of`]'s stack frame, in the heap where it costs nothing but bytes.
#[derive(Debug)]
struct Building {
    /// The node, with [`Parsed::children`] emptied into [`Self::pending`].
    node: Parsed,
    /// The children not yet built, innermost-last so a pop takes the next one
    /// in document order.
    pending: Vec<Parsed>,
    /// The children already built, in document order.
    built: NvsArray,
    /// The namespace this node's name resolved to, read while the walk was
    /// standing on it and the scope was its own.
    namespace: Option<String>,
    /// How many declarations were in scope before this node's own were pushed,
    /// which is what the scope is cut back to once it is built.
    mark: usize,
}

impl Building {
    /// `node` with its children moved into the two lists, its own declarations
    /// pushed onto `scope`, and its namespace read off what that leaves.
    ///
    /// The stack of these frames is the ancestor chain at every moment, so the
    /// scope beside it is exactly the declarations covering the node being
    /// opened — which is why an element resolves here rather than by walking
    /// for a parent it does not hold.
    fn opened(mut node: Parsed, scope: &mut Vec<(String, String)>) -> Self {
        let mark = scope.len();
        let mut namespace = None;
        if node.kind == Kind::Element {
            for (attribute, value) in &node.attributes {
                if let Some(prefix) = declared(attribute) {
                    scope.push((prefix.to_owned(), value.clone()));
                }
            }
            namespace = resolved(scope, &node.name);
        }
        let mut pending = std::mem::take(&mut node.children);
        pending.reverse();
        Self {
            node,
            pending,
            built: NvsArray::new(),
            namespace,
            mark,
        }
    }
}

/// One [`Parsed`] subtree as the [`NODE`] instance a program holds, under the
/// namespace declarations `inherited` from wherever it is being spliced in —
/// none for a whole document, and the open elements' own for the one node a
/// reader answers with.
///
/// **Iterative, over a heap stack rather than the native one**, and that is the
/// whole reason it is written this way: [`DEPTH_CEILING`] is a thousand and a
/// task's stack is `nvs_host::TASK_STACK_SIZE`, which a frame per node exhausts
/// before the ceiling is reached — so a document nested a thousand deep took
/// the process down instead of being refused, which is a crash rather than the
/// bound. `Vec` grows on the heap, where the request's own memory limit already
/// accounts for it.
///
/// Depth is still bounded, by the reader that refused a document past the
/// ceiling and by [`crate::html`]'s parse flattening one past it — this
/// function is simply no longer the thing that has to survive the bound being
/// wrong.
pub(crate) fn instance_of(node: Parsed, inherited: &[(String, String)]) -> Value {
    let mut scope = inherited.to_vec();
    let mut stack = vec![Building::opened(node, &mut scope)];
    loop {
        let top = stack
            .last_mut()
            .expect("the loop returns the moment the last frame is popped");
        if let Some(next) = top.pending.pop() {
            let frame = Building::opened(next, &mut scope);
            stack.push(frame);
            continue;
        }
        let done = stack
            .pop()
            .expect("the frame just read back is still the last one");
        scope.truncate(done.mark);
        let value = built(done.node, done.namespace, done.built);
        match stack.last_mut() {
            Some(parent) => parent.built.append(value),
            None => return value,
        }
    }
}

/// One node with its children already built, as the [`NODE`] instance.
fn built(node: Parsed, namespace: Option<String>, children: NvsArray) -> Value {
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
            namespace.map_or_else(Value::null, |uri| Value::str(NvsStr::new(uri.as_bytes()))),
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

/// Slot `index` of a reader or a writer, as the count it holds.
///
/// # Errors
///
/// A `Fault::fatal` for a slot holding anything else, which only a bug in this
/// module can produce: those slots are written by the two constructors and by
/// the members that step them, and by nothing else.
fn count_slot(
    class: &CoreClass,
    receiver: *mut ObjHeader,
    index: usize,
    member: &str,
) -> Result<usize, Fault> {
    let held = crate::instance::slot(receiver, index);
    held.as_uint()
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| wrong_slot(class, member, index, held))
}

/// The fault for a slot holding something this module did not write there.
fn wrong_slot(class: &CoreClass, member: &str, index: usize, held: Value) -> Fault {
    Fault::fatal(format!(
        "{}::{member} found tag {} in its `{}` slot",
        class.name,
        held.tag_byte(),
        class.slots[index]
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
        .ok_or_else(|| wrong_slot(&READER, "read", DOCUMENT_SLOT, held))?;
    let held = crate::instance::slot(object, ROOTED_SLOT);
    let mut rooted = held
        .as_bool()
        .ok_or_else(|| wrong_slot(&READER, "read", ROOTED_SLOT, held))?;
    let held = crate::instance::slot(object, STACK_SLOT);
    let stack = held
        .array_ptr()
        .ok_or_else(|| wrong_slot(&READER, "read", STACK_SLOT, held))?;
    let mut stack = crate::arr::borrowed(stack);
    let held = crate::instance::slot(object, SCOPES_SLOT);
    let scopes = held
        .array_ptr()
        .ok_or_else(|| wrong_slot(&READER, "read", SCOPES_SLOT, held))?;
    let mut scopes = crate::arr::borrowed(scopes);
    let mut open = Open {
        names: &mut stack,
        scopes: &mut scopes,
        depth: count_slot(&READER, object, OPEN_SLOT, "read")?,
    };

    let mut scan = Reader::at(document, count_slot(&READER, object, CURSOR_SLOT, "read")?);
    let stepped = scan.step(&mut open, &mut rooted).map_err(|why| {
        // A breach is the limit's `FATAL`, as it is for the parse.
        if why == UNAFFORDABLE {
            return Fault::fatal(format!("{READER_NAME}::read(): {why}."));
        }
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
    let carried = inherited(open.scopes, at);
    Ok(instance_of(node, &carried))
}

/// The namespace declarations the elements open above `depth` wrote, outermost
/// first — the scope a node a reader has just read sits in.
///
/// A streaming walk holds its ancestors where a tree door holds its descendants
/// (`rule:core-classes/xml-tree-and-stream`), so this is the same list
/// [`Building::opened`] carries down a tree, read back off the slots a reader
/// keeps it in between two calls.
fn inherited(scopes: &NvsArray, depth: usize) -> Vec<(String, String)> {
    let mut carried = Vec::new();
    for open in 0..depth {
        let at = i64::try_from(open).expect("`DEPTH_CEILING` is far under `i64::MAX`");
        let Some(held) = scopes.get_index(at) else {
            continue;
        };
        let Some(declarations) = held.array_ptr() else {
            continue;
        };
        let declarations = crate::arr::borrowed(declarations);
        let mut from = 0usize;
        while let Some(slot) = declarations.next_slot(from) {
            from = slot + 1;
            let Some(key) = declarations.key_at(slot) else {
                continue;
            };
            let Ok(attribute) = std::str::from_utf8(key.as_bytes()) else {
                continue;
            };
            let uri = declarations
                .value_at(slot)
                .and_then(|value| value.as_text().map(str::to_owned));
            if let (Some(prefix), Some(uri)) = (declared(attribute), uri) {
                carried.push((prefix.to_owned(), uri));
            }
        }
    }
    carried
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
    fn nvs_core_xml_parse(ctx, args: [1]) {
        // unreachable from source: the parameter is `CoreTy::Text`, so anything
        // that is not a `string` is `E0401` at the call site.
        let Some(document) = args[0].as_text() else {
            return Err(Fault::fatal(format!(
                "Core\\Xml::parse expected a `string`, got tag {}",
                args[0].tag_byte()
            )));
        };
        match parse(document) {
            Ok(tree) => Ok(instance_of(tree, &[])),
            // A breach is the limit's `FATAL`, never a `ParseError` a `catch`
            // could swallow and carry on past.
            Err(why) if why == UNAFFORDABLE => Err(ctx
                .memory_breach()
                .unwrap_or_else(|| Fault::fatal(format!("{NAME}::parse(): {why}.")))),
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
    /// `$node->namespaceUri(): ?tainted string` — the namespace this element's
    /// name is in, or `null` where nothing put it in one.
    ///
    /// A slot read like its siblings: the scope an element sits in is known
    /// while the tree is being built and is gone afterwards, so the answer is
    /// resolved there rather than walked for here (the module's § *A name is as
    /// written*).
    fn nvs_core_xml_node_namespace_uri(_ctx, args: [1]) {
        held(args[0], NAMESPACE_SLOT, "namespaceUri")
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

/// One step of [`source`]'s walk: a node whose own text is not written yet, or
/// an end tag waiting under the children it closes.
enum Step {
    /// A node still to be written, and its children after it.
    Node(Value),
    /// Text to append once everything pushed over it has been written.
    Closed(String),
}

/// The refusal [`source`] makes for a tree with no XML spelling: spec § 10's
/// `LogicError`, which is what `Core\Json::encode` already refuses a value it
/// cannot write with.
///
/// # Errors
///
/// Always — this is the error, and the `Result` is so a caller writes
/// `return refused(…)` rather than wrapping it.
fn refused<T>(why: &str) -> Result<T, Fault> {
    Err(Fault::thrown_as(
        ThrownClass::Logic,
        // The sentence is the caller's and the period is here, exactly as the
        // parse's and the writer's are.
        format!("{NODE_NAME}::source(): {why}."),
    ))
}

/// `text` refused if it holds a character a document cannot write.
///
/// # Errors
///
/// A `LogicError` naming the character, which is [`writable`]'s refusal made
/// from the other door: the rule about what a document can carry is the
/// module's and not either writer's.
fn carried(text: &str) -> Result<(), Fault> {
    match unwritable(text) {
        Some(ch) => refused(&format!(
            "the text holds U+{:04X}, which a document has no way to write",
            u32::from(ch)
        )),
        None => Ok(()),
    }
}

/// One node's subtree as the document text a parse reads this same tree back
/// out of.
///
/// **Iterative, over a heap stack rather than the native one**, for
/// [`instance_of`]'s reason: a frame per node exhausts a task's stack before
/// [`DEPTH_CEILING`] is reached, so a document deep enough to be interesting
/// would take the process down instead of being written.
///
/// **The XML rules are the writer's own** — [`is_name`], [`escaped`] and
/// [`quoted`] are shared with it — so the two ways a document leaves this
/// module spell one thing one way. What differs from [`crate::html`]'s
/// serialiser is what `rule:core-classes/html-parsing` says it should, and the
/// module doc's *the way back out of a tree* is where both differences are
/// stated: an end tag on every element, and a refusal where that parser
/// recovered.
///
/// What it spends: the text it is building, plus one entry per node on the path
/// from the root to where the walk is, and the children of the node it is
/// looking at.
///
/// # Errors
///
/// A `LogicError` for a tree with no XML spelling, and a `Fault::fatal` for a
/// slot this module did not write.
fn source(node: Value) -> Result<String, Fault> {
    let mut out = String::new();
    let mut steps = vec![Step::Node(node)];
    while let Some(step) = steps.pop() {
        let node = match step {
            Step::Closed(tag) => {
                out.push_str(&tag);
                continue;
            }
            Step::Node(node) => node,
        };
        let receiver = crate::instance::receiver(node, &NODE, "source")?;
        let held = crate::instance::slot(receiver, KIND_SLOT);
        let kind = held
            .as_int()
            .and_then(|ordinal| Kind::ALL.into_iter().find(|kind| kind.ordinal() == ordinal))
            .ok_or_else(|| wrong_slot(&NODE, "source", KIND_SLOT, held))?;
        let named = crate::instance::slot(receiver, NAME_SLOT);
        let name = named
            .as_text()
            .ok_or_else(|| wrong_slot(&NODE, "source", NAME_SLOT, named))?;
        let carries = crate::instance::slot(receiver, TEXT_SLOT);
        let text = carries
            .as_text()
            .ok_or_else(|| wrong_slot(&NODE, "source", TEXT_SLOT, carries))?;

        match kind {
            // A document is its children and nothing of its own: a parse leaves
            // no declaration behind, so there is none to write back.
            Kind::Document => {}
            Kind::Element => {
                if !is_name(name) {
                    return refused(&format!("`{name}` is not a name a document can write"));
                }
                out.push('<');
                out.push_str(name);
                let attributes = crate::instance::slot(receiver, ATTRIBUTES_SLOT);
                let pairs = attributes
                    .array_ptr()
                    .ok_or_else(|| wrong_slot(&NODE, "source", ATTRIBUTES_SLOT, attributes))?;
                let pairs = crate::arr::borrowed(pairs);
                let mut from = 0usize;
                while let Some(at) = pairs.next_slot(from) {
                    from = at + 1;
                    let key = pairs
                        .key_at(at)
                        .ok_or_else(|| wrong_slot(&NODE, "source", ATTRIBUTES_SLOT, attributes))?;
                    let attribute = std::str::from_utf8(key.as_bytes())
                        .map_err(|_| wrong_slot(&NODE, "source", ATTRIBUTES_SLOT, attributes))?;
                    let held = pairs
                        .value_at(at)
                        .expect("next_slot only names live entries");
                    let value = held
                        .as_text()
                        .ok_or_else(|| wrong_slot(&NODE, "source", ATTRIBUTES_SLOT, held))?;
                    if !is_name(attribute) {
                        return refused(&format!(
                            "`{attribute}` is not a name a document can write"
                        ));
                    }
                    carried(value)?;
                    out.push(' ');
                    out.push_str(attribute);
                    out.push_str("=\"");
                    out.push_str(&quoted(value));
                    out.push('"');
                }
                out.push('>');
                steps.push(Step::Closed(format!("</{name}>")));
            }
            Kind::Text => {
                carried(text)?;
                out.push_str(&escaped(text));
            }
            Kind::Comment => {
                if text.contains("--") || text.ends_with('-') {
                    return refused(
                        "a comment has no escape grammar, so it cannot hold `--` or end with `-`",
                    );
                }
                carried(text)?;
                out.push_str("<!--");
                out.push_str(text);
                out.push_str("-->");
            }
            Kind::ProcessingInstruction => {
                if !is_name(name) {
                    return refused(&format!("`{name}` is not a target a document can write"));
                }
                if text.contains("?>") {
                    return refused(
                        "a processing instruction has no escape grammar, so its data cannot hold \
                         `?>`",
                    );
                }
                carried(text)?;
                out.push_str("<?");
                out.push_str(name);
                if !text.is_empty() {
                    out.push(' ');
                    out.push_str(text);
                }
                out.push_str("?>");
            }
        }

        if matches!(kind, Kind::Document | Kind::Element) {
            let children = crate::instance::slot(receiver, CHILDREN_SLOT);
            let under = children
                .array_ptr()
                .ok_or_else(|| wrong_slot(&NODE, "source", CHILDREN_SLOT, children))?;
            let under = crate::arr::borrowed(under);
            let mut order = Vec::with_capacity(under.count());
            let mut from = 0usize;
            while let Some(at) = under.next_slot(from) {
                from = at + 1;
                order.push(
                    under
                        .value_at(at)
                        .expect("next_slot only names live entries"),
                );
            }
            // Reversed, so a pop takes the next child in document order.
            steps.extend(order.into_iter().rev().map(Step::Node));
        }
    }
    Ok(out)
}

nvs_runtime::nvs_helper! {
    /// `$node->source(): tainted string` — this subtree as the document text it
    /// was written as.
    ///
    /// The way back out of a walk, and the tree half's own: [`WRITER`] is
    /// written to a call at a time and is never handed a tree, which is what
    /// `a_parsed_tree_has_no_path_back_into_execution` holds.
    fn nvs_core_xml_node_source(_ctx, args: [1]) {
        let written = source(args[0])?;
        Ok(Value::str(NvsStr::new(written.as_bytes())))
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
                Value::array(NvsArray::new()),
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

// ============================================================================
// The writer
// ============================================================================

/// Whether `name` is one a document can carry — XML's `Name` production, as
/// [`is_name_start`] and [`is_name_char`] already spell it for the parser, so
/// the writer refuses exactly the names the reader would refuse to read back.
fn is_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(is_name_start) && chars.all(is_name_char)
}

/// `text` with the three characters that would otherwise be markup written as
/// the references XML reads them back from.
///
/// `>` is only markup after `]]`, and escaping it unconditionally is what stops
/// a text ending in `]]` from closing a section a later reader is inside — the
/// cheaper rule, and the one every serialiser that has ever been read by
/// another one converged on.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

/// `text` as an attribute value between double quotes: [`escaped`]'s three,
/// plus the quote itself and the whitespace a parser folds to a space, so what
/// comes back out of a parse is the value that went in.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            _ => out.push(ch),
        }
    }
    out
}

/// A writer's slots, reached as the state they are.
///
/// Every accessor here reads and writes straight through to the receiver rather
/// than keeping a copy, so there is no arrangement in which a refusal has
/// already moved half the state: a member decides on everything it is going to
/// write before it writes any of it. A writer holds no native allocation, for
/// the reason a reader holds none — the pieces of the document written so far
/// are a Novis array, the names of the elements open around it are another, and
/// the rest is counts and flags.
///
/// What it spends, per `rule:programs/memory-priority`: the document it is
/// building, which is what a writer is for and what `endDocument` hands back.
/// What it does **not** hold is a tree of nodes — nothing here is walkable, and
/// nothing written is readable until the document is finished — which is the
/// half of `rule:core-classes/xml-tree-and-stream`'s memory story a writer can
/// keep.
struct Pen {
    /// The receiver these slots belong to.
    object: *mut ObjHeader,
    /// The member reading them, for the sentence a refusal carries.
    member: &'static str,
}

impl Pen {
    /// The writer `receiver` names, for `member`.
    ///
    /// # Errors
    ///
    /// A `Fault::fatal` for a receiver that is not one of [`WRITER`]'s
    /// instances, which the compile-time signature rules out.
    fn of(receiver: Value, member: &'static str) -> Result<Self, Fault> {
        let object = crate::instance::receiver(receiver, &WRITER, member)?;
        Ok(Self { object, member })
    }

    /// The refusal this member makes: spec § 10's `LogicError`, which is the
    /// class for a program that asked for something its own state forbids.
    ///
    /// # Errors
    ///
    /// Always — this is the error, and the `Result` is so a caller writes
    /// `return pen.refuse(…)` rather than wrapping it.
    fn refuse<T>(&self, why: &str) -> Result<T, Fault> {
        Err(Fault::thrown_as(
            ThrownClass::Logic,
            // The sentence is the member's and the period is here, exactly as
            // the parse's is, so no message has to remember to end like one.
            format!("{WRITER_NAME}::{}(): {why}.", self.member),
        ))
    }

    /// Count slot `index`.
    ///
    /// # Errors
    ///
    /// A `Fault::fatal` for a slot holding anything else — a bug in this
    /// module, since nothing outside it writes one.
    fn count(&self, index: usize) -> Result<usize, Fault> {
        count_slot(&WRITER, self.object, index, self.member)
    }

    /// Flag slot `index`.
    ///
    /// # Errors
    ///
    /// [`Self::count`]'s.
    fn flag(&self, index: usize) -> Result<bool, Fault> {
        let held = crate::instance::slot(self.object, index);
        held.as_bool()
            .ok_or_else(|| wrong_slot(&WRITER, self.member, index, held))
    }

    /// Writes a count into slot `index`.
    fn set_count(&self, index: usize, count: usize) {
        crate::instance::set_slot(self.object, index, counted(count));
    }

    /// Writes a flag into slot `index`.
    fn set_flag(&self, index: usize, flag: bool) {
        crate::instance::set_slot(self.object, index, Value::bool(flag));
    }

    /// Array slot `index`, borrowed.
    ///
    /// # Errors
    ///
    /// [`Self::count`]'s.
    fn array(&self, index: usize) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
        let held = crate::instance::slot(self.object, index);
        let array = held
            .array_ptr()
            .ok_or_else(|| wrong_slot(&WRITER, self.member, index, held))?;
        Ok(crate::arr::borrowed(array))
    }

    /// Refuses unless the document is open.
    ///
    /// # Errors
    ///
    /// A `LogicError` before `startDocument` and after `endDocument` alike: the
    /// document is the outermost pair, so every other member is inside it.
    fn writing(&self) -> Result<(), Fault> {
        match self.count(STATE_SLOT)? {
            WRITING => Ok(()),
            BEFORE => self.refuse("the document is not open, and `startDocument` is what opens it"),
            _ => self.refuse("the document is finished, and `endDocument` has answered it"),
        }
    }

    /// Appends `chunk` to what the document holds.
    ///
    /// The pieces are kept apart and joined once, in [`end_document`], so no
    /// write copies what was written before it. Each piece is a string of its
    /// own, so until the join a document of many small writes holds many times
    /// its own size: gap `nvs-stdlib/an-xml-writer-holds-a-string-per-piece`.
    /// The write cannot move the array for [`Open::push`]'s
    /// reason: no member hands a writer's own arrays out, so
    /// [`NvsArray::set_index`]'s copy-on-write separation never fires and the
    /// handle stays the one the slot names.
    ///
    /// # Errors
    ///
    /// [`Self::count`]'s.
    fn emit(&self, chunk: &str) -> Result<(), Fault> {
        let mut written = self.array(WRITTEN_SLOT)?;
        let at =
            i64::try_from(written.count()).expect("a document's piece count is under `i64::MAX`");
        written.set_index(at, Value::str(NvsStr::new(chunk.as_bytes())));
        Ok(())
    }

    /// Closes a start tag that is still taking attributes, if one is open.
    ///
    /// # Errors
    ///
    /// [`Self::count`]'s.
    fn seal(&self) -> Result<(), Fault> {
        if self.flag(TAG_SLOT)? {
            self.emit(">")?;
            self.set_flag(TAG_SLOT, false);
        }
        Ok(())
    }

    /// Writes the line break and indentation a node at `depth` gets.
    ///
    /// Nothing at all when the writer does not indent, and nothing once
    /// character data has reached an element that is still open: whitespace
    /// beside text is text, so indenting there would change what the document
    /// says rather than how it looks.
    ///
    /// # Errors
    ///
    /// [`Self::count`]'s.
    fn lead(&self, depth: usize) -> Result<(), Fault> {
        let held = crate::instance::slot(self.object, INDENT_SLOT);
        let indent = held
            .as_text()
            .ok_or_else(|| wrong_slot(&WRITER, self.member, INDENT_SLOT, held))?;
        if indent.is_empty() || self.count(MIXED_SLOT)? != 0 {
            return Ok(());
        }
        let mut lead = String::with_capacity(1 + indent.len() * depth);
        lead.push('\n');
        for _ in 0..depth {
            lead.push_str(indent);
        }
        self.emit(&lead)
    }

    /// Records that character data has reached the innermost of `open`
    /// elements, which is what [`Self::lead`] reads.
    ///
    /// # Errors
    ///
    /// [`Self::count`]'s.
    fn mark_mixed(&self, open: usize) -> Result<(), Fault> {
        if self.count(MIXED_SLOT)? == 0 {
            self.set_count(MIXED_SLOT, open);
        }
        Ok(())
    }

    /// The name of the innermost of `open` elements.
    ///
    /// # Errors
    ///
    /// A `Fault::fatal` where the stack holds no name for an element the count
    /// says is open, which only a bug in this module can produce.
    fn innermost(&self, open: usize) -> Result<String, Fault> {
        let names = self.array(OPEN_NAMES_SLOT)?;
        let at = i64::try_from(open - 1).expect("`DEPTH_CEILING` is far under `i64::MAX`");
        let held = names.get_index(at);
        held.as_ref()
            .and_then(|name| name.as_text())
            .map(str::to_owned)
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{WRITER_NAME}::{} has an element open with no name on its stack",
                    self.member
                ))
            })
    }
}

/// Refuses `text` if it holds a character XML has no way to write.
///
/// The parser's own answer, read off [`numeric`]'s refusal: a control character
/// other than the three whitespace ones has no spelling in a document, escaped
/// or otherwise, so writing one would produce text this module's own reader
/// would refuse.
///
/// # Errors
///
/// A `LogicError` naming the character.
fn writable(pen: &Pen, text: &str) -> Result<(), Fault> {
    if let Some(ch) = unwritable(text) {
        return pen.refuse(&format!(
            "the text holds U+{:04X}, which a document has no way to write",
            u32::from(ch)
        ));
    }
    Ok(())
}

/// The character in `text` a document has no way to write, if it holds one.
///
/// Both writers ask this and each phrases its own refusal, since what a
/// document can carry is the module's rule rather than either door's:
/// [`writable`] is the stream's and [`carried`] is the tree's.
fn unwritable(text: &str) -> Option<char> {
    text.chars().find(|&ch| forbidden(ch))
}

/// [`nvs_core_xml_writer_start_document`]'s body.
///
/// # Errors
///
/// A `LogicError` for a document already open or already finished.
fn start_document(receiver: Value) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "startDocument")?;
    match pen.count(STATE_SLOT)? {
        BEFORE => {}
        WRITING => return pen.refuse("the document is already open"),
        _ => return pen.refuse("the document is finished, and `endDocument` has answered it"),
    }
    pen.emit(r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    pen.set_count(STATE_SLOT, WRITING);
    Ok(Value::null())
}

/// [`nvs_core_xml_writer_end_document`]'s body: the one place a writer answers
/// anything, and the one place it refuses a tree rather than emitting it.
///
/// # Errors
///
/// A `LogicError` for an element still open, for a document with no root
/// element, and for a document that is not open.
fn end_document(receiver: Value) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "endDocument")?;
    pen.writing()?;
    let open = pen.count(OPEN_COUNT_SLOT)?;
    if open > 0 {
        let name = pen.innermost(open)?;
        return pen.refuse(&format!(
            "`<{name}>` is still open, so what this holds is not a document"
        ));
    }
    if !pen.flag(ROOT_SLOT)? {
        return pen.refuse("a document has a root element, and none was written");
    }

    let written = pen.array(WRITTEN_SLOT)?;
    let mut document = String::new();
    for at in 0..written.count() {
        let at = i64::try_from(at).expect("a document's piece count is under `i64::MAX`");
        let held = written.get_index(at);
        let Some(piece) = held.as_ref().and_then(|piece| piece.as_text()) else {
            return Err(Fault::fatal(format!(
                "{WRITER_NAME}::endDocument found a piece of its document that is not text"
            )));
        };
        document.push_str(piece);
    }
    pen.set_count(STATE_SLOT, FINISHED);
    Ok(Value::str(NvsStr::new(document.as_bytes())))
}

/// [`nvs_core_xml_writer_start_element`]'s body.
///
/// # Errors
///
/// A `LogicError` for a name a document cannot carry, for a second root
/// element, for nesting past [`DEPTH_CEILING`], and for a document that is not
/// open.
fn start_element(receiver: Value, name: &str) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "startElement")?;
    pen.writing()?;
    if !is_name(name) {
        return pen.refuse(&format!("`{name}` is not a name a document can write"));
    }
    let open = pen.count(OPEN_COUNT_SLOT)?;
    if open == 0 && pen.flag(ROOT_SLOT)? {
        return pen.refuse("a document has one root element, and it is already written");
    }
    if open == DEPTH_CEILING {
        return pen.refuse(&format!(
            "elements are nested {DEPTH_CEILING} deep, which is as deep as a document goes"
        ));
    }

    pen.seal()?;
    pen.lead(open)?;
    pen.emit("<")?;
    pen.emit(name)?;
    let mut names = pen.array(OPEN_NAMES_SLOT)?;
    let at = i64::try_from(open).expect("`DEPTH_CEILING` is far under `i64::MAX`");
    names.set_index(at, Value::str(NvsStr::new(name.as_bytes())));
    pen.set_count(OPEN_COUNT_SLOT, open + 1);
    pen.set_flag(TAG_SLOT, true);
    // A start tag carries no attribute twice, and the names it already carries
    // are the tag's rather than the writer's, so they go when it does.
    crate::instance::set_slot(pen.object, TAG_NAMES_SLOT, Value::array(NvsArray::new()));
    if open == 0 {
        pen.set_flag(ROOT_SLOT, true);
    }
    Ok(Value::null())
}

/// [`nvs_core_xml_writer_end_element`]'s body.
///
/// Which of `<a/>` and `<a></a>` an empty element is written as is settled here
/// rather than by a second closing member, per spec § 17's row: an element
/// nothing was written into is the first, and every other one is the second.
///
/// # Errors
///
/// A `LogicError` where nothing is open, and for a document that is not open.
fn end_element(receiver: Value) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "endElement")?;
    pen.writing()?;
    let open = pen.count(OPEN_COUNT_SLOT)?;
    if open == 0 {
        return pen.refuse("no element is open, so there is nothing to close");
    }
    if pen.flag(TAG_SLOT)? {
        pen.emit("/>")?;
        pen.set_flag(TAG_SLOT, false);
    } else {
        pen.lead(open - 1)?;
        let name = pen.innermost(open)?;
        pen.emit("</")?;
        pen.emit(&name)?;
        pen.emit(">")?;
    }
    pen.set_count(OPEN_COUNT_SLOT, open - 1);
    if pen.count(MIXED_SLOT)? > open - 1 {
        pen.set_count(MIXED_SLOT, 0);
    }
    Ok(Value::null())
}

/// [`nvs_core_xml_writer_content`]'s body — the writer's escape point.
///
/// # Errors
///
/// A `LogicError` where no element is open, for a character a document cannot
/// write, and for a document that is not open.
fn content(receiver: Value, text: &str) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "content")?;
    pen.writing()?;
    let open = pen.count(OPEN_COUNT_SLOT)?;
    if open == 0 {
        return pen.refuse("character data belongs inside an element, and none is open");
    }
    writable(&pen, text)?;
    pen.seal()?;
    pen.emit(&escaped(text))?;
    pen.mark_mixed(open)?;
    Ok(Value::null())
}

/// [`nvs_core_xml_writer_attribute`]'s body.
///
/// # Errors
///
/// A `LogicError` where no start tag is still taking attributes, for a name the
/// tag already carries, for a name a document cannot carry, for a value holding
/// a character a document cannot write, and for a document that is not open.
fn attribute(receiver: Value, name: &str, value: &str) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "attribute")?;
    pen.writing()?;
    if !pen.flag(TAG_SLOT)? {
        return pen.refuse(
            "no start tag is still taking attributes — an attribute goes on its element before \
             anything is written inside it",
        );
    }
    if !is_name(name) {
        return pen.refuse(&format!("`{name}` is not a name a document can write"));
    }
    writable(&pen, value)?;

    let mut carried = pen.array(TAG_NAMES_SLOT)?;
    let count = carried.count();
    for at in 0..count {
        let at = i64::try_from(at).expect("a start tag's attribute count is under `i64::MAX`");
        let held = carried.get_index(at);
        if held.as_ref().and_then(|held| held.as_text()) == Some(name) {
            return pen.refuse(&format!(
                "this element already carries an attribute named `{name}`"
            ));
        }
    }
    let at = i64::try_from(count).expect("a start tag's attribute count is under `i64::MAX`");
    carried.set_index(at, Value::str(NvsStr::new(name.as_bytes())));

    pen.emit(" ")?;
    pen.emit(name)?;
    pen.emit("=\"")?;
    pen.emit(&quoted(value))?;
    pen.emit("\"")?;
    Ok(Value::null())
}

/// [`nvs_core_xml_writer_comment`]'s body.
///
/// # Errors
///
/// A `LogicError` for content XML gives a comment no way to hold, and for a
/// document that is not open.
fn comment(receiver: Value, text: &str) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "comment")?;
    pen.writing()?;
    if text.contains("--") || text.ends_with('-') {
        return pen
            .refuse("a comment has no escape grammar, so it cannot hold `--` or end with `-`");
    }
    writable(&pen, text)?;
    pen.seal()?;
    pen.lead(pen.count(OPEN_COUNT_SLOT)?)?;
    pen.emit("<!--")?;
    pen.emit(text)?;
    pen.emit("-->")?;
    Ok(Value::null())
}

/// [`nvs_core_xml_writer_cdata`]'s body.
///
/// # Errors
///
/// A `LogicError` where no element is open, for a text holding `]]>` or a
/// character a document cannot write, and for a document that is not open.
fn cdata(receiver: Value, text: &str) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "cdata")?;
    pen.writing()?;
    let open = pen.count(OPEN_COUNT_SLOT)?;
    if open == 0 {
        return pen.refuse("character data belongs inside an element, and none is open");
    }
    if text.contains("]]>") {
        return pen.refuse("a CDATA section has no escape grammar, so it cannot hold `]]>`");
    }
    writable(&pen, text)?;
    pen.seal()?;
    pen.emit("<![CDATA[")?;
    pen.emit(text)?;
    pen.emit("]]>")?;
    pen.mark_mixed(open)?;
    Ok(Value::null())
}

/// [`nvs_core_xml_writer_instruction`]'s body.
///
/// # Errors
///
/// A `LogicError` for a target a document cannot carry or that is the
/// declaration's own, for data holding `?>` or a character a document cannot
/// write, and for a document that is not open.
fn instruction(receiver: Value, target: &str, data: &str) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "instruction")?;
    pen.writing()?;
    if !is_name(target) {
        return pen.refuse(&format!("`{target}` is not a name a document can write"));
    }
    if target.eq_ignore_ascii_case("xml") {
        return pen.refuse("`xml` is the declaration's own target, which `startDocument` writes");
    }
    if data.contains("?>") {
        return pen.refuse(
            "a processing instruction has no escape grammar, so its data cannot hold `?>`",
        );
    }
    writable(&pen, data)?;
    pen.seal()?;
    pen.lead(pen.count(OPEN_COUNT_SLOT)?)?;
    pen.emit("<?")?;
    pen.emit(target)?;
    if !data.is_empty() {
        pen.emit(" ")?;
        pen.emit(data)?;
    }
    pen.emit("?>")?;
    Ok(Value::null())
}

/// [`nvs_core_xml_writer_doctype`]'s body.
///
/// # Errors
///
/// A `LogicError` once the root element is written, for a name a document
/// cannot carry, and for a document that is not open.
fn doctype(receiver: Value, name: &str) -> Result<Value, Fault> {
    let pen = Pen::of(receiver, "doctype")?;
    pen.writing()?;
    if pen.flag(ROOT_SLOT)? {
        return pen.refuse(
            "a document type declaration goes above the root element, and the root element is \
             already written",
        );
    }
    if !is_name(name) {
        return pen.refuse(&format!("`{name}` is not a name a document can write"));
    }
    pen.lead(0)?;
    pen.emit("<!DOCTYPE ")?;
    pen.emit(name)?;
    pen.emit(">")?;
    Ok(Value::null())
}

/// One of a writer member's text arguments.
///
/// # Errors
///
/// A `Fault::fatal` for anything else.
fn text_of<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    // unreachable from source: every one of these parameters is `CoreTy::Text`,
    // so an argument that is not a `string` is `E0401` at the call site.
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{WRITER_NAME}::{member} expected a `string`, got tag {}",
            value.tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Xml::writer({indent?: string}): Core\Xml\Writer` — replacing
    /// `xmlwriter_open_memory` and the five calls PHP configures one with.
    ///
    /// Nothing is written here, exactly as [`nvs_core_xml_reader`] reads
    /// nothing: the writer holds an empty document, an empty stack and the
    /// indent it will use, and `startDocument` is what puts the first character
    /// into it.
    fn nvs_core_xml_writer(_ctx, args: [1]) {
        // unreachable from source: the option is `CoreTy::Str`, so anything
        // that is not a `string` is `E0401` at the call site.
        let Some(indent) = args[0].as_text() else {
            return Err(Fault::fatal(format!(
                "{NAME}::writer expected a `string` indent, got tag {}",
                args[0].tag_byte()
            )));
        };
        if !indent.chars().all(is_space) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{NAME}::writer(): an indent is whitespace, and `{indent}` holds something \
                     a document would carry as content."
                ),
            ));
        }
        Ok(crate::instance::build(
            &WRITER,
            [
                Value::array(NvsArray::new()),
                Value::array(NvsArray::new()),
                counted(0),
                Value::bool(false),
                Value::array(NvsArray::new()),
                Value::str(NvsStr::new(indent.as_bytes())),
                counted(0),
                counted(BEFORE),
                Value::bool(false),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->startDocument(): void` — the outer pair's opening half.
    fn nvs_core_xml_writer_start_document(_ctx, args: [1]) {
        start_document(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->endDocument(): string` — the outer pair's closing half, and
    /// the document itself.
    fn nvs_core_xml_writer_end_document(_ctx, args: [1]) {
        end_document(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->startElement(string $name): void` — the inner pair's opening
    /// half.
    fn nvs_core_xml_writer_start_element(_ctx, args: [2]) {
        start_element(args[0], text_of(&args[1], "startElement")?)
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->endElement(): void` — the inner pair's closing half, which
    /// takes no name because the writer knows what is open.
    fn nvs_core_xml_writer_end_element(_ctx, args: [1]) {
        end_element(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->content(string $text): void` — character data, escaped.
    fn nvs_core_xml_writer_content(_ctx, args: [2]) {
        content(args[0], text_of(&args[1], "content")?)
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->attribute(string $name, string $value): void` — one attribute
    /// on the element that was just opened.
    fn nvs_core_xml_writer_attribute(_ctx, args: [3]) {
        attribute(
            args[0],
            text_of(&args[1], "attribute")?,
            text_of(&args[2], "attribute")?,
        )
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->comment(string $text): void` — a comment, as one call.
    fn nvs_core_xml_writer_comment(_ctx, args: [2]) {
        comment(args[0], text_of(&args[1], "comment")?)
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->cdata(string $text): void` — character data as a CDATA
    /// section, which is an escaping choice about the same text `content`
    /// writes.
    fn nvs_core_xml_writer_cdata(_ctx, args: [2]) {
        cdata(args[0], text_of(&args[1], "cdata")?)
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->instruction(string $target, string $data): void` — a
    /// processing instruction, as one call.
    fn nvs_core_xml_writer_instruction(_ctx, args: [3]) {
        instruction(
            args[0],
            text_of(&args[1], "instruction")?,
            text_of(&args[2], "instruction")?,
        )
    }
}

nvs_runtime::nvs_helper! {
    /// `$writer->doctype(string $name): void` — the document type declaration,
    /// which names a document type and resolves nothing.
    fn nvs_core_xml_writer_doctype(_ctx, args: [2]) {
        doctype(args[0], text_of(&args[1], "doctype")?)
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsStr, OutputSink, Tag, Value, call};

    use super::{
        ATTRIBUTES_SLOT, CHILDREN_SLOT, CLASS, DOCTYPE_REFUSAL, KIND, KIND_NAME, KIND_SLOT, Kind,
        NAME, NAME_SLOT, NAMESPACE_SLOT, NODE, NODE_NAME, Parsed, TEXT_SLOT, WRITER, WRITER_NAME,
        WRITER_OPTIONS, parse,
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
        // has anywhere to put one. Every door that *reads* takes the document's
        // text and nothing else, with no defaults, so there is no argument a
        // caller could pass and no default a deployment could have changed —
        // and the sweep is over the rows rather than over the parse alone,
        // because a flag added to the stream would resolve entities just as
        // thoroughly. The writing door is asked a different question below,
        // because it takes no document and so has nothing to resolve out of
        // one.
        for member in CLASS.methods {
            if matches!(member.return_ty, CoreTy::Instance(name) if name == WRITER_NAME) {
                continue;
            }
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

        // The writing door: it takes no document, so the question here is that
        // its options are about the output and nothing else. One option, whose
        // whole content is what a level of nesting is indented by — a second
        // one is where a resolver's flag would arrive if it ever did.
        let writer = CLASS
            .methods
            .iter()
            .find(
                |member| matches!(member.return_ty, CoreTy::Instance(name) if name == WRITER_NAME),
            )
            .expect("§ 17's writing door is registered");
        assert!(
            writer
                .params
                .iter()
                .all(|param| matches!(param, CoreTy::Options(_))),
            "`Core\\Xml::writer` takes something other than its own formatting"
        );
        assert_eq!(
            WRITER_OPTIONS
                .iter()
                .map(|option| option.name)
                .collect::<Vec<_>>(),
            vec!["indent"],
            "a second option on the one door that takes no document is where a setting would go"
        );
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

    /// A tag with a great many attributes is read in linear time, and a name
    /// repeated after the set took over from the scan is still refused.
    ///
    /// Fifty thousand names is where a scan over every earlier name stops
    /// finishing inside a test run, so this is the regression for the set as
    /// much as for the answer. The short tag keeps the scan's half asserted.
    // covers: Core\Xml::parse
    #[test]
    fn a_wide_tag_is_read_in_linear_time_and_a_repeated_name_is_still_refused() {
        use std::fmt::Write as _;

        let mut wide = String::from("<item");
        for i in 0..50_000 {
            write!(wide, " a{i}=\"{i}\"").expect("writing to a String cannot fail");
        }
        let document = parse(&format!("{wide}/>")).expect("distinct names are well-formed");
        assert_eq!(document.children[0].attributes.len(), 50_000);
        assert_eq!(document.children[0].attributes[49_999].0, "a49999");

        let err = parse(&format!("{wide} a7=\"again\"/>"))
            .expect_err("a name repeated past the scan is caught by the set");
        assert_eq!(err, "attribute `a7` is written twice on `item`");

        let err = parse("<item a=\"1\" b=\"2\" a=\"3\"/>")
            .expect_err("a name repeated inside the scan is caught by the scan");
        assert_eq!(err, "attribute `a` is written twice on `item`");
    }

    /// A control character XML has no way to write is refused written
    /// literally, in every place a document carries characters, exactly as it
    /// is refused written as a reference — so no parsed tree holds one that its
    /// own `source` would then refuse to write.
    // covers: Core\Xml::parse
    #[test]
    fn a_literal_control_character_is_refused_wherever_the_document_carries_one() {
        for document in [
            "<a>x\0y</a>",
            "<a b=\"x\u{1}y\"/>",
            "<a><!--x\u{1}y--></a>",
            "<a><?go x\u{1}y?></a>",
            "<a><![CDATA[x\u{1}y]]></a>",
        ] {
            let err = parse(document).expect_err("a control character is not XML");
            assert!(
                err.starts_with("the document holds U+000"),
                "{document:?} answered {err}"
            );
        }
        assert!(
            parse("<a>&#1;</a>").is_err(),
            "the reference is refused as before"
        );
        let kept = parse("<a b=\"\t\">x\r\ny\t</a>").expect("the three whitespace ones are XML");
        assert_eq!(
            kept.children[0].attributes[0].1, " ",
            "a literal tab in an attribute value reads as a space"
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
            | CoreTy::SecretStr
            | CoreTy::SecretText(_)
            | CoreTy::TaintedStr
            | CoreTy::TaintedBytes
            | CoreTy::SecretTaintedStr
            | CoreTy::SecretTaintedBytes
            | CoreTy::Void
            | CoreTy::Mixed
            | CoreTy::Object
            | CoreTy::Callable
            | CoreTy::MethodRef
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
            | CoreTy::SecretBlob(_)
            | CoreTy::SecretStr
            | CoreTy::SecretText(_) => false,
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
            | CoreTy::SecretTaintedBytes
            | CoreTy::Void
            | CoreTy::Mixed
            | CoreTy::Object
            | CoreTy::Callable
            | CoreTy::MethodRef
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
            CoreTy::Array(inner) | CoreTy::Nullable(inner) => is_inert(inner),
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
        let namespace = crate::instance::slot(receiver, NAMESPACE_SLOT);
        assert!(
            matches!(namespace.tag(), Some(Tag::Str | Tag::Null)),
            "a node's namespace is text or nothing at all"
        );
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

    /// An element as `name=namespace`, with `none` for one in no namespace —
    /// both read through the members a program calls, so the answer asserted
    /// is the answer a program gets.
    fn described(node: Value, ctx: &mut Ctx) -> String {
        let name = call(super::nvs_core_xml_node_name, ctx, &[node]).expect("a node has a name");
        let uri = call(super::nvs_core_xml_node_namespace_uri, ctx, &[node])
            .expect("a node answers its namespace");
        let described = format!(
            "{}={}",
            name.as_text().expect("a name is text"),
            uri.as_text().unwrap_or("none")
        );
        #[expect(
            unsafe_code,
            reason = "both answers are references this test was handed and now owns"
        )]
        unsafe {
            name.release();
            uri.release();
        }
        described
    }

    /// Every element under `node`, described, in document order.
    fn elements(node: Value, ctx: &mut Ctx, into: &mut Vec<String>) {
        let receiver = node.obj_ptr().expect("a node is an object");
        if crate::instance::slot(receiver, KIND_SLOT).as_int() == Some(Kind::Element.ordinal()) {
            into.push(described(node, ctx));
        }
        let children = crate::instance::slot(receiver, CHILDREN_SLOT);
        let children = children.array_ptr().expect("the slot is an array");
        let children = crate::arr::borrowed(children);
        let mut from = 0usize;
        while let Some(at) = children.next_slot(from) {
            from = at + 1;
            let child = children
                .value_at(at)
                .expect("next_slot only names live entries");
            elements(child, ctx, into);
        }
    }

    /// An element answers the namespace its name is in, resolved against the
    /// declarations in scope where it sits rather than against its own
    /// attributes alone — and the two doors onto the family answer alike.
    ///
    /// Asserted over a sweep rather than off one element, because the
    /// interesting answers are the ones a nesting decides: an inner `xmlns`
    /// rebinds the default for a subtree, an `xmlns=""` undeclares it, a
    /// prefix declared on the root covers a descendant that declares nothing,
    /// and `xml` is bound with no declaration anywhere.
    #[test]
    fn an_xml_element_answers_its_namespace_uri() {
        let document = "<r xmlns=\"urn:d\" xmlns:x=\"urn:x\">\
                        <x:a/><b xmlns=\"urn:b\"><c/></b><d xmlns=\"\"/><x:e xml:lang=\"en\"/>\
                        <f y=\"1\"/></r>";
        let expected = vec![
            "r=urn:d".to_owned(),
            "x:a=urn:x".to_owned(),
            "b=urn:b".to_owned(),
            "c=urn:b".to_owned(),
            "d=none".to_owned(),
            "x:e=urn:x".to_owned(),
            "f=urn:d".to_owned(),
        ];

        let source = Value::str(NvsStr::new(document.as_bytes()));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_xml_parse, &mut ctx, &[source]).expect("that document is one");
        let mut walked = Vec::new();
        elements(tree, &mut ctx, &mut walked);
        assert_eq!(
            walked, expected,
            "an element answers the namespace in scope where it sits"
        );

        // The streaming door holds its ancestors where the tree door holds its
        // descendants, so this is the same question asked of the other one.
        let reader = call(super::nvs_core_xml_reader, &mut ctx, &[source]).expect("a reader opens");
        let mut streamed = Vec::new();
        loop {
            let node = call(super::nvs_core_xml_reader_read, &mut ctx, &[reader])
                .expect("that document is well-formed all the way through");
            if node.tag() == Some(Tag::Null) {
                break;
            }
            let receiver = node.obj_ptr().expect("a node is an object");
            if crate::instance::slot(receiver, KIND_SLOT).as_int() == Some(Kind::Element.ordinal())
            {
                streamed.push(described(node, &mut ctx));
            }
            #[expect(
                unsafe_code,
                reason = "the node is the reference `read` answered with, and this test owns it"
            )]
            unsafe {
                node.release();
            }
        }
        assert_eq!(
            streamed, walked,
            "a reader resolves what the tree door resolves, element for element"
        );

        #[expect(
            unsafe_code,
            reason = "this test owns the references the two doors answered with"
        )]
        unsafe {
            reader.release();
            tree.release();
            source.release();
        }
    }

    /// Every entry of the array `attributes` answered for `node`, as
    /// `(name, value)` pairs in the order the array holds them.
    fn attribute_pairs(node: Value, ctx: &mut Ctx) -> Vec<(String, String)> {
        let answered =
            call(super::nvs_core_xml_node_attributes, ctx, &[node]).expect("a node has attributes");
        let array = answered.array_ptr().expect("attributes is an array");
        let array = crate::arr::borrowed(array);
        let mut pairs = Vec::new();
        let mut from = 0usize;
        while let Some(at) = array.next_slot(from) {
            from = at + 1;
            let key = array.key_at(at).expect("an attribute is keyed by its name");
            let value = array
                .value_at(at)
                .expect("next_slot only names live entries");
            pairs.push((
                String::from_utf8_lossy(key.as_bytes()).into_owned(),
                value
                    .as_text()
                    .expect("an attribute value is text")
                    .to_owned(),
            ));
        }
        dropped(answered);
        pairs
    }

    /// An element's attributes come back keyed by name, in the order the tag
    /// wrote them, with references expanded and a literal tab, line feed or
    /// `\r\n` read as one space (XML 1.0 § 3.3.3); every other kind of node
    /// answers an empty array.
    // covers: Core\Xml\Node::attributes
    #[test]
    fn attributes_keep_written_order_and_normalise_literal_whitespace() {
        let document = "<item b=\"2\" a=\"1 &amp; 2\" c=\"x\r\ny\tz\n\" d=\"&#9;\"/><!--c-->";
        let source = Value::str(NvsStr::new(document.as_bytes()));
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_xml_parse, &mut ctx, &[source]).expect("that document is one");
        assert!(
            attribute_pairs(tree, &mut ctx).is_empty(),
            "the document node has no attributes"
        );

        let children = call(super::nvs_core_xml_node_children, &mut ctx, &[tree])
            .expect("the document has children");
        let list = crate::arr::borrowed(children.array_ptr().expect("children is an array"));
        let item = list.value_at(0).expect("the root element comes first");
        let comment = list.value_at(1).expect("the comment follows it");
        let written = [("b", "2"), ("a", "1 & 2"), ("c", "x y z "), ("d", "\t")]
            .map(|(name, value)| (name.to_owned(), value.to_owned()));
        assert_eq!(attribute_pairs(item, &mut ctx), written);
        assert!(
            attribute_pairs(comment, &mut ctx).is_empty(),
            "a comment has no attributes"
        );

        dropped(children);
        dropped(tree);
        dropped(source);
    }

    /// `children` answers the node's own array, retained rather than copied:
    /// two calls answer one allocation, each call is one more owner, and the
    /// whitespace between elements is kept as text nodes in document order. A
    /// leaf answers an empty array.
    // covers: Core\Xml\Node::children
    #[test]
    fn children_share_the_nodes_own_array_and_keep_the_whitespace_between_elements() {
        let source = word("<list>\n  <a/>\n  <b>x</b>\n</list><!--end-->");
        let mut ctx = Ctx::new(OutputSink::Sink);
        let tree =
            call(super::nvs_core_xml_parse, &mut ctx, &[source]).expect("that document is one");

        let first = call(super::nvs_core_xml_node_children, &mut ctx, &[tree])
            .expect("the document has children");
        let again = call(super::nvs_core_xml_node_children, &mut ctx, &[tree])
            .expect("the same question twice");
        let top = first.array_ptr().expect("children is an array");
        assert_eq!(
            Some(top),
            again.array_ptr(),
            "both calls answer one allocation"
        );
        let top = crate::arr::borrowed(top);
        assert_eq!(top.refcount(), 3, "the node's slot and one owner per call");

        let kinds = |array: &nvs_runtime::NvsArray| -> Vec<&'static str> {
            (0..array.count())
                .map(|at| {
                    let child = array.value_at(at).expect("children are packed");
                    let object = child.obj_ptr().expect("a child is a node");
                    let kind = crate::instance::slot(object, KIND_SLOT).as_int();
                    let kind = usize::try_from(kind.expect("a kind is an int"))
                        .expect("a case's integer is its own index");
                    KIND.cases[kind].0
                })
                .collect()
        };
        assert_eq!(kinds(&top), ["Element", "Comment"]);

        let list = top.value_at(0).expect("the root element comes first");
        let content = call(super::nvs_core_xml_node_children, &mut ctx, &[list])
            .expect("an element has children");
        let inner = crate::arr::borrowed(content.array_ptr().expect("children is an array"));
        assert_eq!(
            kinds(&inner),
            ["Text", "Element", "Text", "Element", "Text"]
        );

        let leaf = inner
            .value_at(0)
            .expect("the first line break is a text node");
        let none =
            call(super::nvs_core_xml_node_children, &mut ctx, &[leaf]).expect("a leaf answers too");
        assert_eq!(
            crate::arr::borrowed(none.array_ptr().expect("children is an array")).count(),
            0,
            "a text node has no children"
        );

        dropped(none);
        dropped(content);
        dropped(again);
        dropped(first);
        assert_eq!(top.refcount(), 1, "the node's slot is the one owner left");
        dropped(tree);
        dropped(source);
    }

    /// A document of a million empty elements builds a Rust tree some thirty
    /// times its own size. The parse asks the budget as it builds, so the
    /// request stops near its ceiling instead of holding that whole tree first.
    // covers: Core\Xml::parse
    #[test]
    fn a_parse_stops_at_the_memory_limit_while_it_builds_the_tree() {
        let mut ctx = Ctx::buffered();
        ctx.set_memory_limit(16 << 20);
        let source = word(&format!("<list>{}</list>", "<i/>".repeat(1_000_000)));
        assert!(call(super::nvs_core_xml_parse, &mut ctx, &[source]).is_err());
        assert!(
            ctx.memory_peak() < 48 << 20,
            "{} bytes at the peak",
            ctx.memory_peak()
        );
        dropped(source);
    }

    /// A writer indenting by `indent`, over the argument list its option takes.
    fn pen(ctx: &mut Ctx, indent: &str) -> Value {
        let indent = Value::str(NvsStr::new(indent.as_bytes()));
        let writer = call(super::nvs_core_xml_writer, ctx, &[indent])
            .expect("whitespace is what an indent is");
        dropped(indent);
        writer
    }

    /// One call against a writer: what it answered, or the sentence it refused
    /// with.
    fn asked(ctx: &mut Ctx, member: nvs_runtime::NvsFn, args: &[Value]) -> Result<Value, String> {
        call(member, ctx, args).map_err(|_| {
            ctx.take_pending()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default()
        })
    }

    /// A call that has to be accepted, and whose answer nothing here keeps.
    fn wrote(answer: Result<Value, String>) {
        match answer {
            Ok(value) => dropped(value),
            Err(why) => panic!("the writer refused a write it had no reason to: {why}"),
        }
    }

    /// A text argument, which the callee borrows and the caller releases.
    fn word(text: &str) -> Value {
        Value::str(NvsStr::new(text.as_bytes()))
    }

    /// What is open is the writer's own state, so a caller cannot get it wrong.
    ///
    /// The acceptance property of `rule:core-classes/xml-tree-and-stream`'s
    /// writing half, asserted the way the goal states it: the caller never says
    /// what is open — `endElement` takes no name at all — so every refusal here
    /// is one the writer makes out of what it is holding. What that buys is
    /// checked at the end by handing the document to this module's own parser,
    /// which refuses everything that is not well-formed: a writer that could be
    /// talked into ill-formed output fails there rather than in a program.
    ///
    /// The refusals are interleaved with the writes on purpose. A refusal
    /// writes nothing, so the document at the end is exactly what the accepted
    /// calls put in it — a writer that half-applied a refused call would still
    /// answer a well-formed document and fail only on this comparison.
    // covers: Core\Xml::writer
    #[test]
    fn the_writer_enforces_nesting_from_its_own_state_and_not_from_the_caller() {
        let close = WRITER
            .members()
            .find(|member| member.name == "endElement")
            .expect("the writer closes an element");
        assert!(
            close.params.is_empty() && close.names.is_empty(),
            "`endElement` reads what is open off the writer, so there is no name to get wrong"
        );

        let start = super::nvs_core_xml_writer_start_element;
        let end = super::nvs_core_xml_writer_end_element;
        let attribute = super::nvs_core_xml_writer_attribute;
        let content = super::nvs_core_xml_writer_content;
        let doctype = super::nvs_core_xml_writer_doctype;

        let mut ctx = Ctx::new(OutputSink::Sink);
        let writer = pen(&mut ctx, "");
        let (order, item, other) = (word("order"), word("item"), word("other"));
        let (sku, one, mixed, digit) = (word("sku"), word("1"), word("a & b"), word("2bad"));

        wrote(asked(
            &mut ctx,
            super::nvs_core_xml_writer_start_document,
            &[writer],
        ));

        // Nothing is open, so there is nothing to close. A caller holding the
        // nesting itself has no way to make this refusal at all.
        let why = asked(&mut ctx, end, &[writer]).expect_err("nothing is open");
        assert!(why.contains("nothing to close"), "{why}");

        wrote(asked(&mut ctx, start, &[writer, order]));

        // A document type declaration goes above the root element, and the root
        // element is written — again a fact about the writer rather than about
        // the argument.
        let why = asked(&mut ctx, doctype, &[writer, order]).expect_err("the root is written");
        assert!(why.contains("above the root element"), "{why}");

        wrote(asked(&mut ctx, attribute, &[writer, sku, one]));
        let why =
            asked(&mut ctx, attribute, &[writer, sku, one]).expect_err("that name is carried");
        assert!(why.contains("already carries an attribute"), "{why}");

        wrote(asked(&mut ctx, start, &[writer, item]));
        wrote(asked(&mut ctx, content, &[writer, mixed]));

        // The start tag is closed because something was written inside it, and
        // an attribute after that would land on nothing.
        let why = asked(&mut ctx, attribute, &[writer, sku, one]).expect_err("the tag is closed");
        assert!(why.contains("still taking attributes"), "{why}");

        wrote(asked(&mut ctx, end, &[writer]));
        wrote(asked(&mut ctx, start, &[writer, item]));
        wrote(asked(&mut ctx, end, &[writer]));

        // A name a document cannot carry, refused where it is written rather
        // than where it is read.
        let why = asked(&mut ctx, start, &[writer, digit]).expect_err("that is not a name");
        assert!(why.contains("is not a name a document can write"), "{why}");

        wrote(asked(&mut ctx, end, &[writer]));

        // The root element is closed, so a second one is a second document —
        // which the writer knows because it is what it has been holding.
        let why = asked(&mut ctx, start, &[writer, other]).expect_err("the root is written");
        assert!(why.contains("one root element"), "{why}");
        let document = asked(&mut ctx, super::nvs_core_xml_writer_end_document, &[writer])
            .expect("nothing is open and the root element is written");
        let text = document
            .as_text()
            .expect("a finished document is text")
            .to_owned();
        assert_eq!(
            text,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <order sku=\"1\"><item>a &amp; b</item><item/></order>",
            "a refusal writes nothing, so this is what the accepted calls wrote"
        );

        // The strict parser is the check the writer's own state is meant to
        // buy: it refuses everything that is not well-formed, and this is what
        // came out.
        let tree = parse(&text).expect("what the writer answered is a document");
        assert_eq!(
            tree.children.len(),
            1,
            "the root element, and nothing beside it"
        );

        dropped(document);
        for held in [order, item, other, sku, one, mixed, digit] {
            dropped(held);
        }
        dropped(writer);
    }

    /// The end of a document with something still open is not a document.
    ///
    /// The reader's half of this is pinned by
    /// `tests/conformance/core/xml-a-reader-refuses-where-the-walk-reaches-it-and-does-not-advance-past-it.nvst`,
    /// and this is the writing half: `endDocument` is the one member that
    /// answers, so it is the one place the writer can refuse a tree rather than
    /// emit it. A refusal leaves the writer where it was, so closing the
    /// element it named and asking again is what finishes the document — the
    /// error is about the state, not about the writer being spent.
    #[test]
    fn an_unclosed_element_at_the_end_is_an_error_and_not_a_document() {
        let start = super::nvs_core_xml_writer_start_element;
        let end = super::nvs_core_xml_writer_end_element;
        let finish = super::nvs_core_xml_writer_end_document;

        let mut ctx = Ctx::new(OutputSink::Sink);
        let writer = pen(&mut ctx, "");
        let (outer, inner) = (word("a"), word("b"));

        // A document with no root element is not one either: what `endDocument`
        // refuses is everything a parser would refuse to read back.
        wrote(asked(
            &mut ctx,
            super::nvs_core_xml_writer_start_document,
            &[writer],
        ));
        let why = asked(&mut ctx, finish, &[writer]).expect_err("nothing was written");
        assert!(
            why.contains("a root element, and none was written"),
            "{why}"
        );

        wrote(asked(&mut ctx, start, &[writer, outer]));
        wrote(asked(&mut ctx, start, &[writer, inner]));

        // Each refusal names the innermost element that is open, and closing it
        // moves the answer one out rather than finishing anything.
        let why = asked(&mut ctx, finish, &[writer]).expect_err("`<b>` is open");
        assert!(why.contains("`<b>` is still open"), "{why}");
        wrote(asked(&mut ctx, end, &[writer]));
        let why = asked(&mut ctx, finish, &[writer]).expect_err("`<a>` is open");
        assert!(why.contains("`<a>` is still open"), "{why}");
        wrote(asked(&mut ctx, end, &[writer]));

        let document = asked(&mut ctx, finish, &[writer]).expect("everything is closed");
        let text = document
            .as_text()
            .expect("a finished document is text")
            .to_owned();
        assert_eq!(
            text, "<?xml version=\"1.0\" encoding=\"UTF-8\"?><a><b/></a>",
            "the refusals wrote nothing, and the closes wrote what they close"
        );
        parse(&text).expect("what the writer answered is a document");

        // And it is finished: a writer answers its document once, and writing
        // into one that has been answered is refused for the same reason
        // closing something that is not open is.
        let why = asked(&mut ctx, finish, &[writer]).expect_err("the document is answered");
        assert!(why.contains("the document is finished"), "{why}");
        let why = asked(&mut ctx, start, &[writer, outer]).expect_err("the document is answered");
        assert!(why.contains("the document is finished"), "{why}");

        dropped(document);
        dropped(outer);
        dropped(inner);
        dropped(writer);
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
    // covers: Core\Xml::reader
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
