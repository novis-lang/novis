//! Machine code one function at a time, with every target named so that the
//! function's place in the program does not matter: what the artifact cache
//! stores per function and places again in any program that has the same key.
//!
//! [`crate::compile_pieces`] emits every function, because the emit is what
//! writes a function's key ([`crate::function_keys`]), and compiles only the
//! functions whose key the caller does not already hold. Each compiled
//! function comes back as a [`Piece`]: its bytes, its alignment and its
//! relocations. A relocation names its target by a [`Target`], never by a
//! symbol that carries a program index, so a piece compiled in one program is
//! the right code in another program where its key is the same.
//!
//! A literal the code reaches is a [`LiteralPiece`] under the digest of its
//! contents (`emit::Literals`). Two functions that use the same literal name
//! it by the same digest, so whoever places the pieces may place it once.
//!
//! **What it spends:** every compiled function's bytes and relocations, and
//! every literal the program defines, held until the caller drops the
//! [`Pieces`]. That is about the object file [`crate::compile_object`] writes
//! for the same functions. A function whose key the caller holds costs its
//! emit and its key, and no Cranelift compile.

use cranelift::codegen;
use cranelift_module::{DataDescription, DataId, FuncId, Module, ModuleRelocTarget};
use rustc_hash::FxHashMap;

/// What a relocation in a [`Piece`] or a [`LiteralPiece`] points at.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// A function of the same program, by its Novis name.
    Function(String),
    /// A literal, by the digest of its contents ([`LiteralPiece::digest`]).
    Literal([u8; 32]),
    /// Anything the loading process resolves by name: a runtime entry, a
    /// class descriptor, a codec, a base symbol or a library call.
    Symbol(String),
}

/// How a relocation is written into the bytes. These are the kinds the
/// backend emits with `is_pic` off ([`crate::host_isa`]). Any other kind is
/// [`RelocKind::Other`], which no loader places.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelocKind {
    /// The target's absolute address, eight bytes.
    Abs8,
    /// The target's absolute address, four bytes.
    Abs4,
    /// x86-64: the target minus the field's address, four bytes.
    X86PcRel4,
    /// x86-64: a call's target minus the field's address, four bytes.
    X86CallPcRel4,
    /// aarch64: a `bl` or `b` to the target, 26 bits of words.
    Arm64Call,
    /// A kind this list does not name.
    Other,
}

impl RelocKind {
    /// Every kind, in the order of [`Self::code`].
    pub const ALL: [Self; 6] = [
        Self::Abs8,
        Self::Abs4,
        Self::X86PcRel4,
        Self::X86CallPcRel4,
        Self::Arm64Call,
        Self::Other,
    ];

    /// The kind as one byte, for a stored piece.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Self::Abs8 => 0,
            Self::Abs4 => 1,
            Self::X86PcRel4 => 2,
            Self::X86CallPcRel4 => 3,
            Self::Arm64Call => 4,
            Self::Other => 255,
        }
    }

    /// The kind [`Self::code`] wrote, or `None` for a byte it never writes.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.code() == code)
    }

    fn of(reloc: codegen::binemit::Reloc) -> Self {
        use codegen::binemit::Reloc;
        match reloc {
            Reloc::Abs8 => Self::Abs8,
            Reloc::Abs4 => Self::Abs4,
            Reloc::X86PCRel4 => Self::X86PcRel4,
            Reloc::X86CallPCRel4 => Self::X86CallPcRel4,
            Reloc::Arm64Call => Self::Arm64Call,
            _ => Self::Other,
        }
    }
}

/// One relocation: at `offset` in the bytes, write the address of `target`
/// plus `addend`, in the form `kind` says.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PieceReloc {
    /// The field's offset from the start of the piece's bytes.
    pub offset: u32,
    /// How the address is written.
    pub kind: RelocKind,
    /// What is added to the target's address.
    pub addend: i64,
    /// What the field points at.
    pub target: Target,
}

/// One function's machine code, with its key and its relocations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    /// The function's Novis name.
    pub name: String,
    /// The function's key ([`crate::function_keys`]).
    pub key: [u8; 32],
    /// The alignment its first byte needs, in bytes.
    pub align: u64,
    /// The machine code, with every relocated field still zero.
    pub bytes: Vec<u8>,
    /// Every field the loader writes, in the order Cranelift wrote them.
    pub relocs: Vec<PieceReloc>,
}

/// One read-only literal, under the digest of its contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiteralPiece {
    /// The digest a function's key and a [`Target::Literal`] name it by.
    pub digest: [u8; 32],
    /// The alignment its first byte needs, in bytes.
    pub align: u64,
    /// The contents, with every relocated field still zero.
    pub bytes: Vec<u8>,
    /// Every pointer in it, each eight bytes ([`RelocKind::Abs8`]).
    pub relocs: Vec<PieceReloc>,
}

/// What [`crate::compile_pieces`] returns.
#[derive(Debug, Default)]
pub struct Pieces {
    /// Every function of the program with its key, in the program's order.
    pub keys: Vec<(String, [u8; 32])>,
    /// Every function whose key the caller did not hold, compiled, in the
    /// program's order.
    pub compiled: Vec<Piece>,
    /// Every literal the program defines, by digest.
    pub literals: FxHashMap<[u8; 32], LiteralPiece>,
}

impl Pieces {
    /// The literals `pieces` reach, directly or through another literal, each
    /// once, in the order they are first reached.
    #[must_use]
    pub fn literals_for<'a>(&'a self, pieces: &[&Piece]) -> Vec<&'a LiteralPiece> {
        let mut seen = rustc_hash::FxHashSet::default();
        let mut out = Vec::new();
        let mut queue: Vec<&[u8; 32]> = pieces
            .iter()
            .flat_map(|piece| &piece.relocs)
            .filter_map(|reloc| match &reloc.target {
                Target::Literal(digest) => Some(digest),
                _ => None,
            })
            .collect();
        queue.reverse();
        while let Some(digest) = queue.pop() {
            if !seen.insert(*digest) {
                continue;
            }
            let Some(literal) = self.literals.get(digest) else {
                continue;
            };
            out.push(literal);
            for reloc in literal.relocs.iter().rev() {
                if let Target::Literal(next) = &reloc.target {
                    queue.push(next);
                }
            }
        }
        out
    }
}

/// The name a relocation in a function or a literal gives `target`.
/// `functions` names each function of the unit by its Novis name, and
/// `literals` each literal by its digest.
pub(crate) fn target_of(
    target: &ModuleRelocTarget,
    module: &dyn Module,
    functions: &FxHashMap<FuncId, String>,
    literals: Option<&FxHashMap<DataId, [u8; 32]>>,
) -> Target {
    let declarations = module.declarations();
    match target {
        ModuleRelocTarget::User {
            namespace: 0,
            index,
        } => {
            let id = FuncId::from_u32(*index);
            functions.get(&id).map_or_else(
                || {
                    Target::Symbol(
                        declarations
                            .get_function_decl(id)
                            .name
                            .clone()
                            .unwrap_or_default(),
                    )
                },
                |name| Target::Function(name.clone()),
            )
        }
        ModuleRelocTarget::User {
            namespace: 1,
            index,
        } => {
            let id = DataId::from_u32(*index);
            literals
                .and_then(|literals| literals.get(&id))
                .map_or_else(
                    || {
                        Target::Symbol(
                            declarations
                                .get_data_decl(id)
                                .name
                                .clone()
                                .unwrap_or_default(),
                        )
                    },
                    |digest| Target::Literal(*digest),
                )
        }
        ModuleRelocTarget::LibCall(call) => {
            Target::Symbol((cranelift_module::default_libcall_names())(*call))
        }
        other => Target::Symbol(format!("{other:?}")),
    }
}

/// The piece of the function Cranelift just compiled into `ctx`.
pub(crate) fn piece_of(
    name: &str,
    key: [u8; 32],
    id: FuncId,
    ctx: &codegen::Context,
    module: &dyn Module,
    functions: &FxHashMap<FuncId, String>,
    literals: Option<&FxHashMap<DataId, [u8; 32]>>,
) -> Option<Piece> {
    let code = ctx.compiled_code()?;
    let relocs = code
        .buffer
        .relocs()
        .iter()
        .map(|reloc| {
            let reloc = cranelift_module::ModuleReloc::from_mach_reloc(reloc, &ctx.func, id);
            PieceReloc {
                offset: reloc.offset,
                kind: RelocKind::of(reloc.kind),
                addend: reloc.addend,
                target: target_of(&reloc.name, module, functions, literals),
            }
        })
        .collect();
    Some(Piece {
        name: name.to_owned(),
        key,
        align: u64::from(code.buffer.min_alignment),
        bytes: code.buffer.data().to_vec(),
        relocs,
    })
}

/// The piece of the literal `desc` describes, under `digest`.
pub(crate) fn literal_piece(
    digest: [u8; 32],
    desc: &DataDescription,
    module: &dyn Module,
    literals: Option<&FxHashMap<DataId, [u8; 32]>>,
) -> LiteralPiece {
    let bytes = match &desc.init {
        cranelift_module::Init::Bytes { contents } => contents.to_vec(),
        cranelift_module::Init::Zeros { size } => vec![0; *size],
        cranelift_module::Init::Uninitialized => Vec::new(),
    };
    let none = FxHashMap::default();
    let functions = desc.function_relocs.iter().map(|(offset, func)| PieceReloc {
        offset: *offset,
        kind: RelocKind::Abs8,
        addend: 0,
        target: target_of(&desc.function_decls[*func], module, &none, literals),
    });
    let data = desc
        .data_relocs
        .iter()
        .map(|(offset, global, addend)| PieceReloc {
            offset: *offset,
            kind: RelocKind::Abs8,
            addend: *addend,
            target: target_of(&desc.data_decls[*global], module, &none, literals),
        });
    LiteralPiece {
        digest,
        align: desc.align.unwrap_or(1),
        bytes,
        relocs: functions.chain(data).collect(),
    }
}
