//! Emits one [`mwl_ir::Function`] as Cranelift IR.
//!
//! The whole file is a walk of already-checked, already-lowered input: by the
//! time an [`mwl_ir::ir::Function`] reaches here, `mwl_types::check_program`
//! has proven it well-typed and `mwl_ir::lower` has settled every operand's
//! representation. Nothing is re-checked; a shape this slice cannot lower is a
//! [`CodegenError::Unsupported`] naming it, never a diagnostic.
//!
//! # Blocks and phis
//!
//! [`mwl_ir::ir::InstKind::Phi`] becomes a Cranelift block parameter. The IR
//! always emits a block's phis as a prefix of its instruction list — every
//! producer in `mwl_ir::lower` writes them into a freshly created merge or
//! loop-header block before anything else — so the translation is positional:
//! the *n*th leading phi is the *n*th block parameter, and every jump into
//! that block supplies its own incoming value for each. A phi appearing after
//! a non-phi instruction would break that correspondence, so it is rejected
//! rather than silently mistranslated.
//!
//! # The status check
//!
//! Every call this file emits — a runtime helper, `mwl_safepoint` — is
//! followed by the compare-and-branch [ADR 0002](../../../docs/adr/0002-error-propagation.md)
//! puts in place of a landing pad, and a non-`OK` status returns onward
//! unchanged. `mwl_probe_stmt` and the refcount primitives are the exceptions,
//! and only because they return no status at all: neither can fail.
//!
//! # A runtime call that is not a helper
//!
//! Four symbols this file calls take neither ADR 0002's helper convention nor
//! the status check above: `mwl_str_eq`, `mwl_array_eq`, `mwl_float_pow` and
//! the refcount primitives. The rule they share is that the operand
//! *representation* is already statically known at the emit site and the
//! operation is total, so the convention's price — marshalling each argument
//! into a stack slot of tagged `mwl_runtime::Value`s, reading the result back
//! out of an out-slot, and branching on a status that is always `OK` — would
//! buy nothing. Each gets a two-argument signature of its own in
//! `crate::Signatures` instead.
//!
//! `mwl_float_pow` is the one of those that had a real alternative, so it is
//! worth naming: ADR 0007 § 4's `**` over two `float`s could have been one
//! more `mwl_ir::Helper`. It is not, because Cranelift has no `fpow`
//! instruction and no `LibCall::Pow` either — the row has to be *some* call,
//! and given that, the cheap shape is the honest one. AGENTS.md's priority
//! ordering puts latency (3) above simplicity (4), and the cost is one
//! `Signature`, one `RuntimeSig` arm and a thirty-line runtime module. The
//! integer row of the same operator stays inline as a square-and-multiply
//! loop; see `Emitter::emit_int_pow`.

use cranelift::prelude::*;
use cranelift_jit::JITModule;
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use mwl_ir::Ty;
use mwl_ir::ids::{BlockId, ValueId};
use mwl_ir::ir::{
    AbsentKey, BasicBlock, BinOp, Function, Helper, Inst, InstKind, Terminator, UnOp,
};
use mwl_runtime::{
    DEBUG_FLAGS_OFFSET, Decimal as MwlDecimal, OK, SAFEPOINT_OFFSET, STACK_LIMIT_OFFSET, THROWN,
    Tag, Value as MwlValue,
};
use rustc_hash::FxHashMap;

use crate::ty::{clif_ty, tag_of};
use crate::{Classes, CodegenError, Signatures};

/// Size of one 16-byte [`mwl_runtime::Value`], as an offset multiplier.
const VALUE_SIZE: i32 = 16;

/// `align_shift` for a 16-byte-aligned stack slot: 2^4 == 16.
const VALUE_ALIGN_SHIFT: u8 = 4;

/// Size of one raw pointer, as an offset multiplier. This JIT compiles for
/// 64-bit targets only, which is the same assumption every `AbiParam::new(ptr)`
/// in [`crate::Sigs`] already makes.
const POINTER_SIZE: i32 = 8;

/// `align_shift` for an 8-byte-aligned stack slot: 2^3 == 8.
const POINTER_ALIGN_SHIFT: u8 = 3;

/// Memory flags for a load or store this frame fully controls — a stack slot
/// it just allocated, or the `out` pointer its caller promised is writable.
fn trusted() -> MemFlagsData {
    MemFlagsData::trusted()
}

/// Memory flags for reading [`mwl_runtime::Ctx`]'s two hot words.
///
/// Deliberately *not* [`MemFlagsData::trusted`]: `trusted` asserts nothing about
/// aliasing today, but the safepoint word is written from outside the running
/// frame (a CPU-limit watchdog, a cancellation), so the load must stay a real
/// load rather than becoming something Cranelift may prove redundant. Marking
/// it `readonly` — which is what a future "this is immutable" annotation would
/// mean — would be exactly wrong.
fn ctx_word() -> MemFlagsData {
    MemFlagsData::new().with_notrap()
}

/// Everything about the compilation *unit* that emitting one function needs:
/// the runtime signatures, the unit's own function and class tables, and the
/// running literal counter that keeps data-object names unique.
///
/// Bundled rather than passed one by one because every field has the same
/// lifetime and the same "read this, don't rebuild it" role — and because
/// eight parameters is where the shape stops being readable.
pub(crate) struct UnitTables<'a> {
    pub sigs: &'a Signatures,
    /// Every function the unit defines, by MWL name — see
    /// [`crate::Jit::compile_all`] for why it is complete before any body is
    /// emitted.
    pub functions: &'a FxHashMap<String, FuncId>,
    /// Every class the unit declares — see [`crate::Classes`].
    pub classes: &'a Classes,
    /// Every `static` property the unit declares, by `(declaring class, name)`
    /// — see [`crate::Jit::statics`].
    pub statics: &'a FxHashMap<(String, String), u32>,
    /// One entry per emitted `ConstStr`, so data-object names stay unique.
    pub literals: &'a mut usize,
}

/// Emits `f` into `ctx.func`, which the caller has already given the ABI
/// signature.
pub(crate) fn emit_function(
    module: &mut JITModule,
    ctx: &mut codegen::Context,
    fn_ctx: &mut FunctionBuilderContext,
    tables: UnitTables<'_>,
    f: &Function,
) -> Result<(), CodegenError> {
    let UnitTables {
        sigs,
        functions,
        classes,
        statics,
        literals,
    } = tables;
    let target_config = module.target_config();
    let mut b = FunctionBuilder::new(&mut ctx.func, fn_ctx);

    // A dedicated Cranelift entry block carrying the ABI parameters, jumping
    // straight into the IR's own entry block. Keeping the two separate means
    // the IR entry block is translated exactly like every other block, with no
    // special case for "this one also has the function's parameters".
    let abi_entry = b.create_block();
    b.append_block_params_for_function_params(abi_entry);
    b.switch_to_block(abi_entry);
    let ctx_p = b.block_params(abi_entry)[0];
    let args_p = b.block_params(abi_entry)[1];
    let out_p = b.block_params(abi_entry)[2];

    // One walk, used twice: which blocks exist at all, and the order they are
    // filled in — see [`reachable_in_reverse_postorder`] for both reasons.
    let order = reachable_in_reverse_postorder(f);
    let mut blocks = FxHashMap::default();
    let mut phi_counts = FxHashMap::default();
    for &index in &order {
        let block = &f.blocks[index];
        let clif = b.create_block();
        let phis = leading_phis(block)?;
        for inst in &block.insts[..phis] {
            let ty = inst
                .ty
                .ok_or_else(|| internal("a phi with no representation type"))?;
            let clif_ty = clif_ty(ty).ok_or_else(|| internal("a phi of representation `void`"))?;
            b.append_block_param(clif, clif_ty);
        }
        // A landing block is entered carrying the status that sent control
        // there — from a call site's own compare-and-branch, or as the
        // constant `THROWN` from a `Terminator::Throw`. It is a block
        // parameter rather than a re-read of anything, because the two entry
        // paths have nothing in common to re-read.
        if is_landing(block) {
            if phis > 0 {
                return Err(internal("a landing block with phis"));
            }
            b.append_block_param(clif, types::I32);
        }
        blocks.insert(block.id.index(), clif);
        phi_counts.insert(block.id.index(), phis);
    }

    let ir_entry = *blocks
        .get(&f.entry.index())
        .ok_or_else(|| internal("the entry block is not among the function's blocks"))?;
    b.ins().jump(ir_entry, &[]);

    let mut emitter = Emitter {
        b,
        module,
        sigs,
        functions,
        classes,
        statics,
        literals,
        f,
        values: FxHashMap::default(),
        blocks,
        phi_counts,
        frefs: FxHashMap::default(),
        callee_refs: FxHashMap::default(),
        ctx_p,
        args_p,
        out_p,
        landing_status: None,
        order,
        stack_check_pending: !is_leaf(f),
    };
    emitter.emit_blocks()?;
    emitter.b.seal_all_blocks();
    emitter.b.finalize(target_config);
    Ok(())
}

/// How many instructions at the head of `block` are phis, rejecting a phi that
/// appears anywhere else — see this module's docs.
fn leading_phis(block: &BasicBlock) -> Result<usize, CodegenError> {
    let count = block
        .insts
        .iter()
        .take_while(|i| matches!(i.kind, InstKind::Phi { .. }))
        .count();
    if block.insts[count..]
        .iter()
        .any(|i| matches!(i.kind, InstKind::Phi { .. }))
    {
        return Err(internal(
            "a phi that is not part of its block's leading run",
        ));
    }
    Ok(count)
}

/// Whether `f` can be entered without growing the machine stack by more than
/// [`mwl_runtime::STACK_RESERVE`] — in which case
/// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1 elides
/// its call-stack check, because whoever called it passed the compare with
/// that much stack still underneath.
///
/// Read as "does it transfer control anywhere that could recurse": a `Call`,
/// a virtual or dynamic one, a constructor, a runtime helper or a `Core`
/// member. The last two cannot recurse into MWL by themselves, but a `Core`
/// member taking a closure does, and telling those apart would mean a table
/// this pass has no reason to own — so the predicate is deliberately the
/// conservative one, and a function is a leaf only if it calls *nothing*.
///
/// Cranelift decides the real frame size long after this runs, so a frame
/// bound is not available to check the reserve against directly. A function
/// that calls nothing has no `alloca`, no by-value aggregate and no spill set
/// that MWL's own lowering can make large — everything of unbounded size is on
/// the heap — which is what makes the syntactic test sufficient here.
fn is_leaf(f: &Function) -> bool {
    !f.blocks.iter().flat_map(|b| &b.insts).any(|inst| {
        matches!(
            inst.kind,
            InstKind::Call { .. }
                | InstKind::CallVirtual { .. }
                | InstKind::New { .. }
                | InstKind::NewDynamic { .. }
                | InstKind::HelperCall { .. }
                | InstKind::CoreCall { .. }
        )
    })
}

/// Whether this block is one of `mwl_ir`'s landing blocks — the ones
/// [`mwl_ir::ir::Inst::on_error`] and [`Terminator::Throw`] branch to.
///
/// Read off the terminator rather than carried as a flag: only a landing
/// block ends in [`Terminator::Propagate`] or [`Terminator::Catch`], so the
/// terminator already is the marker.
fn is_landing(block: &BasicBlock) -> bool {
    matches!(
        block.term,
        Terminator::Propagate { .. } | Terminator::Catch { .. }
    )
}

pub(crate) fn internal(what: &str) -> CodegenError {
    CodegenError::Unsupported(format!("{what} (this is a bug in mwl-ir or mwl-codegen)"))
}

/// The blocks of `f` that codegen touches at all, in the order it walks them:
/// reverse postorder from [`Function::entry`], as indices into
/// [`Function::blocks`].
///
/// **Order.** [`Emitter::values`] is one flat map filled as blocks are
/// emitted, so a value has to be *defined* before the block using it is
/// reached — a stronger requirement than SSA's, which only asks that the
/// definition dominate the use. Reverse postorder turns one into the other: a
/// dominator always precedes what it dominates, so every non-loop-carried
/// value is in the map by the time it is read, and a loop-carried one arrives
/// through a block parameter rather than the map. `mwl-ir` numbers a block
/// when it *creates* one, which coincides with this order for straight-line
/// and loop code but not for
/// [ADR 0002](../../../docs/adr/0002-error-propagation.md)'s landing blocks:
/// a nested `try`'s inner cleanup block is created *after* the outer one it
/// flows into, so the outer one reads a value the inner one defines.
///
/// **Reachability.** A block this walk never reaches is dropped rather than
/// emitted — no Cranelift block is created for it at all. `mwl-ir` lowers a
/// `try`'s landing block unconditionally, so a protected region whose body
/// turns out to contain nothing fallible leaves a whole dead cleanup chain
/// behind; emitting one would mean asking Cranelift to fill blocks nothing
/// branches to, and there is no order that satisfies the map for a region
/// with no entry.
fn reachable_in_reverse_postorder(f: &Function) -> Vec<usize> {
    let mut index_of: FxHashMap<u32, usize> = FxHashMap::default();
    for (index, block) in f.blocks.iter().enumerate() {
        index_of.insert(block.id.index(), index);
    }
    let Some(&entry) = index_of.get(&f.entry.index()) else {
        return Vec::new();
    };
    let mut seen = vec![false; f.blocks.len()];
    let mut postorder = Vec::with_capacity(f.blocks.len());
    // An explicit stack rather than recursion: a long straight-line function
    // is one deep chain of blocks, and this runs on the compiler's own stack.
    seen[entry] = true;
    let mut stack: Vec<(usize, usize)> = vec![(entry, 0)];
    while let Some((index, next)) = stack.pop() {
        let successors = f.blocks[index].successors();
        if next == successors.len() {
            postorder.push(index);
            continue;
        }
        stack.push((index, next + 1));
        if let Some(&child) = index_of.get(&successors[next].index())
            && !seen[child]
        {
            seen[child] = true;
            stack.push((child, 0));
        }
    }
    postorder.reverse();
    postorder
}

struct Emitter<'a, 'f> {
    b: FunctionBuilder<'f>,
    module: &'a mut JITModule,
    sigs: &'a Signatures,
    /// Every function the unit defines, by MWL name — see
    /// [`crate::Jit::compile_all`] for why it is complete before any body is
    /// emitted.
    functions: &'a FxHashMap<String, FuncId>,
    /// Every class the unit declares — see [`crate::Classes`].
    classes: &'a Classes,
    /// Every `static` property the unit declares — see [`crate::Jit::statics`].
    statics: &'a FxHashMap<(String, String), u32>,
    literals: &'a mut usize,
    f: &'a Function,
    /// Every SSA value defined so far, with the representation it was defined
    /// at — the IR carries that on the defining instruction, and an operand
    /// needs it back to pick an instruction (`sdiv` vs `udiv` vs `fdiv`).
    values: FxHashMap<u32, (Value, Ty)>,
    blocks: FxHashMap<u32, Block>,
    phi_counts: FxHashMap<u32, usize>,
    frefs: FxHashMap<&'static str, codegen::ir::FuncRef>,
    /// The same cache as `frefs`, for the unit's *own* functions.
    callee_refs: FxHashMap<String, codegen::ir::FuncRef>,
    ctx_p: Value,
    args_p: Value,
    out_p: Value,
    /// The status parameter of the landing block currently being emitted, or
    /// `None` for an ordinary block — see [`is_landing`].
    landing_status: Option<Value>,
    /// Which of `f`'s blocks to emit, in the order to emit them —
    /// [`reachable_in_reverse_postorder`], computed once before any Cranelift
    /// block was created and reused here so the two cannot disagree.
    order: Vec<usize>,
    /// Whether this function still owes the call-stack compare
    /// [`Self::emit_stack_check`] emits — see [`is_leaf`], and
    /// [`Self::emit_safepoint`] for why taking it at the first poll lands it
    /// at function entry.
    stack_check_pending: bool,
}

impl Emitter<'_, '_> {
    fn emit_blocks(&mut self) -> Result<(), CodegenError> {
        for index in std::mem::take(&mut self.order) {
            let block = &self.f.blocks[index];
            let start = self.block(block.id)?;
            self.b.switch_to_block(start);

            let phis = *self
                .phi_counts
                .get(&block.id.index())
                .ok_or_else(|| internal("a block with no recorded phi count"))?;
            for (index, inst) in block.insts[..phis].iter().enumerate() {
                let result = inst
                    .result
                    .ok_or_else(|| internal("a phi defining no value"))?;
                let ty = inst
                    .ty
                    .ok_or_else(|| internal("a phi with no representation type"))?;
                let param = self.b.block_params(start)[index];
                self.values.insert(result.index(), (param, ty));
            }

            self.landing_status =
                if is_landing(block) {
                    Some(*self.b.block_params(start).get(phis).ok_or_else(|| {
                        internal("a landing block with no status parameter appended")
                    })?)
                } else {
                    None
                };

            let mut cur = start;
            for inst in &block.insts[phis..] {
                cur = self.emit_inst(cur, inst)?;
            }
            self.emit_terminator(cur, block)?;
        }
        Ok(())
    }

    fn block(&self, id: BlockId) -> Result<Block, CodegenError> {
        self.blocks
            .get(&id.index())
            .copied()
            .ok_or_else(|| internal("a jump to a block that does not exist"))
    }

    fn value(&self, id: ValueId) -> Result<(Value, Ty), CodegenError> {
        self.values
            .get(&id.index())
            .copied()
            .ok_or_else(|| internal("an operand used before it is defined"))
    }

    fn define(&mut self, inst: &Inst, value: Value) -> Result<(), CodegenError> {
        let result = inst
            .result
            .ok_or_else(|| internal("an instruction produced a value the IR did not name"))?;
        let ty = inst
            .ty
            .ok_or_else(|| internal("a value-defining instruction with no representation"))?;
        self.values.insert(result.index(), (value, ty));
        Ok(())
    }

    // -- instructions -------------------------------------------------------

    fn emit_inst(&mut self, cur: Block, inst: &Inst) -> Result<Block, CodegenError> {
        match &inst.kind {
            InstKind::StmtMarker(stmt) => return self.emit_stmt_probe(cur, stmt.index()),
            InstKind::Safepoint => return self.emit_safepoint(cur),
            InstKind::ConstBool(v) => {
                let v = self.b.ins().iconst(types::I8, i64::from(*v));
                self.define(inst, v)?;
            }
            InstKind::ConstInt(v) => {
                let v = self.b.ins().iconst(types::I64, *v);
                self.define(inst, v)?;
            }
            InstKind::ConstUint(v) => {
                let v = self.b.ins().iconst(types::I64, v.cast_signed());
                self.define(inst, v)?;
            }
            InstKind::ConstFloat(v) => {
                let v = self.b.ins().f64const(*v);
                self.define(inst, v)?;
            }
            // ADR 0054 § 2's literal, folded to its sixteen-byte image. This
            // is the one place `mwl_ir`'s three-part constant and
            // `mwl_runtime::decimal`'s bit layout meet — the IR carries the
            // parts because it does not depend on the runtime, and this crate
            // depends on both.
            InstKind::ConstDecimal {
                negative,
                mantissa,
                scale,
            } => {
                let value = MwlDecimal::new(*negative, *mantissa, *scale)
                    .ok_or_else(|| internal("a `decimal` constant outside ADR 0054 § 1's range"))?;
                let bits = value.to_bits();
                let word = |bits: u128| -> Result<i64, CodegenError> {
                    Ok(u64::try_from(bits & u128::from(u64::MAX))
                        .map_err(|_| internal("a `decimal` word past u64"))?
                        .cast_signed())
                };
                let (low, high) = (word(bits)?, word(bits >> 64)?);
                let low = self.b.ins().iconst(types::I64, low);
                let high = self.b.ins().iconst(types::I64, high);
                let value = self.join_tagged(low, high);
                self.define(inst, value)?;
            }
            // `null` is a zero payload under a `Tag::Null` tag byte, and the
            // tag comes from `Ty::Null` at whatever slot this value is stored
            // into — so the value itself is just the zero.
            InstKind::ConstNull => {
                let v = self.b.ins().iconst(types::I64, 0);
                self.define(inst, v)?;
            }
            InstKind::ConstStr(text) => {
                let (value, next) = self.emit_const_str(cur, text.as_bytes())?;
                self.define(inst, value)?;
                return Ok(next);
            }
            // The same emission as `ConstStr`, and deliberately not a second
            // one: a `bytes` is a `string`'s allocation minus the UTF-8
            // promise, so the only thing that differs is the `Ty` this
            // instruction carries — which is what decides the tag a boxed
            // value gets (`crate::ty::tag_of`).
            InstKind::ConstBytes(octets) => {
                let (value, next) = self.emit_const_str(cur, octets)?;
                self.define(inst, value)?;
                return Ok(next);
            }
            InstKind::Param(index) => {
                let ty = inst
                    .ty
                    .ok_or_else(|| internal("a parameter read with no representation"))?;
                let offset = i32::try_from(*index)
                    .map_err(|_| internal("a parameter index past i32"))?
                    * VALUE_SIZE;
                let value = self.load_value(self.args_p, offset, ty)?;
                self.define(inst, value)?;
            }
            InstKind::BinOp { op, lhs, rhs } => {
                let (value, next) = self.emit_binop(cur, inst, *op, *lhs, *rhs)?;
                self.define(inst, value)?;
                return Ok(next);
            }
            InstKind::UnOp { op, operand } => {
                let (value, next) = self.emit_unop(cur, inst, *op, *operand)?;
                self.define(inst, value)?;
                return Ok(next);
            }
            InstKind::HelperCall { helper, args } => {
                let symbol = helper_symbol(*helper)?;
                // One row of the table is variadic — the closure call, whose
                // arity is the call site's and not the helper's — so it is
                // the one that passes a count. See `mwl_ir::Helper::CallClosure`.
                let sig = match helper {
                    Helper::CallClosure => RuntimeSig::HelperVariadic,
                    _ => RuntimeSig::Helper,
                };
                return self.emit_helper(cur, inst, symbol, args, sig);
            }
            // A `Core` member is native Rust behind the same ADR 0002 helper
            // entry point every runtime helper uses, so it needs no path of
            // its own here beyond naming a symbol `mwl-stdlib` registered
            // instead of one this crate's own `Helper` table does. See
            // `mwl_ir::ir::InstKind::CoreCall`.
            InstKind::CoreCall { symbol, args } => {
                return self.emit_helper(cur, inst, symbol, args, RuntimeSig::Helper);
            }
            InstKind::Call {
                target,
                receiver,
                args,
            } => {
                return self.emit_call(cur, inst, target, *receiver, args);
            }
            InstKind::New { class, ctor, args } => {
                let desc = self.class_desc_const(class)?;
                return self.emit_new(cur, inst, desc, ctor.as_deref(), false, args);
            }
            InstKind::NewDynamic { desc, ctor, args } => {
                let (desc, _) = self.value(*desc)?;
                return self.emit_new(cur, inst, desc, ctor.as_deref(), true, args);
            }
            InstKind::ClassDescConst { class } => {
                let desc = self.class_desc_const(class)?;
                self.define(inst, desc)?;
            }
            InstKind::ClassDescOf { object } => {
                let (object, _) = self.value(*object)?;
                let offset = i32::try_from(mwl_runtime::OBJ_CLASS_OFFSET)
                    .map_err(|_| internal("a class-pointer offset past i32"))?;
                let desc = self.b.ins().load(types::I64, trusted(), object, offset);
                self.define(inst, desc)?;
            }
            InstKind::CallVirtual {
                lsb,
                method,
                fallback,
                receiver,
                args,
            } => {
                return self.emit_call_virtual(
                    inst,
                    *lsb,
                    method,
                    fallback.as_deref(),
                    *receiver,
                    args,
                );
            }
            InstKind::FieldGet {
                object,
                class,
                field,
            } => {
                let value = self.emit_field_get(inst, *object, class, field)?;
                self.define(inst, value)?;
            }
            InstKind::SlotGet {
                object,
                field,
                slot,
            } => {
                return self.emit_slot_get(cur, inst, *object, field, *slot);
            }
            InstKind::SlotSet {
                object,
                field,
                slot,
                value,
            } => {
                return self.emit_slot_set(inst, *object, field, *slot, *value);
            }
            InstKind::FieldSet {
                object,
                class,
                field,
                value,
            } => {
                self.emit_field_set(*object, class, field, *value)?;
            }
            InstKind::StaticGet { class, name } => {
                let value = self.emit_static_get(inst, class, name)?;
                self.define(inst, value)?;
            }
            InstKind::StaticSet { class, name, value } => {
                self.emit_static_set(class, name, *value)?;
            }
            InstKind::InstanceOf { value, class } => {
                let result = self.emit_instanceof(*value, class)?;
                self.define(inst, result)?;
            }
            InstKind::Concat { pieces } => {
                let value = self.emit_concat(pieces)?;
                self.define(inst, value)?;
            }
            InstKind::StrAppend { target, suffix } => {
                let value = self.emit_str_append(*target, *suffix)?;
                self.define(inst, value)?;
            }
            InstKind::Clone { object } => {
                let (object, _) = self.value(*object)?;
                let callee = self.runtime_ref("mwl_object_clone", RuntimeSig::PtrToPtr)?;
                let call = self.b.ins().call(callee, &[object]);
                let value = self.b.inst_results(call)[0];
                self.define(inst, value)?;
            }
            // No machine instruction at all: the operand's own Cranelift value
            // is recorded a second time under this instruction's id, with the
            // new representation. See `mwl_ir::ir::InstKind::Reinterpret` for
            // why the IR spends an instruction on a relabelling.
            InstKind::Reinterpret { operand } => {
                let (value, from) = self.value(*operand)?;
                let to = inst.ty.ok_or_else(|| {
                    internal("a value-defining instruction with no representation")
                })?;
                if crate::ty::clif_ty(from) != crate::ty::clif_ty(to) {
                    return Err(CodegenError::Unsupported(format!(
                        "`reinterpret` between {from:?} and {to:?}, which do not share a machine                          type — it is a relabelling, never a bit cast"
                    )));
                }
                self.define(inst, value)?;
            }
            // The three tagged-value instructions. None of them calls, none
            // allocates, and only `IsNull` reads a tag — see
            // `mwl_ir::Ty::Tagged` for the representation all three assume.
            InstKind::Tag { operand } => {
                let (value, from) = self.value(*operand)?;
                // A `decimal` already *is* a `Value` at the tagged width, so
                // widening one into a `mixed`/`?decimal` is the identity —
                // see `mwl_ir::Ty::Decimal`.
                if from == Ty::Decimal {
                    self.define(inst, value)?;
                    return Ok(cur);
                }
                let tag = tag_of(from)?;
                let tag_word = self.b.ins().iconst(types::I64, i64::from(tag as u8));
                let bits = match from {
                    // The payload is eight bytes; a `bool` occupies one of
                    // them, so the rest are zeroed rather than left as
                    // whatever the register held — `Self::store_value`'s own
                    // rule, applied one step earlier.
                    Ty::Bool => self.b.ins().uextend(types::I64, value),
                    // A float's payload is its bit pattern, which is what the
                    // `Value` slot holds and what `Tag::Float` promises.
                    Ty::Float => self.b.ins().bitcast(types::I64, MemFlagsData::new(), value),
                    Ty::Void | Ty::Tagged => {
                        return Err(CodegenError::Unsupported(format!(
                            "widening a value of representation {from:?} into a tagged one"
                        )));
                    }
                    _ => value,
                };
                let value = self.join_tagged(tag_word, bits);
                self.define(inst, value)?;
            }
            InstKind::Untag { operand } => {
                let (value, from) = self.value(*operand)?;
                if from != Ty::Tagged {
                    return Err(internal("`untag` of a value that is not tagged"));
                }
                let to = inst.ty.ok_or_else(|| {
                    internal("a value-defining instruction with no representation")
                })?;
                // Two identities. A `decimal` for `InstKind::Tag`'s reason
                // exactly; a still-tagged target because narrowing a union to
                // a *narrower union* is a checker fact, not a change of
                // representation — which is what `??` over a `?(float|decimal)`
                // asks for.
                if matches!(to, Ty::Decimal | Ty::Tagged) {
                    self.define(inst, value)?;
                    return Ok(cur);
                }
                let (_, bits) = self.split_tagged(value);
                let value = match to {
                    Ty::Bool => self.b.ins().ireduce(types::I8, bits),
                    Ty::Float => self.b.ins().bitcast(types::F64, MemFlagsData::new(), bits),
                    Ty::Void => {
                        return Err(CodegenError::Unsupported(format!(
                            "narrowing a tagged value to representation {to:?}"
                        )));
                    }
                    _ => bits,
                };
                self.define(inst, value)?;
            }
            InstKind::IsNull { operand } => {
                let (value, from) = self.value(*operand)?;
                if from != Ty::Tagged {
                    return Err(internal("`is.null` of a value that is not tagged"));
                }
                let (tag_word, _) = self.split_tagged(value);
                // `Tag::Null` is the only tag whose byte is zero, and every
                // producer leaves the rest of the word zero outside the bytes
                // its own representation uses (`crate::ty::clif_ty`), so this
                // is one compare against zero with nothing to mask off.
                let value = self.b.ins().icmp_imm_u(IntCC::Equal, tag_word, 0);
                self.define(inst, value)?;
            }
            InstKind::ArrayNew { entries } => {
                let value = self.emit_array_new(entries)?;
                self.define(inst, value)?;
            }
            InstKind::ArrayGet { array, key, absent } => {
                // The key's own representation picks nothing here: both entry
                // points tell a rendered key from an `int` by its tag. What
                // `absent` picks is the answer to a missing one, and with it
                // whether `inst.on_error` is `Some` — see `mwl_ir::AbsentKey`.
                let symbol = match absent {
                    AbsentKey::Throws => "mwl_array_required_get",
                    AbsentKey::Null => "mwl_array_optional_get",
                };
                return self.emit_helper(cur, inst, symbol, &[*array, *key], RuntimeSig::Helper);
            }
            InstKind::ArraySet { array, key, value } => {
                // The key operand's own representation picks the primitive,
                // exactly as in `Self::emit_array_get`.
                let (symbol, sig) = if self.value(*key)?.1 == Ty::Int {
                    ("mwl_array_set_index", RuntimeSig::ArraySetIndex)
                } else {
                    ("mwl_array_set", RuntimeSig::ArraySet)
                };
                let result = self.emit_array_write(symbol, sig, *array, Some(*key), *value)?;
                self.define(inst, result)?;
            }
            InstKind::ArrayAppend { array, value } => {
                return self.emit_array_append(inst, *array, *value);
            }
            InstKind::ArraySpread { array, subject } => {
                return self.emit_array_spread(inst, *array, *subject);
            }
            InstKind::ArrayUnset { array, key } => {
                let (array, _) = self.value(*array)?;
                let (key, _) = self.value(*key)?;
                let callee = self.runtime_ref("mwl_array_unset", RuntimeSig::ArrayUnset)?;
                let call = self.b.ins().call(callee, &[array, key]);
                let result = self.b.inst_results(call)[0];
                self.define(inst, result)?;
            }
            InstKind::ArrayNextSlot { array, from } => {
                let (array, _) = self.value(*array)?;
                let (from, _) = self.value(*from)?;
                let callee = self.runtime_ref("mwl_array_next_slot", RuntimeSig::ArrayNextSlot)?;
                let call = self.b.ins().call(callee, &[array, from]);
                let result = self.b.inst_results(call)[0];
                self.define(inst, result)?;
            }
            InstKind::ArrayKeyAt { array, slot } => {
                let (array, _) = self.value(*array)?;
                let (slot, _) = self.value(*slot)?;
                let callee = self.runtime_ref("mwl_array_key_at", RuntimeSig::ArrayKeyAt)?;
                let call = self.b.ins().call(callee, &[array, slot]);
                let result = self.b.inst_results(call)[0];
                self.define(inst, result)?;
            }
            InstKind::ArrayValueAt { array, slot } => {
                let ty = inst
                    .ty
                    .ok_or_else(|| internal("a foreach value binding with no representation"))?;
                let (array, _) = self.value(*array)?;
                let (slot, _) = self.value(*slot)?;
                let out = self.value_slot();
                let callee = self.runtime_ref("mwl_array_value_at", RuntimeSig::ArrayValueAt)?;
                self.b.ins().call(callee, &[array, slot, out]);
                let value = self.load_value(out, 0, ty)?;
                self.define(inst, value)?;
            }
            // A by-reference parameter's caller-staged one-cell slot — see
            // `mwl_ir::Ty::Ref`, which owns the representation decision. All
            // three arms are pure address/load/store: the retain and release
            // that keep the slot owning exactly one reference are ordinary
            // `Retain`/`Release` instructions `mwl_ir::lower` emits around
            // them, so nothing here has an ownership rule of its own.
            InstKind::RefSlot { init } => {
                let (value, ty) = self.value(*init)?;
                let slot = self.value_slot();
                self.store_value(slot, 0, value, ty)?;
                self.define(inst, slot)?;
            }
            InstKind::RefLoad { slot } => {
                let ty = inst
                    .ty
                    .ok_or_else(|| internal("a by-reference read with no representation"))?;
                let (slot, _) = self.value(*slot)?;
                let value = self.load_value(slot, 0, ty)?;
                self.define(inst, value)?;
            }
            InstKind::RefStore { slot, value } => {
                let (slot, _) = self.value(*slot)?;
                let (value, ty) = self.value(*value)?;
                self.store_value(slot, 0, value, ty)?;
            }
            InstKind::Retain { operand } => {
                let (value, ty) = self.value(*operand)?;
                self.emit_refcount(true, value, ty)?;
            }
            InstKind::Release { operand } => {
                let (value, ty) = self.value(*operand)?;
                self.emit_refcount(false, value, ty)?;
            }
            InstKind::TakeThrown => {
                let callee = self.runtime_ref("mwl_take_thrown", RuntimeSig::PtrToPtr)?;
                let call = self.b.ins().call(callee, &[self.ctx_p]);
                let value = self.b.inst_results(call)[0];
                self.define(inst, value)?;
            }
            InstKind::Phi { .. } => return Err(internal("a phi reached the instruction walk")),
            other => {
                return Err(CodegenError::Unsupported(describe(other)));
            }
        }
        Ok(cur)
    }

    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
    /// call-stack limit: one load of [`mwl_runtime::Ctx`]'s third word, one
    /// compare against this frame's stack pointer, one predicted-not-taken
    /// branch, and an out-of-line call to
    /// [`mwl_runtime::mwl_stack_check`] whose status is checked like any other.
    ///
    /// The stack grows down, so exhaustion is an *unsigned less-than*: the
    /// addresses are real ones, and a request whose bounds were armed from a
    /// low address must not have the compare wrap.
    ///
    /// Only the **soft** address is compared here. Which of the ADR's two
    /// tiers a crossing is gets decided in the slow path, which is what makes
    /// two tiers cost exactly what one costs at the site.
    fn emit_stack_check(&mut self, _cur: Block) -> Result<Block, CodegenError> {
        let offset = i32::try_from(STACK_LIMIT_OFFSET)
            .map_err(|_| internal("the stack-limit word sits past a 2 GiB offset"))?;
        let limit = self
            .b
            .ins()
            .load(types::I64, ctx_word(), self.ctx_p, offset);
        let sp = self.b.ins().get_stack_pointer(types::I64);
        let past = self.b.ins().icmp(IntCC::UnsignedLessThan, sp, limit);

        let slow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(past, slow, &[], cont, &[]);

        self.b.switch_to_block(slow);
        let callee = self.runtime_ref("mwl_stack_check", RuntimeSig::StackCheck)?;
        let call = self.b.ins().call(callee, &[self.ctx_p, sp]);
        let status = self.b.inst_results(call)[0];
        let stop = self.b.create_block();
        self.b.ins().brif(status, stop, &[], cont, &[]);

        self.b.switch_to_block(stop);
        self.b.ins().return_(&[status]);

        self.b.switch_to_block(cont);
        Ok(cont)
    }

    /// The safepoint poll: one load of [`mwl_runtime::Ctx`]'s first word, one
    /// predicted-not-taken branch, and an out-of-line call to
    /// [`mwl_runtime::mwl_safepoint`] whose status is checked like any other.
    ///
    /// This is also where the call-stack check rides, at the **first**
    /// safepoint of a non-leaf function — which is the function-entry one,
    /// since `mwl_ir::lower` emits that before anything else and the entry
    /// block is first in [`reachable_in_reverse_postorder`]. A loop back
    /// edge's poll gets no check: going round a loop does not grow the stack.
    fn emit_safepoint(&mut self, cur: Block) -> Result<Block, CodegenError> {
        if std::mem::take(&mut self.stack_check_pending) {
            self.emit_stack_check(cur)?;
        }
        let offset = i32::try_from(SAFEPOINT_OFFSET)
            .map_err(|_| internal("the safepoint word sits past a 2 GiB offset"))?;
        let flags = self
            .b
            .ins()
            .load(types::I64, ctx_word(), self.ctx_p, offset);

        let slow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(flags, slow, &[], cont, &[]);

        self.b.switch_to_block(slow);
        let callee = self.runtime_ref("mwl_safepoint", RuntimeSig::Safepoint)?;
        let call = self.b.ins().call(callee, &[self.ctx_p]);
        let status = self.b.inst_results(call)[0];
        let stop = self.b.create_block();
        self.b.ins().brif(status, stop, &[], cont, &[]);

        self.b.switch_to_block(stop);
        self.b.ins().return_(&[status]);

        self.b.switch_to_block(cont);
        Ok(cont)
    }

    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's statement-boundary probe: the identical load-and-branch shape as
    /// [`Self::emit_safepoint`], against the *second* hot word, with no status
    /// to check because [`mwl_runtime::mwl_probe_stmt`] cannot fail.
    ///
    /// Emitted unconditionally, in every compiled unit, whether or not any
    /// request ever sets a bit — that is the entire mechanism, and the reason
    /// coverage can be switched on for a request already running.
    fn emit_stmt_probe(&mut self, _cur: Block, stmt: u32) -> Result<Block, CodegenError> {
        let offset = i32::try_from(DEBUG_FLAGS_OFFSET)
            .map_err(|_| internal("the debug-flags word sits past a 2 GiB offset"))?;
        let flags = self
            .b
            .ins()
            .load(types::I64, ctx_word(), self.ctx_p, offset);

        let slow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(flags, slow, &[], cont, &[]);

        self.b.switch_to_block(slow);
        let callee = self.runtime_ref("mwl_probe_stmt", RuntimeSig::Probe)?;
        let id = self.b.ins().iconst(types::I32, i64::from(stmt));
        self.b.ins().call(callee, &[self.ctx_p, id]);
        self.b.ins().jump(cont, &[]);

        self.b.switch_to_block(cont);
        Ok(cont)
    }

    /// A string literal: **one address, no call and no allocation**, whatever
    /// it costs to reach the expression it sits in.
    ///
    /// The whole string — a `mwl_runtime::StrHeader` and then the payload —
    /// goes into the unit's data section, so `$a["beta"]` inside a loop
    /// materializes a constant pointer rather than an `mwl_str_new` per
    /// evaluation. The header's refcount is
    /// [`mwl_runtime::IMMORTAL_REFCOUNT`], which every retain and release
    /// compares against and steps over; the data object is declared
    /// **not writable**, so a lapse in that protocol faults here instead of
    /// silently corrupting a word two requests share.
    ///
    /// The layout stays `mwl-runtime`'s: this function asks
    /// [`mwl_runtime::immortal_header_bytes`] for the bytes and does not know
    /// what is in them. What it does own is the *alignment* — a data section
    /// object has no allocator to pick one, so the header's own
    /// [`mwl_runtime::HEADER_ALIGN`] is stated here.
    fn emit_const_str(&mut self, cur: Block, bytes: &[u8]) -> Result<(Value, Block), CodegenError> {
        let value = self.emit_immortal_str(bytes)?;
        Ok((value, cur))
    }

    /// Puts a whole immortal string — header and payload — in the unit's data
    /// section and materializes its address.
    ///
    /// Every call site gets its own data object: identical literals are not
    /// shared, which is the crate docs' known gap 4 and costs a few bytes of
    /// unit rather than anything on the request path.
    fn emit_immortal_str(&mut self, bytes: &[u8]) -> Result<Value, CodegenError> {
        let mut object =
            Vec::with_capacity(mwl_runtime::PAYLOAD_OFFSET.saturating_add(bytes.len()));
        object.extend_from_slice(&mwl_runtime::immortal_header_bytes(bytes.len()));
        object.extend_from_slice(bytes);

        let mut desc = DataDescription::new();
        desc.define(object.into_boxed_slice());
        desc.set_align(
            u64::try_from(mwl_runtime::HEADER_ALIGN)
                .map_err(|_| internal("a string header's alignment past u64"))?,
        );
        self.define_literal(&desc)
    }

    /// Puts `bytes` in the unit's data section and materializes its address
    /// and length as two values — the shape every runtime primitive taking
    /// static bytes wants (`mwl_str_new`, and ADR 0018's call-site probes).
    fn emit_bytes(&mut self, bytes: &[u8]) -> Result<(Value, Value), CodegenError> {
        let mut desc = DataDescription::new();
        // A zero-length literal still needs a real address to hand to its
        // consumer, which ignores the pointer when the length is zero but is
        // handed one regardless; a one-byte object is the cheapest way to keep
        // the two cases identical here.
        desc.define(if bytes.is_empty() {
            Box::new([0_u8]) as Box<[u8]>
        } else {
            bytes.to_vec().into_boxed_slice()
        });
        let address = self.define_literal(&desc)?;
        let len = self.b.ins().iconst(
            types::I64,
            i64::try_from(bytes.len()).map_err(|_| internal("a literal past i64 bytes"))?,
        );
        Ok((address, len))
    }

    /// Defines one read-only data object in the unit under a fresh name and
    /// materializes its address.
    ///
    /// Read-only because nothing this crate emits into a data section is ever
    /// written: a probe's bytes, a field name, and — since a literal became an
    /// immortal string — a `StrHeader` whose refcount is pinned exactly so it
    /// stays that way. Anything that did write one would be a bug in two
    /// requests at once, so the mapping is the place to catch it.
    fn define_literal(&mut self, desc: &DataDescription) -> Result<Value, CodegenError> {
        let name = format!("mwl_bytes_{}", *self.literals);
        *self.literals += 1;

        let data = self
            .module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|source| CodegenError::Cranelift {
                function: self.f.name.clone(),
                source: Box::new(source),
            })?;
        self.module
            .define_data(data, desc)
            .map_err(|source| CodegenError::Cranelift {
                function: self.f.name.clone(),
                source: Box::new(source),
            })?;

        let global = self.module.declare_data_in_func(data, self.b.func);
        Ok(self.b.ins().symbol_value(types::I64, global))
    }

    /// Emits one [`InstKind::BinOp`], returning both its value and the block
    /// execution continues in — which is the *entry* block for every operator
    /// but integer `%`, whose zero-divisor guard splits the flow (see
    /// [`Self::emit_int_mod`]).
    fn emit_binop(
        &mut self,
        cur: Block,
        inst: &Inst,
        op: BinOp,
        lhs: ValueId,
        rhs: ValueId,
    ) -> Result<(Value, Block), CodegenError> {
        let (l, ty) = self.value(lhs)?;
        let (r, rty) = self.value(rhs)?;
        if ty != rty {
            return Err(internal(
                "a binary operator over mismatched representations",
            ));
        }

        // A `string` or `array` comparison is a *content* comparison in the
        // runtime, not a machine instruction: both representations are a
        // pointer, so `icmp` would compare identity, which is never what `==`
        // means for either (ADR 0090 § 3's string and array rows). Each takes
        // a two-pointer call rather than the tagged helper convention because
        // the row is already known here — see `mwl_runtime::mwl_array_eq`.
        if matches!(ty, Ty::Str | Ty::Bytes | Ty::Array) && matches!(op, BinOp::Eq | BinOp::NotEq) {
            let symbol = if matches!(ty, Ty::Array) {
                "mwl_array_eq"
            } else {
                "mwl_str_eq"
            };
            let callee = self.runtime_ref(symbol, RuntimeSig::PtrEq)?;
            let call = self.b.ins().call(callee, &[l, r]);
            let equal = self.b.inst_results(call)[0];
            return Ok((
                match op {
                    BinOp::Eq => equal,
                    // `bxor 1` rather than `icmp_imm 0`: the helper returns a
                    // Rust `bool`, so the byte is already exactly 0 or 1.
                    _ => self.b.ins().bxor_imm_u(equal, 1),
                },
                cur,
            ));
        }

        // ADR 0090 § 3's object row is *identity*, so here the pointer
        // comparison the two rows above refuse is exactly right — and it is
        // one instruction, which is why an object pair calls nothing at all.
        // Comparing contents is `Comparable::compareTo`, a method call that
        // never reaches this instruction (`mwl_ir`'s `lower_object_comparison`).
        if matches!(ty, Ty::Object) && matches!(op, BinOp::Eq | BinOp::NotEq) {
            let cc = if matches!(op, BinOp::Eq) {
                IntCC::Equal
            } else {
                IntCC::NotEqual
            };
            return Ok((self.b.ins().icmp(cc, l, r), cur));
        }

        let signed = matches!(ty, Ty::Int);
        let float = matches!(ty, Ty::Float);
        let integral = matches!(ty, Ty::Int | Ty::Uint | Ty::Bool);
        if !float && !integral {
            return Err(CodegenError::Unsupported(format!(
                "a `{op:?}` over representation {ty:?}"
            )));
        }

        // The one operator whose flow is not straight-line — it owns its own
        // continuation block, so it returns rather than falling through to the
        // single-value tail below.
        if matches!(op, BinOp::Mod) && matches!(ty, Ty::Int | Ty::Uint) {
            return self.emit_int_mod(inst, l, r, signed);
        }
        // The other one, and for a second reason on top of the trap it also
        // guards: ADR 0007 § 4 types integer `/` as a union, so its result is
        // a *tagged* value rather than this function's one representation and
        // it could not join the table below either. See `Self::emit_int_div`.
        if matches!(op, BinOp::Div) && matches!(ty, Ty::Int | Ty::Uint) {
            return self.emit_int_div(inst, l, r, signed);
        }
        // The remaining three integer rows, which own a continuation block for
        // the same reason: ADR 0007 § 4 makes `+`, `-` and `*` throw on
        // overflow rather than wrap. See `Self::emit_checked_int_arith`.
        if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) && matches!(ty, Ty::Int | Ty::Uint) {
            return self.emit_checked_int_arith(inst, op, l, r, signed);
        }
        // And the two shifts, whose count is a value PHP judges rather than a
        // field the machine masks. See `Self::emit_shift`.
        if matches!(op, BinOp::Shl | BinOp::Shr) && matches!(ty, Ty::Int | Ty::Uint) {
            return self.emit_shift(cur, inst, op, l, r, signed);
        }
        // And `**`, the one row in this table that is not a bounded run of
        // instructions at all. See `Self::emit_int_pow`.
        if matches!(op, BinOp::Pow) && matches!(ty, Ty::Int | Ty::Uint) {
            return self.emit_int_pow(inst, l, r, signed);
        }

        let value = match op {
            BinOp::Add if float => self.b.ins().fadd(l, r),
            BinOp::Sub if float => self.b.ins().fsub(l, r),
            BinOp::Mul if float => self.b.ins().fmul(l, r),
            BinOp::Div if float => self.b.ins().fdiv(l, r),
            // The one arm here that is a *call*: there is no `fpow`
            // instruction on any target Cranelift supports and no
            // `LibCall::Pow` to defer to, so ADR 0007 § 4's float `**` row has
            // to leave the compiled function. It goes direct rather than
            // through ADR 0002's helper convention for the reason this
            // module's docs give — the row's representation is already known
            // here and `f64::powf` raises nothing — which is the same trade
            // the `mwl_str_eq` arm above makes.
            BinOp::Pow if float => {
                let callee = self.runtime_ref("mwl_float_pow", RuntimeSig::FloatPow)?;
                let call = self.b.ins().call(callee, &[l, r]);
                self.b.inst_results(call)[0]
            }
            // Reached only by `Ty::Bool`, the third member of `integral`
            // above: every `Ty::Int`/`Ty::Uint` pair has already gone to
            // `Self::emit_checked_int_arith`, which is where ADR 0007 § 4's
            // overflow throw lives. No `bool` arithmetic exists in the
            // language, so in practice these three arms are the table's
            // exhaustiveness and nothing else.
            BinOp::Add => self.b.ins().iadd(l, r),
            BinOp::Sub => self.b.ins().isub(l, r),
            BinOp::Mul => self.b.ins().imul(l, r),
            // ADR 0007 § 4 preserves the operand type across these three and
            // they cannot fail, so unlike the shifts they are one instruction
            // in the straight-line table. `Ty::Bool` reaches them too and is
            // exactly right there: a `bool` is one byte holding 0 or 1.
            BinOp::BitAnd => self.b.ins().band(l, r),
            BinOp::BitOr => self.b.ins().bor(l, r),
            BinOp::BitXor => self.b.ins().bxor(l, r),
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                if float {
                    let cc = match op {
                        BinOp::Eq => FloatCC::Equal,
                        BinOp::NotEq => FloatCC::NotEqual,
                        BinOp::Lt => FloatCC::LessThan,
                        BinOp::LtEq => FloatCC::LessThanOrEqual,
                        BinOp::Gt => FloatCC::GreaterThan,
                        _ => FloatCC::GreaterThanOrEqual,
                    };
                    self.b.ins().fcmp(cc, l, r)
                } else {
                    let cc = match (op, signed) {
                        (BinOp::Eq, _) => IntCC::Equal,
                        (BinOp::NotEq, _) => IntCC::NotEqual,
                        (BinOp::Lt, true) => IntCC::SignedLessThan,
                        (BinOp::Lt, false) => IntCC::UnsignedLessThan,
                        (BinOp::LtEq, true) => IntCC::SignedLessThanOrEqual,
                        (BinOp::LtEq, false) => IntCC::UnsignedLessThanOrEqual,
                        (BinOp::Gt, true) => IntCC::SignedGreaterThan,
                        (BinOp::Gt, false) => IntCC::UnsignedGreaterThan,
                        (_, true) => IntCC::SignedGreaterThanOrEqual,
                        (_, false) => IntCC::UnsignedGreaterThanOrEqual,
                    };
                    self.b.ins().icmp(cc, l, r)
                }
            }
            // `<=>`, spelled "less, else equal, else 1" rather than as the
            // tidier `(a > b) - (a < b)`. The two agree everywhere except on a
            // `NaN` operand, where every float comparison is false: the second
            // formula would answer `0` — "equal" — for a pair that is not, and
            // PHP answers `1`. Two `select`s and no branch, which is why the
            // spelling that is correct is also the one that is cheap.
            BinOp::Cmp => {
                let (less, equal) = if float {
                    (
                        self.b.ins().fcmp(FloatCC::LessThan, l, r),
                        self.b.ins().fcmp(FloatCC::Equal, l, r),
                    )
                } else {
                    let cc = if signed {
                        IntCC::SignedLessThan
                    } else {
                        IntCC::UnsignedLessThan
                    };
                    (
                        self.b.ins().icmp(cc, l, r),
                        self.b.ins().icmp(IntCC::Equal, l, r),
                    )
                };
                let below = self.b.ins().iconst(types::I64, -1);
                let same = self.b.ins().iconst(types::I64, 0);
                let above = self.b.ins().iconst(types::I64, 1);
                let not_below = self.b.ins().select(equal, same, above);
                self.b.ins().select(less, below, not_below)
            }
            other => {
                return Err(CodegenError::Unsupported(format!(
                    "the binary operator {other:?}"
                )));
            }
        };
        Ok((value, cur))
    }

    /// Integer `%`, with the two divisors that would otherwise **trap the
    /// whole process** dealt with before the machine instruction runs.
    ///
    /// A trap is a request-isolation failure — AGENTS.md's priority 1 — not a
    /// wrong answer, so neither case may reach `srem`/`urem`:
    ///
    /// * **A zero divisor** throws spec § 10's `ArithmeticError`, carrying
    ///   PHP's own `Modulo by zero` message. That is the branch: compare,
    ///   branch to a block that raises and takes this instruction's error edge
    ///   ([`mwl_ir::ir::Inst::on_error`]), carry on in a fresh one. The
    ///   not-taken side costs a compare and a predicted branch, which is the
    ///   same shape and the same cost as ADR 0002's status check.
    /// * **`i64::MIN % -1`** does not throw, because it is not an overflow:
    ///   `x % -1` is exactly `0` for every `x`, which is representable. PHP 8
    ///   answers `0` here and AGENTS.md's priority 2 keeps that. So the
    ///   divisor is rewritten to `1` when it is `-1` — a compare and a
    ///   `select`, no branch, and `x % 1` is `0` by the same identity.
    ///
    /// The unsigned case needs only the zero guard: `urem` has no second
    /// trapping input.
    ///
    /// The exception is built by [`mwl_runtime::mwl_raise_new`] from a
    /// descriptor address baked in as an `iconst` — see [`crate::Classes`] —
    /// rather than by a helper's `Fault`, which could only ever name
    /// `RuntimeError`.
    fn emit_int_mod(
        &mut self,
        inst: &Inst,
        lhs: Value,
        rhs: Value,
        signed: bool,
    ) -> Result<(Value, Block), CodegenError> {
        let by_zero = self.b.ins().icmp_imm_s(IntCC::Equal, rhs, 0);
        let raise = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(by_zero, raise, &[], cont, &[]);

        self.b.switch_to_block(raise);
        self.raise_arithmetic_error(inst, b"Modulo by zero")?;

        self.b.switch_to_block(cont);
        let value = if signed {
            let minus_one = self.b.ins().icmp_imm_s(IntCC::Equal, rhs, -1);
            let one = self.b.ins().iconst(types::I64, 1);
            let divisor = self.b.ins().select(minus_one, one, rhs);
            self.b.ins().srem(lhs, divisor)
        } else {
            self.b.ins().urem(lhs, rhs)
        };
        Ok((value, cont))
    }

    /// Integer `/`, whose result is ADR 0007 § 4's `int|float` union — "PHP-
    /// exact, so `6/3` is an integer and `7/2` is a float" — and therefore the
    /// one arithmetic operator that produces a [`Ty::Tagged`] value rather
    /// than a machine one.
    ///
    /// Which of the two it produces is a **runtime** question, so it is a
    /// branch and not a type: the quotient is an integer exactly where the
    /// remainder is zero. Three guards stand in front of it, and each is a
    /// trap — a request-isolation failure, AGENTS.md's priority 1 — rather
    /// than a wrong answer, so none may reach `sdiv`/`udiv`:
    ///
    /// * **A zero divisor** throws spec § 10's `ArithmeticError` carrying
    ///   PHP's own `Division by zero` message, exactly as
    ///   [`Self::emit_int_mod`] does and by the same route: the exception is
    ///   built by [`mwl_runtime::mwl_raise_new`] from a baked-in descriptor
    ///   address, since a helper's `Fault` could only ever name `RuntimeError`.
    /// * **`i64::MIN / -1`** is the signed overflow, and it is not an integer
    ///   at all: PHP answers the `float` `9.2233720368548E+18`, which is
    ///   exactly what the inexact arm here computes. So it joins "the
    ///   remainder is not zero" in choosing that arm rather than becoming a
    ///   second throw — no `sdiv` runs on it.
    /// * **`i64::MIN % -1`** would trap the *remainder* the same way, so the
    ///   divisor feeding `srem` is rewritten to `1` when it is `-1`, which
    ///   `emit_int_mod` explains: `x % -1` and `x % 1` are both `0` for every
    ///   `x`. The `sdiv` on the exact arm keeps the real divisor, being
    ///   reachable only where neither overflow guard fired.
    fn emit_int_div(
        &mut self,
        inst: &Inst,
        lhs: Value,
        rhs: Value,
        signed: bool,
    ) -> Result<(Value, Block), CodegenError> {
        let by_zero = self.b.ins().icmp_imm_s(IntCC::Equal, rhs, 0);
        let raise = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(by_zero, raise, &[], cont, &[]);

        self.b.switch_to_block(raise);
        self.raise_arithmetic_error(inst, b"Division by zero")?;

        self.b.switch_to_block(cont);
        let exact = if signed {
            let minus_one = self.b.ins().icmp_imm_s(IntCC::Equal, rhs, -1);
            let one = self.b.ins().iconst(types::I64, 1);
            let divisor = self.b.ins().select(minus_one, one, rhs);
            let remainder = self.b.ins().srem(lhs, divisor);
            let divides = self.b.ins().icmp_imm_s(IntCC::Equal, remainder, 0);
            let min = self.b.ins().iconst(types::I64, i64::MIN);
            let is_min = self.b.ins().icmp(IntCC::Equal, lhs, min);
            let overflows = self.b.ins().band(is_min, minus_one);
            let in_range = self.b.ins().bxor_imm_u(overflows, 1);
            self.b.ins().band(divides, in_range)
        } else {
            let remainder = self.b.ins().urem(lhs, rhs);
            self.b.ins().icmp_imm_s(IntCC::Equal, remainder, 0)
        };

        let integral = self.b.create_block();
        let fractional = self.b.create_block();
        let merge = self.b.create_block();
        self.b.append_block_param(merge, types::I128);
        self.b.ins().brif(exact, integral, &[], fractional, &[]);

        self.b.switch_to_block(integral);
        let quotient = if signed {
            self.b.ins().sdiv(lhs, rhs)
        } else {
            self.b.ins().udiv(lhs, rhs)
        };
        let tag = tag_of(if signed { Ty::Int } else { Ty::Uint })?;
        let tag_word = self.b.ins().iconst(types::I64, i64::from(tag as u8));
        let tagged = self.join_tagged(tag_word, quotient);
        self.b
            .ins()
            .jump(merge, &[codegen::ir::BlockArg::Value(tagged)]);

        self.b.switch_to_block(fractional);
        let (left, right) = if signed {
            (
                self.b.ins().fcvt_from_sint(types::F64, lhs),
                self.b.ins().fcvt_from_sint(types::F64, rhs),
            )
        } else {
            (
                self.b.ins().fcvt_from_uint(types::F64, lhs),
                self.b.ins().fcvt_from_uint(types::F64, rhs),
            )
        };
        let quotient = self.b.ins().fdiv(left, right);
        let bits = self
            .b
            .ins()
            .bitcast(types::I64, MemFlagsData::new(), quotient);
        let tag = tag_of(Ty::Float)?;
        let tag_word = self.b.ins().iconst(types::I64, i64::from(tag as u8));
        let tagged = self.join_tagged(tag_word, bits);
        self.b
            .ins()
            .jump(merge, &[codegen::ir::BlockArg::Value(tagged)]);

        self.b.switch_to_block(merge);
        let value = self.b.block_params(merge)[0];
        Ok((value, merge))
    }

    /// Integer `+`, `-` and `*`, which ADR 0007 § 4 makes **throw
    /// `ArithmeticError`** rather than wrap — "the divergence from PHP this
    /// ADR is least willing to trade", because a silent promotion to `float`
    /// changes a binding's type behind its declaration and a silent wrap is
    /// the classic size-computation bug.
    ///
    /// The check is the machine's own: Cranelift's `sadd_overflow` family
    /// answers the flag the CPU already sets, so the not-taken side costs one
    /// predicted branch on top of the arithmetic instruction and no extra
    /// compare at all — cheaper than the zero-divisor guard next door, which
    /// has to synthesize its own condition. The overflow edge is
    /// [`mwl_ir::ir::Inst::on_error`]'s, exactly as [`Self::emit_int_mod`]'s
    /// is, so this function owns a continuation block and its caller must use
    /// the one it returns.
    ///
    /// The unsigned rows are genuinely different instructions and not the
    /// same ones read differently: `uadd_overflow` reports a carry out of bit
    /// 63 where `sadd_overflow` reports a sign flip, which is what makes
    /// `uint` arithmetic exact over `0 … 2^64−1` rather than over `int`'s
    /// range.
    fn emit_checked_int_arith(
        &mut self,
        inst: &Inst,
        op: BinOp,
        lhs: Value,
        rhs: Value,
        signed: bool,
    ) -> Result<(Value, Block), CodegenError> {
        let (value, overflowed) = match (op, signed) {
            (BinOp::Add, true) => self.b.ins().sadd_overflow(lhs, rhs),
            (BinOp::Add, false) => self.b.ins().uadd_overflow(lhs, rhs),
            (BinOp::Sub, true) => self.b.ins().ssub_overflow(lhs, rhs),
            (BinOp::Sub, false) => self.b.ins().usub_overflow(lhs, rhs),
            (BinOp::Mul, true) => self.b.ins().smul_overflow(lhs, rhs),
            (BinOp::Mul, false) => self.b.ins().umul_overflow(lhs, rhs),
            (other, _) => {
                return Err(internal(&format!(
                    "a checked integer arithmetic guard over {other:?}"
                )));
            }
        };
        let message: &[u8] = match op {
            BinOp::Add => b"Integer addition overflowed",
            BinOp::Sub => b"Integer subtraction overflowed",
            _ => b"Integer multiplication overflowed",
        };
        self.emit_overflow_guard(inst, value, overflowed, message)
    }

    /// `<<` and `>>`, whose count PHP *judges* where the machine merely masks
    /// it — three rules, none of which x86 or aarch64 gives for free:
    ///
    /// * **A negative count throws** `ArithmeticError`, carrying PHP's own
    ///   `Bit shift by negative number` message. Only the signed row can
    ///   produce one, so only it owns the guard and the continuation block —
    ///   which is why `mwl_ir::lower` marks a shift fallible on
    ///   [`mwl_ir::ty::Ty::Int`] and not on `Ty::Uint`.
    /// * **A count of 64 or more answers all-zeros, or all-sign.** `ishl`
    ///   masks the count to 6 bits, so `1 << 64` would be `1`; PHP answers
    ///   `0`. The correction is a `select` on an unsigned compare rather than
    ///   a branch, since both arms are one instruction and neither is cold.
    /// * **`>>` reads its operand's signedness**, ADR 0007 § 4 making it
    ///   arithmetic on an `int` and logical on a `uint`. That is `sshr` versus
    ///   `ushr`, and it is also what the past-the-width arm fills with: the
    ///   sign bit for the first, zero for the second.
    fn emit_shift(
        &mut self,
        cur: Block,
        inst: &Inst,
        op: BinOp,
        lhs: Value,
        rhs: Value,
        signed: bool,
    ) -> Result<(Value, Block), CodegenError> {
        let mut cur = cur;
        if signed {
            let zero = self.b.ins().iconst(types::I64, 0);
            let negative = self.b.ins().icmp(IntCC::SignedLessThan, rhs, zero);
            let raise = self.b.create_block();
            let cont = self.b.create_block();
            self.b.ins().brif(negative, raise, &[], cont, &[]);

            self.b.switch_to_block(raise);
            self.raise_arithmetic_error(inst, b"Bit shift by negative number")?;

            self.b.switch_to_block(cont);
            cur = cont;
        }
        // Unsigned, and correct for both rows: the signed one has already
        // refused every negative count above, so what reaches here is a
        // magnitude either way.
        let width = self.b.ins().iconst(types::I64, 64);
        let past_width = self
            .b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, rhs, width);
        let arithmetic = signed && matches!(op, BinOp::Shr);
        let shifted = match (op, arithmetic) {
            (BinOp::Shl, _) => self.b.ins().ishl(lhs, rhs),
            (_, true) => self.b.ins().sshr(lhs, rhs),
            _ => self.b.ins().ushr(lhs, rhs),
        };
        let saturated = if arithmetic {
            let top = self.b.ins().iconst(types::I64, 63);
            self.b.ins().sshr(lhs, top)
        } else {
            self.b.ins().iconst(types::I64, 0)
        };
        Ok((self.b.ins().select(past_width, saturated, shifted), cur))
    }

    /// Integer `**` — square-and-multiply, with ADR 0007 § 4's overflow throw
    /// checked at **every** step rather than only on the result.
    ///
    /// This is the only operator in the table whose emission is a *loop*, and
    /// it is one because no target has an integer power instruction: the
    /// exponent's bits are walked low to high, the accumulator taking a factor
    /// on each set bit and the running square doubling its exponent on each
    /// step. At most 64 iterations, and the usual small exponent leaves after
    /// two or three.
    ///
    /// Three details are load-bearing:
    ///
    /// * **The square is not taken after the last set bit.** `2 ** 62` would
    ///   otherwise overflow on a `base` nothing then multiplies by, reporting
    ///   a failure for a representable answer. So the exhaustion test happens
    ///   between the multiply and the square, not at the top of the loop only.
    /// * **The product's overflow flag is masked by the bit.** The multiply is
    ///   computed unconditionally — `smul_overflow` cannot straddle two blocks,
    ///   the same constraint [`Self::emit_overflow_guard`] documents — and a
    ///   `select` keeps it or drops it, so its overflow is only a throw on the
    ///   step that actually wanted the factor.
    /// * **A negative exponent throws**, except over a base of `1` or `-1`.
    ///   ADR 0007 § 4's row is "the same type … no wrap, no promotion to
    ///   `float`", so `2 ** -1` has no `int` to answer and PHP's `0.5` is
    ///   exactly the promotion that row refuses; `1 ** -1` and `(-1) ** -3` do
    ///   have one, and answering it costs nothing on the hot path because the
    ///   whole decision sits behind the sign test. A `uint` exponent cannot be
    ///   negative, so the unsigned row carries no guard at all — which is why
    ///   `mwl_ir::lower` marks `**` fallible on both, and only this function
    ///   knows the two rows raise for different reasons.
    fn emit_int_pow(
        &mut self,
        inst: &Inst,
        base: Value,
        exponent: Value,
        signed: bool,
    ) -> Result<(Value, Block), CodegenError> {
        let done = self.b.create_block();
        self.b.append_block_param(done, types::I64);
        let overflow = self.b.create_block();
        // The accumulator, the running square, and what is left of the
        // exponent — carried as block parameters because this is a loop and
        // `FunctionBuilder` has no other way to phi them.
        let header = self.b.create_block();
        for _ in 0..3 {
            self.b.append_block_param(header, types::I64);
        }
        let one = self.b.ins().iconst(types::I64, 1);

        if signed {
            let negative_exponent = self.b.create_block();
            let start = self.b.create_block();
            let negative = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, exponent, 0);
            self.b
                .ins()
                .brif(negative, negative_exponent, &[], start, &[]);

            self.b.switch_to_block(negative_exponent);
            let reciprocal = self.b.create_block();
            let raise = self.b.create_block();
            let is_one = self.b.ins().icmp_imm_s(IntCC::Equal, base, 1);
            let is_minus_one = self.b.ins().icmp_imm_s(IntCC::Equal, base, -1);
            let unit = self.b.ins().bor(is_one, is_minus_one);
            self.b.ins().brif(unit, reciprocal, &[], raise, &[]);

            self.b.switch_to_block(raise);
            self.raise_arithmetic_error(inst, b"Negative exponent has no integer result")?;

            // `base` is `1` or `-1` here, so `1/base` is `base` — and the
            // answer alternates with the exponent's parity for the second and
            // is constantly `1` for the first, which `select` gives at once.
            self.b.switch_to_block(reciprocal);
            let bit = self.b.ins().band_imm_u(exponent, 1);
            let odd = self.b.ins().icmp_imm_u(IntCC::NotEqual, bit, 0);
            let value = self.b.ins().select(odd, base, one);
            self.b
                .ins()
                .jump(done, &[codegen::ir::BlockArg::Value(value)]);

            self.b.switch_to_block(start);
        }
        self.b.ins().jump(
            header,
            &[
                codegen::ir::BlockArg::Value(one),
                codegen::ir::BlockArg::Value(base),
                codegen::ir::BlockArg::Value(exponent),
            ],
        );

        self.b.switch_to_block(header);
        let accumulator = self.b.block_params(header)[0];
        let square = self.b.block_params(header)[1];
        let remaining = self.b.block_params(header)[2];
        let body = self.b.create_block();
        let exhausted = self.b.ins().icmp_imm_u(IntCC::Equal, remaining, 0);
        self.b.ins().brif(
            exhausted,
            done,
            &[codegen::ir::BlockArg::Value(accumulator)],
            body,
            &[],
        );

        self.b.switch_to_block(body);
        let bit = self.b.ins().band_imm_u(remaining, 1);
        let odd = self.b.ins().icmp_imm_u(IntCC::NotEqual, bit, 0);
        let (product, product_overflowed) = if signed {
            self.b.ins().smul_overflow(accumulator, square)
        } else {
            self.b.ins().umul_overflow(accumulator, square)
        };
        let wanted = self.b.ins().band(odd, product_overflowed);
        let multiplied = self.b.create_block();
        self.b.ins().brif(wanted, overflow, &[], multiplied, &[]);

        self.b.switch_to_block(multiplied);
        let next_accumulator = self.b.ins().select(odd, product, accumulator);
        // Logical, on both rows: the signed one refused every negative
        // exponent above, so what is left here is a magnitude either way.
        let next_remaining = self.b.ins().ushr_imm_u(remaining, 1);
        let squaring = self.b.create_block();
        let last = self.b.ins().icmp_imm_u(IntCC::Equal, next_remaining, 0);
        self.b.ins().brif(
            last,
            done,
            &[codegen::ir::BlockArg::Value(next_accumulator)],
            squaring,
            &[],
        );

        self.b.switch_to_block(squaring);
        let (next_square, square_overflowed) = if signed {
            self.b.ins().smul_overflow(square, square)
        } else {
            self.b.ins().umul_overflow(square, square)
        };
        let again = self.b.create_block();
        self.b
            .ins()
            .brif(square_overflowed, overflow, &[], again, &[]);

        self.b.switch_to_block(again);
        self.b.ins().jump(
            header,
            &[
                codegen::ir::BlockArg::Value(next_accumulator),
                codegen::ir::BlockArg::Value(next_square),
                codegen::ir::BlockArg::Value(next_remaining),
            ],
        );

        self.b.switch_to_block(overflow);
        self.raise_arithmetic_error(inst, b"Integer exponentiation overflowed")?;

        self.b.switch_to_block(done);
        let value = self.b.block_params(done)[0];
        Ok((value, done))
    }

    /// The branch shared by every overflow row: throw where `overflowed` is
    /// set, otherwise carry on in a fresh block with `value`.
    ///
    /// `value` is computed *before* the branch on purpose. The overflowing
    /// result is a well-defined wrapped integer that nothing then reads, and
    /// the alternative — computing it on the not-taken side — would put the
    /// arithmetic and the flag it produces in two different blocks, which is
    /// not a shape `sadd_overflow` can take.
    fn emit_overflow_guard(
        &mut self,
        inst: &Inst,
        value: Value,
        overflowed: Value,
        message: &[u8],
    ) -> Result<(Value, Block), CodegenError> {
        let raise = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(overflowed, raise, &[], cont, &[]);

        self.b.switch_to_block(raise);
        self.raise_arithmetic_error(inst, message)?;

        self.b.switch_to_block(cont);
        Ok((value, cont))
    }

    /// Raise spec § 10's `ArithmeticError` inline and leave the current block
    /// on [`mwl_ir::ir::Inst::on_error`]'s edge.
    ///
    /// Every arithmetic throw in this file goes through here — the two zero
    /// divisors and the four overflow rows — and none of them goes through a
    /// helper's `Fault`, which could only ever name `RuntimeError`. The
    /// exception is built by [`mwl_runtime::mwl_raise_new`] from a descriptor
    /// address baked in as an `iconst`, see [`crate::Classes`].
    ///
    /// The caller has already switched to the block this terminates, and must
    /// switch to its own continuation afterwards.
    fn raise_arithmetic_error(&mut self, inst: &Inst, message: &[u8]) -> Result<(), CodegenError> {
        let desc = self.class_desc_const("ArithmeticError")?;
        let (text, len) = self.emit_bytes(message)?;
        let callee = self.runtime_ref("mwl_raise_new", RuntimeSig::RaiseNew)?;
        self.b.ins().call(callee, &[self.ctx_p, desc, text, len]);
        let status = self.b.ins().iconst(types::I32, i64::from(THROWN));
        match inst.on_error {
            Some(landing) => {
                let target = self.block(landing)?;
                self.b
                    .ins()
                    .jump(target, &[codegen::ir::BlockArg::Value(status)]);
            }
            // The pre-error-edge shape `Self::emit_status_check` also keeps
            // for a `None`: return the status onward, releasing nothing.
            // Unreachable from `mwl_ir::lower`, which emits every one of
            // these instructions through `emit_fallible`.
            None => {
                self.b.ins().return_(&[status]);
            }
        }
        Ok(())
    }

    fn emit_unop(
        &mut self,
        cur: Block,
        inst: &Inst,
        op: UnOp,
        operand: ValueId,
    ) -> Result<(Value, Block), CodegenError> {
        let (v, ty) = self.value(operand)?;
        // `-i64::MIN` is the one `int` with no negation, and every non-zero
        // `uint` is a `uint` with none, so ADR 0007 § 4's overflow throw
        // reaches this row too — spelled as `0 - v` because that is exactly
        // what a negation is and `ssub_overflow` already answers it.
        if matches!(op, UnOp::Neg) && matches!(ty, Ty::Int | Ty::Uint) {
            let zero = self.b.ins().iconst(types::I64, 0);
            let (value, overflowed) = if matches!(ty, Ty::Int) {
                self.b.ins().ssub_overflow(zero, v)
            } else {
                self.b.ins().usub_overflow(zero, v)
            };
            return self.emit_overflow_guard(
                inst,
                value,
                overflowed,
                b"Integer negation overflowed",
            );
        }
        let value = match (op, ty) {
            (UnOp::Neg, Ty::Float) => self.b.ins().fneg(v),
            // Total over both integer representations — every 64-bit pattern
            // is a value of each — so `~` needs none of the guard the
            // negation above does.
            (UnOp::BitNot, Ty::Int | Ty::Uint) => self.b.ins().bnot(v),
            (UnOp::Not, Ty::Bool) => {
                let zero = self.b.ins().iconst(types::I8, 0);
                self.b.ins().icmp(IntCC::Equal, v, zero)
            }
            (op, ty) => {
                return Err(CodegenError::Unsupported(format!(
                    "the unary operator {op:?} over representation {ty:?}"
                )));
            }
        };
        Ok((value, cur))
    }

    /// One runtime helper call, in ADR 0002's shape: the arguments
    /// materialized into a stack slot of 16-byte [`mwl_runtime::Value`]s, a
    /// second slot for the result, and the status check after.
    ///
    /// `sig` is [`RuntimeSig::Helper`] for all but one row of the table.
    /// [`RuntimeSig::HelperVariadic`] is the same call with the argument
    /// **count** passed beside the slot, which one helper needs because its
    /// arity is a property of the call site rather than of its own
    /// declaration — see `mwl_ir::Helper::CallClosure`, the only one so far.
    fn emit_helper(
        &mut self,
        cur: Block,
        inst: &Inst,
        symbol: &'static str,
        args: &[ValueId],
        sig: RuntimeSig,
    ) -> Result<Block, CodegenError> {
        let count =
            i32::try_from(args.len()).map_err(|_| internal("a helper call past i32 args"))?;
        let args_p = if args.is_empty() {
            // `mwl_runtime::run_helper` reads no argument slice at arity zero,
            // and splits that case out precisely so a null pointer is legal.
            self.b.ins().iconst(types::I64, 0)
        } else {
            let slot = self.b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (count * VALUE_SIZE).cast_unsigned(),
                VALUE_ALIGN_SHIFT,
            ));
            let base = self.b.ins().stack_addr(types::I64, slot, 0);
            for (index, arg) in args.iter().enumerate() {
                let (value, ty) = self.value(*arg)?;
                let offset = i32::try_from(index)
                    .map_err(|_| internal("a helper call past i32 args"))?
                    * VALUE_SIZE;
                self.store_value(base, offset, value, ty)?;
            }
            base
        };

        let out_slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let out_p = self.b.ins().stack_addr(types::I64, out_slot, 0);

        let callee = self.runtime_ref(symbol, sig)?;
        let call = match sig {
            RuntimeSig::HelperVariadic => {
                let argc = self.b.ins().iconst(types::I64, i64::from(count));
                self.b
                    .ins()
                    .call(callee, &[self.ctx_p, args_p, argc, out_p])
            }
            _ => self.b.ins().call(callee, &[self.ctx_p, args_p, out_p]),
        };
        let status = self.b.inst_results(call)[0];
        let cont = self.emit_status_check(status, inst.on_error)?;

        // `Ty::Void` is filtered out for [`Self::emit_call`]'s reason: a helper
        // still writes its `out` slot, but nothing may *read* one — `void` has
        // no register representation at all (`crate::ty::clif_ty`). The first
        // helper this mattered for is `Core\Time::sleep`.
        if let Some(ty) = inst.ty.filter(|ty| !matches!(ty, Ty::Void)) {
            let value = self.load_value(out_p, 0, ty)?;
            self.define(inst, value)?;
        }
        let _ = cur;
        Ok(cont)
    }

    /// One MWL-level call, in ADR 0002's shape.
    ///
    /// Structurally identical to [`Self::emit_helper`] — arguments
    /// materialized into a stack slot of 16-byte [`mwl_runtime::Value`]s, a
    /// second slot for the result, the compare-and-branch on the returned
    /// status — and deliberately so: ADR 0002 makes one calling convention
    /// normative for *every* call, so a runtime helper and a compiled MWL
    /// method differ here only in which `FuncRef` is called.
    ///
    /// # The receiver slot
    ///
    /// `mwl_ir::lower::lower_method` gives every lowered method an implicit
    /// receiver at parameter index 0, whether or not its body reads `$this`
    /// (see `Function::params`' own doc comment). So argument slot 0 always
    /// exists: an instance call stores its receiver there like any other
    /// argument, and a static call — which has no receiver value at all —
    /// fills it with `null`.
    ///
    /// Ownership follows the same convention every argument does: the caller
    /// retains an aliasing receiver and the callee releases it at scope exit.
    /// `mwl_ir::lower`'s `MethodCall` arm inserts that retain, so nothing here
    /// does.
    fn emit_call(
        &mut self,
        cur: Block,
        inst: &Inst,
        target: &str,
        receiver: Option<ValueId>,
        args: &[ValueId],
    ) -> Result<Block, CodegenError> {
        let receiver = receiver.map(|id| self.value(id)).transpose()?;
        let (cont, out_p) = self.emit_invoke(inst, target, receiver, args)?;
        if let Some(ty) = inst.ty.filter(|ty| !matches!(ty, Ty::Void)) {
            let value = self.load_value(out_p, 0, ty)?;
            self.define(inst, value)?;
        }
        let _ = cur;
        Ok(cont)
    }

    /// `static::method(...)`: look the method up on the late-static-binding
    /// class, then call whatever came back through the ordinary ADR 0002
    /// signature.
    ///
    /// The lookup is one call to `mwl_runtime::mwl_class_method` with the
    /// statically resolved target's own address as the fallback, so there is
    /// no branch and no null to guard — see that helper's docs. Everything
    /// after it (both probes, the argument slots, the status check, the
    /// landing block) is [`Self::emit_invoke`]'s, unchanged: an indirect call
    /// differs from a direct one only in where the callee comes from.
    ///
    /// Unlike [`Self::emit_call`] this takes no current block: it has no use
    /// for one, and the argument budget is already full.
    fn emit_call_virtual(
        &mut self,
        inst: &Inst,
        lsb: ValueId,
        method: &str,
        fallback: Option<&str>,
        receiver: Option<ValueId>,
        args: &[ValueId],
    ) -> Result<Block, CodegenError> {
        let (lsb, _) = self.value(lsb)?;
        let callee = self.method_address(lsb, method, fallback)?;
        let receiver = receiver.map(|id| self.value(id)).transpose()?;
        let receiver = match receiver {
            Some(pair) => Some(pair),
            // A `static` target's slot 0 is the called class itself — the same
            // value the lookup dispatched on. See
            // `mwl_ir::ir::InstKind::CallVirtual`.
            None => Some((lsb, Ty::ClassDesc)),
        };
        // The probe label names the *call site*, which for a bodiless target
        // is the declaration it resolved to.
        let label = fallback.unwrap_or(method).to_owned();
        let (cont, out_p) =
            self.emit_invoke_at(inst, Callee::Indirect(callee), &label, receiver, args)?;
        if let Some(ty) = inst.ty.filter(|ty| !matches!(ty, Ty::Void)) {
            let value = self.load_value(out_p, 0, ty)?;
            self.define(inst, value)?;
        }
        Ok(cont)
    }

    /// The address `class` answers `method` with, falling back to the
    /// statically resolved `fallback` label's own address — or, when the
    /// resolved declaration has no body at all, to
    /// [`mwl_runtime::mwl_abstract_method`], whose whole job is to make that
    /// a reported `FATAL` instead of a jump through null.
    ///
    /// `fallback` is passed as a `func_addr` rather than resolved here: the
    /// unit is not finalized yet, so a compiled function has no address until
    /// Cranelift relocates one in.
    fn method_address(
        &mut self,
        class: Value,
        method: &str,
        fallback: Option<&str>,
    ) -> Result<Value, CodegenError> {
        let (name, len) = self.emit_bytes(method.as_bytes())?;
        let target = match fallback {
            Some(label) => self.callee_ref(label)?,
            None => self.runtime_ref("mwl_abstract_method", RuntimeSig::Helper)?,
        };
        let fallback = self.b.ins().func_addr(types::I64, target);
        let lookup = self.runtime_ref("mwl_class_method", RuntimeSig::ClassMethod)?;
        let call = self.b.ins().call(lookup, &[class, name, len, fallback]);
        Ok(self.b.inst_results(call)[0])
    }

    /// The class descriptor address for a label named in the IR, as one
    /// `iconst` — see [`crate::Classes`] for why a JIT can bake one in.
    fn class_desc_const(&mut self, class: &str) -> Result<Value, CodegenError> {
        let desc = self.classes.desc(class).ok_or_else(|| {
            CodegenError::Unsupported(format!(
                "a reference to class `{class}`, which this unit declares no descriptor for"
            ))
        })?;
        let address = i64::try_from(desc.addr())
            .map_err(|_| internal("a class descriptor above i64::MAX"))?;
        Ok(self.b.ins().iconst(types::I64, address))
    }

    /// The call itself, shared by [`Self::emit_call`] and the constructor
    /// invocation inside [`Self::emit_new`]: arguments materialized, both
    /// probes emitted, the status checked. Returns the block execution
    /// continues in and the `out` slot the callee wrote its result into —
    /// which caller decides what to do with, since `new`'s own result is the
    /// instance rather than anything the constructor returned.
    fn emit_invoke(
        &mut self,
        inst: &Inst,
        target: &str,
        receiver: Option<(Value, Ty)>,
        args: &[ValueId],
    ) -> Result<(Block, Value), CodegenError> {
        let callee = self.callee_ref(target)?;
        self.emit_invoke_at(inst, Callee::Direct(callee), target, receiver, args)
    }

    /// [`Self::emit_invoke`]'s body, with the callee already decided — a
    /// `FuncRef` for a statically resolved target, or a code address for
    /// [`Self::emit_call_virtual`]'s runtime-resolved one. `label` is what the
    /// call probes report either way, which for a virtual call is the
    /// statically resolved target: the probe names the *call site*, and that
    /// is the only spelling of it a compile-time data section can hold.
    fn emit_invoke_at(
        &mut self,
        inst: &Inst,
        callee: Callee,
        label: &str,
        receiver: Option<(Value, Ty)>,
        args: &[ValueId],
    ) -> Result<(Block, Value), CodegenError> {
        let label = self.emit_bytes(label.as_bytes())?;
        self.emit_call_probe("mwl_probe_call_enter", RuntimeSig::ProbeCall, label, None)?;

        // One slot per argument, plus the implicit receiver at index 0.
        let count =
            i32::try_from(args.len() + 1).map_err(|_| internal("a call past i32 arguments"))?;
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (count * VALUE_SIZE).cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let args_p = self.b.ins().stack_addr(types::I64, slot, 0);
        match receiver {
            Some((value, ty)) => self.store_value(args_p, 0, value, ty)?,
            None => self.store_tag_and_bits(args_p, 0, Tag::Null, None)?,
        }
        for (index, arg) in args.iter().enumerate() {
            let (value, ty) = self.value(*arg)?;
            let offset = i32::try_from(index + 1)
                .map_err(|_| internal("a call past i32 arguments"))?
                * VALUE_SIZE;
            self.store_value(args_p, offset, value, ty)?;
        }

        let out_slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let out_p = self.b.ins().stack_addr(types::I64, out_slot, 0);

        let call = match callee {
            Callee::Direct(func) => self.b.ins().call(func, &[self.ctx_p, args_p, out_p]),
            // An indirect call through ADR 0002's one signature — every
            // caller-side obligation above and below it is identical.
            Callee::Indirect(address) => {
                let sig = self.b.import_signature(self.sigs.helper.clone());
                self.b
                    .ins()
                    .call_indirect(sig, address, &[self.ctx_p, args_p, out_p])
            }
        };
        let status = self.b.inst_results(call)[0];
        // Before the status check, so a thrown or `FATAL` exit is traced as it
        // happened rather than skipped along with the rest of the frame.
        self.emit_call_probe(
            "mwl_probe_call_exit",
            RuntimeSig::ProbeCallExit,
            label,
            Some(status),
        )?;
        let cont = self.emit_status_check(status, inst.on_error)?;
        Ok((cont, out_p))
    }

    /// `new Foo(...)`: allocate the instance, then invoke its resolved
    /// constructor on it.
    ///
    /// The IR bundles the two into one instruction
    /// (`mwl_ir::ir::InstKind::New`), so the ownership bookkeeping between
    /// them is this function's rather than lowering's:
    /// `mwl_object_new` returns the one reference the `New` *result* owns, and
    /// the constructor — an ordinary method whose frame releases every
    /// refcounted parameter at scope exit — needs a reference of its own. So
    /// the receiver is retained before the call, exactly the retain
    /// `mwl_ir::lower` inserts at an ordinary `$obj->m()` site.
    ///
    /// The descriptor address is an `iconst`: see [`crate::Classes`] for why a
    /// JIT can bake one in.
    fn emit_new(
        &mut self,
        cur: Block,
        inst: &Inst,
        desc: Value,
        ctor: Option<&str>,
        dynamic: bool,
        args: &[ValueId],
    ) -> Result<Block, CodegenError> {
        let callee = self.runtime_ref("mwl_object_new", RuntimeSig::PtrToPtr)?;
        let call = self.b.ins().call(callee, &[desc]);
        let object = self.b.inst_results(call)[0];
        self.define(inst, object)?;

        let Some(ctor) = ctor else {
            // No constructor anywhere in the chain — `mwl_ir::lower` already
            // asserted the call site passed no arguments, so allocation is the
            // whole of `new`. Every slot is `null`, which ADR 0022 makes
            // unobservable.
            return Ok(cur);
        };
        let retain = self.runtime_ref("mwl_object_retain", RuntimeSig::Refcount)?;
        self.b.ins().call(retain, &[object]);
        // `new Foo(...)` names a class, so its constructor is settled at
        // compile time and costs a direct call. Only `new static(...)`, whose
        // class is a run-time value, pays for a lookup — see
        // `mwl_runtime::mwl_class_method`.
        let callee = if dynamic {
            Callee::Indirect(self.method_address(desc, "constructor", Some(ctor))?)
        } else {
            Callee::Direct(self.callee_ref(ctor)?)
        };
        let (cont, _out) =
            self.emit_invoke_at(inst, callee, ctor, Some((object, Ty::Object)), args)?;
        Ok(cont)
    }

    /// `$obj instanceof Class`: one call, with the descriptor address baked in
    /// exactly the way [`Self::emit_new`] bakes the allocated class's.
    ///
    /// Nothing is retained: the receiver is only read, the way a `FieldGet`
    /// reads its own.
    ///
    /// **Two entry points, picked by the subject's representation.** A proven
    /// [`Ty::Object`] passes its bare pointer and pays nothing new. A
    /// [`Ty::Tagged`] — a `mixed`, or a `?Box` no test narrowed, which is the
    /// shape `instanceof` exists to interrogate — goes through
    /// [`Self::materialize_receiver`] and `mwl_value_instanceof`, which reads
    /// the tag and answers `false` for anything that is not an object. Two
    /// stores, on the only path that needs them; the branch is here rather
    /// than in the runtime because the proven case is the common one and it
    /// already had a pointer in hand.
    fn emit_instanceof(&mut self, value: ValueId, class: &str) -> Result<Value, CodegenError> {
        let desc = self.classes.desc(class).ok_or_else(|| {
            CodegenError::Unsupported(format!(
                "`instanceof {class}`, whose class this unit declares no descriptor for"
            ))
        })?;
        let address = i64::try_from(desc.addr())
            .map_err(|_| internal("a class descriptor above i64::MAX"))?;
        let desc = self.b.ins().iconst(types::I64, address);
        let (bare, subject_ty) = self.value(value)?;
        let (symbol, subject) = if matches!(subject_ty, Ty::Tagged) {
            ("mwl_value_instanceof", self.materialize_receiver(value)?)
        } else {
            ("mwl_object_instanceof", bare)
        };
        let callee = self.runtime_ref(symbol, RuntimeSig::InstanceOf)?;
        let call = self.b.ins().call(callee, &[subject, desc]);
        Ok(self.b.inst_results(call)[0])
    }

    /// `$obj->prop`: one load out of the receiver's field slot.
    ///
    /// The slot's *offset* comes from [`mwl_runtime::field_offset`], so this
    /// crate never restates the object layout — and the tag is not re-read,
    /// because the field's static type is already settled (see
    /// [`Self::load_value`]'s own note, which this shares).
    ///
    /// Nothing is retained here. `mwl_ir::ir::InstKind::FieldGet` reads the
    /// field without taking ownership, and `mwl_ir::lower::is_aliasing_read`
    /// makes the consumer insert the retain if it keeps the value.
    fn emit_field_get(
        &mut self,
        inst: &Inst,
        object: ValueId,
        class: &str,
        field: &str,
    ) -> Result<Value, CodegenError> {
        let offset = self.field_offset(class, field)?;
        let (base, _) = self.value(object)?;
        let ty = inst
            .ty
            .ok_or_else(|| internal("a property read with no representation"))?;
        self.load_value(base, offset, ty)
    }

    /// `$issue->path`: one call to `mwl_runtime::mwl_object_slot_get`, which
    /// finds the slot by **name** on the receiver's own descriptor.
    ///
    /// Not the inline load [`Self::emit_field_get`] emits, and deliberately:
    /// an ADR 0036 § 4 shape value has no class label to resolve a layout
    /// under, and the receiver's static shape may be a *widened* view of a
    /// value that lays its slots out differently — see
    /// `mwl_ir::ir::InstKind::SlotGet`, which owns the whole decision. The
    /// field name goes in this unit's data section rather than through
    /// `mwl_str_new`, so the read costs a call and no allocation, and the IR's
    /// slot index rides along as the hint that keeps the runtime's lookup one
    /// comparison on the common case.
    ///
    /// The borrow is [`Self::emit_field_get`]'s unchanged: the runtime copies
    /// the slot's value into `out` without retaining, so the consumer inserts
    /// the retain if it keeps it.
    fn emit_slot_get(
        &mut self,
        cur: Block,
        inst: &Inst,
        object: ValueId,
        field: &str,
        slot: u32,
    ) -> Result<Block, CodegenError> {
        let (name, len) = self.emit_bytes(field.as_bytes())?;
        let hint = self.b.ins().iconst(types::I64, i64::from(slot));
        let recv_p = self.materialize_receiver(object)?;

        let out_slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let out_p = self.b.ins().stack_addr(types::I64, out_slot, 0);

        let callee = self.runtime_ref("mwl_object_slot_get", RuntimeSig::SlotGet)?;
        let call = self
            .b
            .ins()
            .call(callee, &[self.ctx_p, recv_p, name, len, hint, out_p]);
        let status = self.b.inst_results(call)[0];
        let cont = self.emit_status_check(status, inst.on_error)?;

        let ty = inst
            .ty
            .ok_or_else(|| internal("a shape-field read with no representation"))?;
        let value = self.load_value(out_p, 0, ty)?;
        self.define(inst, value)?;
        let _ = cur;
        Ok(cont)
    }

    /// `$issue->path = "x";`: one call to `mwl_runtime::mwl_object_slot_set`,
    /// [`Self::emit_slot_get`]'s write half and a call for the same reason —
    /// a shape value has no class label to resolve a layout under, and the
    /// receiver's static shape may be a widened view of a value that lays its
    /// slots out differently.
    ///
    /// The value is materialized into a caller-owned 16-byte stack slot and
    /// passed by address, exactly as the read's result travels back: the
    /// runtime has to see the *tag*, both to check it against what the
    /// concrete class declares the field to hold and to release what the slot
    /// held, and neither is a fact the static representation here carries. A
    /// second slot takes the helper ABI's ignored result.
    ///
    /// Defines nothing, and inserts no retain or release: the runtime retains
    /// what it stores and releases what it displaced — see
    /// `mwl_ir::ir::InstKind::SlotSet`, which owns why this one field write
    /// borrows where [`Self::emit_field_set`] transfers.
    fn emit_slot_set(
        &mut self,
        inst: &Inst,
        object: ValueId,
        field: &str,
        slot: u32,
        value: ValueId,
    ) -> Result<Block, CodegenError> {
        let (name, len) = self.emit_bytes(field.as_bytes())?;
        let hint = self.b.ins().iconst(types::I64, i64::from(slot));
        let recv_p = self.materialize_receiver(object)?;
        let (value, value_ty) = self.value(value)?;

        let in_slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let in_p = self.b.ins().stack_addr(types::I64, in_slot, 0);
        self.store_value(in_p, 0, value, value_ty)?;

        let out_slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let out_p = self.b.ins().stack_addr(types::I64, out_slot, 0);

        let callee = self.runtime_ref("mwl_object_slot_set", RuntimeSig::SlotSet)?;
        let call = self
            .b
            .ins()
            .call(callee, &[self.ctx_p, recv_p, name, len, hint, in_p, out_p]);
        let status = self.b.inst_results(call)[0];
        self.emit_status_check(status, inst.on_error)
    }

    /// The receiver of an ADR 0036 § 4 name-keyed access, in the one shape
    /// both halves of it take: a caller-owned 16-byte
    /// [`mwl_runtime::Value`] passed by address.
    ///
    /// Not the bare pointer a [`Self::emit_field_get`] receiver is, and the
    /// difference is the whole erased half of § 4: this access may reach a
    /// `mixed`, whose tag no pass before it proved (`ReceiverProof::Erased`
    /// in `mwl_ir::lower::expr`). The runtime therefore has to *see* the tag —
    /// checking it where it already checks the name — so a `mixed` holding an
    /// `int` throws instead of being dereferenced as a pointer.
    ///
    /// A receiver that is statically a [`Ty::Object`] pays two stores for
    /// that uniformity, on a path that was already one call.
    fn materialize_receiver(&mut self, object: ValueId) -> Result<Value, CodegenError> {
        let (base, base_ty) = self.value(object)?;
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let recv_p = self.b.ins().stack_addr(types::I64, slot, 0);
        self.store_value(recv_p, 0, base, base_ty)?;
        Ok(recv_p)
    }

    /// `$obj->prop = expr;`: one store into the receiver's field slot.
    ///
    /// A plain store, with no release of what the slot held:
    /// `mwl_ir::lower::lower_reassignment` already emitted the `FieldGet` and
    /// `Release` pair for the previous value, ahead of this instruction. That
    /// split is why this is not `mwl_runtime::mwl_object_field_set`, which
    /// releases for its caller.
    fn emit_field_set(
        &mut self,
        object: ValueId,
        class: &str,
        field: &str,
        value: ValueId,
    ) -> Result<(), CodegenError> {
        let offset = self.field_offset(class, field)?;
        let (base, _) = self.value(object)?;
        let (value, ty) = self.value(value)?;
        self.store_value(base, offset, value, ty)
    }

    /// `Class::$prop`: two loads — the request's static-slot base out of the
    /// context, then the payload out of the slot.
    ///
    /// The same shape [`Self::emit_field_get`] has once its receiver is in
    /// hand, and for the same reason: the slot's static type is settled, so
    /// the tag is not re-read. What replaces the receiver is one load at
    /// [`mwl_runtime::STATICS_OFFSET`] — a hot-word read exactly like the
    /// safepoint poll's, against a pointer `mwl_runtime::Ctx::install_statics`
    /// armed before any of this unit's code ran.
    ///
    /// Nothing is retained: the slot keeps its one reference and
    /// `mwl_ir::ir::InstKind::StaticGet` borrows, so the consumer inserts the
    /// retain if it keeps the value.
    fn emit_static_get(
        &mut self,
        inst: &Inst,
        class: &str,
        name: &str,
    ) -> Result<Value, CodegenError> {
        let offset = self.static_offset(class, name)?;
        let base = self.statics_base();
        let ty = inst
            .ty
            .ok_or_else(|| internal("a static property read with no representation"))?;
        self.load_value(base, offset, ty)
    }

    /// `Class::$prop = v`: [`Self::emit_static_get`]'s store side, and
    /// [`Self::emit_field_set`]'s policy unchanged — the release of what the
    /// slot held is the IR's, emitted before this.
    fn emit_static_set(
        &mut self,
        class: &str,
        name: &str,
        value: ValueId,
    ) -> Result<(), CodegenError> {
        let offset = self.static_offset(class, name)?;
        let base = self.statics_base();
        let (value, ty) = self.value(value)?;
        self.store_value(base, offset, value, ty)
    }

    /// This request's static-slot base, loaded out of the context.
    ///
    /// Re-loaded per access rather than hoisted to function entry: nothing in
    /// a frame can re-arm the vector — `install_statics` runs before the
    /// request's first frame — so the two are equivalent, and leaving the load
    /// where the access is keeps it out of the far more common function that
    /// touches no static at all.
    fn statics_base(&mut self) -> Value {
        let offset = i32::try_from(mwl_runtime::STATICS_OFFSET)
            .unwrap_or_else(|_| unreachable!("the statics base sits within the context's head"));
        self.b
            .ins()
            .load(types::I64, ctx_word(), self.ctx_p, offset)
    }

    /// The byte offset of `class::$name`'s slot within this request's static
    /// storage, as an `i32` Cranelift memory operand.
    fn static_offset(&self, class: &str, name: &str) -> Result<i32, CodegenError> {
        let slot = *self
            .statics
            .get(&(class.to_owned(), name.to_owned()))
            .ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "the static property `{class}::${name}`, which this unit declares no slot for"
                ))
            })?;
        i32::try_from(usize::try_from(slot).unwrap_or(usize::MAX) * size_of::<MwlValue>())
            .map_err(|_| internal("a static property sitting past a 2 GiB offset"))
    }

    /// The byte offset of `class::field` within an instance, as an `i32`
    /// Cranelift memory operand.
    fn field_offset(&self, class: &str, field: &str) -> Result<i32, CodegenError> {
        let slot = self.classes.slot(class, field).ok_or_else(|| {
            CodegenError::Unsupported(format!(
                "the property `{class}::{field}`, which this unit declares no slot for"
            ))
        })?;
        i32::try_from(mwl_runtime::field_offset(slot))
            .map_err(|_| internal("an object field sitting past a 2 GiB offset"))
    }

    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's call-site probe, emitted twice per call: once before and once
    /// after, `status` distinguishing them.
    ///
    /// Exactly [`Self::emit_stmt_probe`]'s shape — one load of the debug-flags
    /// word, one predicted-not-taken branch, an out-of-line call that never
    /// runs while every bit is off — because it is the same mechanism at a
    /// different site, and ADR 0018's whole argument is that both are one
    /// cost class rather than a second compiled tier.
    ///
    /// The word is re-read at the exit site rather than the entry site's load
    /// being reused: a request may turn tracing on or off *during* the call,
    /// and `mwl_probe_call_exit`'s own doc comment says why recording that
    /// honestly beats a balanced-looking reconstruction.
    fn emit_call_probe(
        &mut self,
        symbol: &'static str,
        sig: RuntimeSig,
        label: (Value, Value),
        status: Option<Value>,
    ) -> Result<(), CodegenError> {
        let offset = i32::try_from(DEBUG_FLAGS_OFFSET)
            .map_err(|_| internal("the debug-flags word sits past a 2 GiB offset"))?;
        let flags = self
            .b
            .ins()
            .load(types::I64, ctx_word(), self.ctx_p, offset);

        let slow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(flags, slow, &[], cont, &[]);

        self.b.switch_to_block(slow);
        let probe = self.runtime_ref(symbol, sig)?;
        let (address, len) = label;
        match status {
            None => {
                self.b.ins().call(probe, &[self.ctx_p, address, len]);
            }
            Some(status) => {
                self.b
                    .ins()
                    .call(probe, &[self.ctx_p, address, len, status]);
            }
        }
        self.b.ins().jump(cont, &[]);

        self.b.switch_to_block(cont);
        Ok(())
    }

    /// A [`codegen::ir::FuncRef`] for one of the unit's own functions, cached
    /// per emitted function the same way [`Self::runtime_ref`] caches an
    /// import.
    fn callee_ref(&mut self, target: &str) -> Result<codegen::ir::FuncRef, CodegenError> {
        if let Some(reference) = self.callee_refs.get(target) {
            return Ok(*reference);
        }
        let id = *self
            .functions
            .get(target)
            .ok_or_else(|| CodegenError::UnknownTarget {
                caller: self.f.name.clone(),
                target: target.to_owned(),
            })?;
        let reference = self.module.declare_func_in_func(id, self.b.func);
        self.callee_refs.insert(target.to_owned(), reference);
        Ok(reference)
    }

    /// `.` concatenation: one call, which allocates the joined buffer once
    /// however many pieces there are.
    ///
    /// **Which call is the piece count.** Two pieces stay on the two-argument
    /// `mwl_str_concat`, which is the common shape and reads both operands out
    /// of registers. Three or more go to `mwl_str_concat_n` through a
    /// [`Self::pointer_array_slot`] — the same stack-array shape a helper
    /// call's argument list already uses, over bare `StrHeader` pointers
    /// instead of 16-byte `Value`s. Both allocate exactly one buffer, which is
    /// the whole point of `mwl_ir::ir::InstKind::Concat` being n-ary.
    ///
    /// No status check and no `Value` materialization: like `mwl_str_new`,
    /// these are memory primitives over bare `StrHeader` pointers rather than
    /// ADR 0002 helpers, because they cannot fail — see `mwl-runtime`'s
    /// "primitives compiled code calls" section for that split. No piece is
    /// retained or released here; `mwl_ir::ir::InstKind::Concat`'s own doc
    /// comment owns that rule and `mwl-ir` emits the releases.
    fn emit_concat(&mut self, pieces: &[ValueId]) -> Result<Value, CodegenError> {
        let mut operands = Vec::with_capacity(pieces.len());
        for piece in pieces {
            let (value, ty) = self.value(*piece)?;
            if !matches!(ty, Ty::Str | Ty::Bytes) {
                return Err(internal(
                    "a concatenation operand that lowering left unconverted",
                ));
            }
            operands.push(value);
        }
        // Two operands is the common shape and keeps the two-argument call:
        // nothing is stored, nothing is addressed, and `mwl_str_concat` reads
        // its pieces straight out of registers.
        if let [l, r] = operands[..] {
            let callee = self.runtime_ref("mwl_str_concat", RuntimeSig::StrConcat)?;
            let call = self.b.ins().call(callee, &[l, r]);
            return Ok(self.b.inst_results(call)[0]);
        }
        if operands.len() < 2 {
            return Err(internal("a concatenation of fewer than two operands"));
        }
        let count = i32::try_from(operands.len())
            .map_err(|_| internal("a concatenation past i32 pieces"))?;
        let base = self.pointer_array_slot(count);
        for (index, operand) in operands.iter().enumerate() {
            let offset = i32::try_from(index)
                .map_err(|_| internal("a concatenation past i32 pieces"))?
                * POINTER_SIZE;
            self.b.ins().store(trusted(), *operand, base, offset);
        }
        let len = self.b.ins().iconst(types::I64, i64::from(count));
        let callee = self.runtime_ref("mwl_str_concat_n", RuntimeSig::StrConcat)?;
        let call = self.b.ins().call(callee, &[base, len]);
        Ok(self.b.inst_results(call)[0])
    }

    /// `.=` on a `string` local: one call to `mwl_str_append`, which writes
    /// into the target's own buffer whenever it is solely owned and has the
    /// room, and separates copy-on-write when it is not.
    ///
    /// The same non-helper memory primitive [`Self::emit_concat`] calls, and
    /// the same two-pointers-to-a-pointer signature, so it shares
    /// [`RuntimeSig::StrConcat`]. What it does *not* share is ownership: this
    /// call consumes the reference `target` arrived with and produces the one
    /// the result carries, which is `mwl_ir::ir::InstKind::StrAppend`'s
    /// protocol and why nothing is retained or released around it here either.
    fn emit_str_append(&mut self, target: ValueId, suffix: ValueId) -> Result<Value, CodegenError> {
        let (t, tty) = self.value(target)?;
        let (s, sty) = self.value(suffix)?;
        for ty in [tty, sty] {
            if !matches!(ty, Ty::Str | Ty::Bytes) {
                return Err(internal(
                    "a string-append operand that lowering left unconverted",
                ));
            }
        }
        let callee = self.runtime_ref("mwl_str_append", RuntimeSig::StrConcat)?;
        let call = self.b.ins().call(callee, &[t, s]);
        Ok(self.b.inst_results(call)[0])
    }

    /// A 16-byte stack slot and its address — the one shape a [`MwlValue`]
    /// crosses a runtime-primitive boundary in, since a struct that size is
    /// classified differently by the SysV and Windows x64 ABIs (see
    /// `mwl_runtime::array`'s own note).
    fn value_slot(&mut self) -> Value {
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        self.b.ins().stack_addr(types::I64, slot, 0)
    }

    /// A stack slot holding `count` consecutive raw pointers, and its address
    /// — [`Self::value_slot`] for a runtime primitive that takes a *list* of
    /// bare pointers rather than one [`mwl_runtime::Value`].
    ///
    /// Frame-scoped, like every Cranelift stack slot: an n-ary concatenation
    /// inside a loop allocates this once per call site and restages into it
    /// every iteration, so what it costs is `count` stores and nothing else.
    fn pointer_array_slot(&mut self, count: i32) -> Value {
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (count * POINTER_SIZE).cast_unsigned(),
            POINTER_ALIGN_SHIFT,
        ));
        self.b.ins().stack_addr(types::I64, slot, 0)
    }

    /// `[...]`: one allocation, then one write per entry.
    ///
    /// `mwl_ir::ir::InstKind::ArrayNew` carries each key as a decimal string
    /// computed at lowering time, so each one is emitted here exactly like a
    /// `ConstStr` — an immortal header in the data section whose reference
    /// transfers straight into the array, which is why no retain accompanies
    /// it. The transfer is what it always was: the array owns a reference and
    /// releases it when it is freed, and that release happens to be the no-op
    /// [`mwl_runtime::IMMORTAL_REFCOUNT`] describes. Each write yields the
    /// array the next one writes into, per that instruction's
    /// consume-one-reference-yield-one protocol; the pointer never actually
    /// changes here, because a literal under construction is solely owned, but
    /// threading it is what keeps this on the one protocol rather than beside
    /// it.
    fn emit_array_new(&mut self, entries: &[(String, ValueId)]) -> Result<Value, CodegenError> {
        let callee = self.runtime_ref("mwl_array_new", RuntimeSig::ArrayNew)?;
        let call = self.b.ins().call(callee, &[]);
        let mut array = self.b.inst_results(call)[0];

        for (key, value) in entries {
            let key = self.emit_immortal_str(key.as_bytes())?;

            let slot = self.value_slot();
            let (value, ty) = self.value(*value)?;
            self.store_value(slot, 0, value, ty)?;

            let set = self.runtime_ref("mwl_array_set", RuntimeSig::ArraySet)?;
            let call = self.b.ins().call(set, &[array, key, slot]);
            array = self.b.inst_results(call)[0];
        }
        Ok(array)
    }

    /// `$a[$k] = expr;` and `$a[] = expr;`: one call that consumes the array
    /// and yields the array that now holds the entry.
    ///
    /// `symbol` and `sig` come from the caller because an `ArraySet` picks
    /// between `mwl_array_set` and `mwl_array_set_index` off its key
    /// operand's representation, while an `ArrayAppend` has no key to pick
    /// with. The *read* side picks nothing here at all: an
    /// `mwl_ir::ir::InstKind::ArrayGet` can throw, so it goes through
    /// [`Self::emit_helper`] against one entry point that tells the two key
    /// representations apart by tag.
    ///
    /// No refcount operation of any kind. `mwl_ir::ir::InstKind::ArraySet`'s
    /// own doc comment owns that rule: the reference the primitive consumes
    /// and the one it yields are the holder's same one slot, and `mwl-ir`
    /// already emitted whatever retain the key and value needed.
    fn emit_array_write(
        &mut self,
        symbol: &'static str,
        sig: RuntimeSig,
        array: ValueId,
        key: Option<ValueId>,
        value: ValueId,
    ) -> Result<Value, CodegenError> {
        let (array, _) = self.value(array)?;
        let key = key.map(|k| self.value(k)).transpose()?.map(|(v, _)| v);
        let slot = self.value_slot();
        let (value, ty) = self.value(value)?;
        self.store_value(slot, 0, value, ty)?;

        let callee = self.runtime_ref(symbol, sig)?;
        let call = match key {
            Some(key) => self.b.ins().call(callee, &[array, key, slot]),
            None => self.b.ins().call(callee, &[array, slot]),
        };
        Ok(self.b.inst_results(call)[0])
    }

    /// `$a[] = expr;`: the one array write that can fail, so the one emitted
    /// as a status check rather than as a value.
    ///
    /// PHP 8.5 refuses an append whose next integer key is already live, and
    /// `mwl_runtime::mwl_array_append` matches that refusal — see its own doc
    /// comment, and `mwl_runtime::array`'s *the append is the one array write
    /// with a fault channel*, which owns the signature. The array it yields
    /// comes back through a caller-owned pointer-wide slot, and on the error
    /// edge there is nothing to re-point: the refusal leaves the pointer the
    /// frame already holds live and owned.
    fn emit_array_append(
        &mut self,
        inst: &Inst,
        array: ValueId,
        value: ValueId,
    ) -> Result<Block, CodegenError> {
        let (array, _) = self.value(array)?;
        let in_p = self.value_slot();
        let (value, ty) = self.value(value)?;
        self.store_value(in_p, 0, value, ty)?;

        let out_slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            u32::try_from(std::mem::size_of::<usize>()).unwrap_or(8),
            3,
        ));
        let out_p = self.b.ins().stack_addr(types::I64, out_slot, 0);

        let callee = self.runtime_ref("mwl_array_append", RuntimeSig::ArrayAppend)?;
        let call = self.b.ins().call(callee, &[self.ctx_p, array, in_p, out_p]);
        let status = self.b.inst_results(call)[0];
        let cont = self.emit_status_check(status, inst.on_error)?;

        let written = self.b.ins().load(types::I64, trusted(), out_p, 0);
        self.define(inst, written)?;
        Ok(cont)
    }

    /// `[...$a]`: the whole-array copy, and the second write emitted as a
    /// status check.
    ///
    /// [`Self::emit_array_append`] with an array pointer where that one builds
    /// a 16-byte value slot — the subject is borrowed, so nothing about it is
    /// stored or read back. Which entry of it is renumbered and which keeps
    /// its key is `mwl_runtime::mwl_array_spread`'s, not this crate's: the
    /// whole point of one instruction here is that no key crosses this
    /// boundary at all.
    fn emit_array_spread(
        &mut self,
        inst: &Inst,
        array: ValueId,
        subject: ValueId,
    ) -> Result<Block, CodegenError> {
        let (array, _) = self.value(array)?;
        let (subject, _) = self.value(subject)?;

        let out_slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            u32::try_from(std::mem::size_of::<usize>()).unwrap_or(8),
            3,
        ));
        let out_p = self.b.ins().stack_addr(types::I64, out_slot, 0);

        let callee = self.runtime_ref("mwl_array_spread", RuntimeSig::ArraySpread)?;
        let call = self
            .b
            .ins()
            .call(callee, &[self.ctx_p, array, subject, out_p]);
        let status = self.b.inst_results(call)[0];
        let cont = self.emit_status_check(status, inst.on_error)?;

        let written = self.b.ins().load(types::I64, trusted(), out_p, 0);
        self.define(inst, written)?;
        Ok(cont)
    }

    /// A retain or release of one refcounted value.
    ///
    /// [`Ty::Str`] and [`Ty::Bytes`] share the `StrHeader` representation, so
    /// they share the primitive.
    ///
    /// An object's or an array's release is where a whole graph can be freed
    /// at once — `mwl_runtime::release`'s own docs explain why that sweep is
    /// one iterative worklist shared by both, which is what keeps this a
    /// single call rather than a depth-bounded one.
    fn emit_refcount(&mut self, retain: bool, value: Value, ty: Ty) -> Result<(), CodegenError> {
        // A tagged value's payload may or may not be refcounted, and its own
        // tag is what says which — so the branch is the runtime's, out of
        // line, rather than this table's. `mwl_ir::Ty::Tagged` states what
        // that costs.
        if ty == Ty::Tagged {
            let symbol = if retain {
                "mwl_value_retain"
            } else {
                "mwl_value_release"
            };
            let (tag_word, bits) = self.split_tagged(value);
            let callee = self.runtime_ref(symbol, RuntimeSig::ValueRefcount)?;
            self.b.ins().call(callee, &[tag_word, bits]);
            return Ok(());
        }
        let symbol = match (ty, retain) {
            (Ty::Str | Ty::Bytes, true) => "mwl_str_retain",
            (Ty::Str | Ty::Bytes, false) => "mwl_str_release",
            (Ty::Object, true) => "mwl_object_retain",
            (Ty::Object, false) => "mwl_object_release",
            (Ty::Array, true) => "mwl_array_retain",
            (Ty::Array, false) => "mwl_array_release",
            (other, _) => {
                return Err(CodegenError::Unsupported(format!(
                    "a refcount operation on representation {other:?}"
                )));
            }
        };
        let callee = self.runtime_ref(symbol, RuntimeSig::Refcount)?;
        self.b.ins().call(callee, &[value]);
        Ok(())
    }

    /// ADR 0002's compare-and-branch: on a non-`OK` status, take the
    /// instruction's error edge; otherwise carry on in a fresh block.
    ///
    /// `on_error` is [`mwl_ir::ir::Inst::on_error`] — the landing block the IR
    /// built for this exact program point, carrying the frame's cleanup and
    /// the decision between propagating and entering a `catch`. The status
    /// travels there as that block's one parameter.
    ///
    /// `None` keeps the pre-error-edge shape: return the status onward from a
    /// bare fail block, releasing nothing. That is reached only by the
    /// instructions whose sole non-`OK` outcome is a `FATAL` — see
    /// `Inst::on_error`'s own doc comment, which states that consequence
    /// rather than hiding it.
    fn emit_status_check(
        &mut self,
        status: Value,
        on_error: Option<BlockId>,
    ) -> Result<Block, CodegenError> {
        let failed = self
            .b
            .ins()
            .icmp_imm_s(IntCC::NotEqual, status, i64::from(OK));
        let cont = self.b.create_block();
        match on_error {
            Some(landing) => {
                let target = self.block(landing)?;
                let args = [codegen::ir::BlockArg::Value(status)];
                self.b.ins().brif(failed, target, &args, cont, &[]);
            }
            None => {
                let fail = self.b.create_block();
                self.b.ins().brif(failed, fail, &[], cont, &[]);
                self.b.switch_to_block(fail);
                self.b.ins().return_(&[status]);
            }
        }
        self.b.switch_to_block(cont);
        Ok(cont)
    }

    // -- terminators --------------------------------------------------------

    fn emit_terminator(&mut self, cur: Block, block: &BasicBlock) -> Result<(), CodegenError> {
        match &block.term {
            Terminator::Return(None) => {
                self.store_tag_and_bits(self.out_p, 0, Tag::Null, None)?;
                let ok = self.b.ins().iconst(types::I32, i64::from(OK));
                self.b.ins().return_(&[ok]);
            }
            Terminator::Return(Some(id)) => {
                let (value, ty) = self.value(*id)?;
                self.store_value(self.out_p, 0, value, ty)?;
                let ok = self.b.ins().iconst(types::I32, i64::from(OK));
                self.b.ins().return_(&[ok]);
            }
            Terminator::Jump(target) => {
                let args = self.phi_args(block.id, *target)?;
                let target = self.block(*target)?;
                self.b.ins().jump(target, &args);
            }
            Terminator::Branch {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let (cond, _) = self.value(*cond)?;
                let then_args = self.phi_args(block.id, *then_block)?;
                let else_args = self.phi_args(block.id, *else_block)?;
                let then_target = self.block(*then_block)?;
                let else_target = self.block(*else_block)?;
                self.b
                    .ins()
                    .brif(cond, then_target, &then_args, else_target, &else_args);
            }
            // A compare chain, not a jump table. Correct for any case set —
            // the IR deliberately does not require a dense or sorted one —
            // and the arms are few in the one producer there is today (one
            // per `yield` in a generator, plus the entry and the exhausted
            // arm). A `br_table` over a dense case set is this module's
            // known gap 6.
            Terminator::Switch {
                value,
                arms,
                default,
                default_edge: _,
            } => {
                let (value, _) = self.value(*value)?;
                for (case, target, _) in arms {
                    let hit = self.b.ins().icmp_imm_s(IntCC::Equal, value, *case);
                    let args = self.phi_args(block.id, *target)?;
                    let target = self.block(*target)?;
                    let next = self.b.create_block();
                    self.b.ins().brif(hit, target, &args, next, &[]);
                    self.b.switch_to_block(next);
                }
                let args = self.phi_args(block.id, *default)?;
                let target = self.block(*default)?;
                self.b.ins().jump(target, &args);
            }
            Terminator::Throw { value, landing } => {
                let (thrown, ty) = self.value(*value)?;
                if !matches!(ty, Ty::Object) {
                    return Err(internal("a `throw` of something that is not an object"));
                }
                // Ownership of the exception transfers to the context here —
                // `mwl_ir::lower` already retained an aliasing operand.
                let callee = self.runtime_ref("mwl_raise", RuntimeSig::Raise)?;
                self.b.ins().call(callee, &[self.ctx_p, thrown]);
                let status = self.b.ins().iconst(types::I32, i64::from(THROWN));
                let target = self.block(*landing)?;
                self.b
                    .ins()
                    .jump(target, &[codegen::ir::BlockArg::Value(status)]);
            }
            Terminator::Propagate { frame } => {
                let status = self.landing_status()?;
                let (address, len) = self.emit_bytes(frame.as_bytes())?;
                let callee = self.runtime_ref("mwl_trace_push", RuntimeSig::ProbeCallExit)?;
                self.b
                    .ins()
                    .call(callee, &[self.ctx_p, address, len, status]);
                self.b.ins().return_(&[status]);
            }
            Terminator::Catch { handler } => {
                let status = self.landing_status()?;
                // Only a `THROWN` is catchable: ADR 0020 keeps a `FATAL` out
                // of every `catch`, at the type level in the language and by
                // this comparison in the generated code.
                let caught = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, status, i64::from(THROWN));
                let args = self.phi_args(block.id, *handler)?;
                let target = self.block(*handler)?;
                let onward = self.b.create_block();
                self.b.ins().brif(caught, target, &args, onward, &[]);
                self.b.switch_to_block(onward);
                self.b.ins().return_(&[status]);
            }
            other => {
                return Err(CodegenError::Unsupported(format!(
                    "the terminator {other:?}"
                )));
            }
        }
        let _ = cur;
        Ok(())
    }

    /// The status parameter of the landing block being emitted.
    fn landing_status(&self) -> Result<Value, CodegenError> {
        self.landing_status
            .ok_or_else(|| internal("a landing terminator outside a landing block"))
    }

    /// The values `pred`'s jump into `succ` supplies for `succ`'s phis, in
    /// block-parameter order.
    ///
    /// A phi's `incoming` list is keyed by the *IR* block a value arrived
    /// from, which is the block whose terminator this is — not the Cranelift
    /// block that terminator physically ends up in, since a status check or a
    /// flag probe may have split it.
    fn phi_args(
        &self,
        pred: BlockId,
        succ: BlockId,
    ) -> Result<Vec<codegen::ir::BlockArg>, CodegenError> {
        let count = *self
            .phi_counts
            .get(&succ.index())
            .ok_or_else(|| internal("a jump to a block with no recorded phi count"))?;
        let block = self
            .f
            .blocks
            .iter()
            .find(|b| b.id.index() == succ.index())
            .ok_or_else(|| internal("a jump to a block that does not exist"))?;

        let mut args = Vec::with_capacity(count);
        for inst in &block.insts[..count] {
            let InstKind::Phi { incoming } = &inst.kind else {
                return Err(internal("a non-phi in a block's leading phi run"));
            };
            let (_, id) = incoming
                .iter()
                .find(|(from, _)| from.index() == pred.index())
                .ok_or_else(|| internal("a phi with no incoming value for a real predecessor"))?;
            let (value, _) = self.value(*id)?;
            args.push(codegen::ir::BlockArg::Value(value));
        }
        Ok(args)
    }

    // -- the 16-byte Value boundary -----------------------------------------

    /// Materializes a native value into a [`mwl_runtime::Value`] at
    /// `base + offset`.
    fn store_value(
        &mut self,
        base: Value,
        offset: i32,
        value: Value,
        ty: Ty,
    ) -> Result<(), CodegenError> {
        // A tagged value already *is* the two halves of a `Value`, so it is
        // written as those two words rather than through `tag_of` — see
        // `crate::ty::clif_ty`. Storing the whole low word (not just the tag
        // byte) is what keeps the seven padding bytes zero, which is the
        // struct's own `repr(C)` shape.
        //
        // A `decimal` takes the same path because it is the same sixteen
        // bytes, and *must*: those seven bytes are not padding for one, they
        // carry its scale, its sign and a third of its mantissa
        // (`mwl_runtime::decimal`).
        if matches!(ty, Ty::Tagged | Ty::Decimal) {
            let (tag_word, bits) = self.split_tagged(value);
            let tag_offset = offset
                + i32::try_from(MwlValue::TAG_OFFSET).map_err(|_| internal("a tag past i32"))?;
            let bits_offset = offset
                + i32::try_from(MwlValue::BITS_OFFSET)
                    .map_err(|_| internal("a payload past i32"))?;
            self.b.ins().store(trusted(), tag_word, base, tag_offset);
            self.b.ins().store(trusted(), bits, base, bits_offset);
            return Ok(());
        }
        let tag = tag_of(ty)?;
        let bits = match ty {
            // The payload is a `u64`; a `bool` occupies one byte of it, so the
            // other seven are zeroed rather than left as whatever the register
            // happened to hold.
            Ty::Bool => Some(self.b.ins().uextend(types::I64, value)),
            // Stored at its own type: the payload slot is eight bytes either
            // way, so a float needs no bitcast to reach it.
            Ty::Float => Some(value),
            _ => Some(value),
        };
        self.store_tag_and_bits(base, offset, tag, bits)
    }

    /// A [`Ty::Tagged`] value's two halves: the tag word and the payload.
    ///
    /// The one place `isplit` is written, so the "low half is the tag word"
    /// convention [`crate::ty::clif_ty`] states has a single reader.
    fn split_tagged(&mut self, value: Value) -> (Value, Value) {
        let (tag_word, bits) = self.b.ins().isplit(value);
        (tag_word, bits)
    }

    /// The inverse of [`Self::split_tagged`]: one [`Ty::Tagged`] register pair
    /// from a tag word and a payload.
    ///
    /// `tag_word` must be zero-extended from the tag byte — see
    /// [`crate::ty::clif_ty`] for why every producer holds that.
    fn join_tagged(&mut self, tag_word: Value, bits: Value) -> Value {
        self.b.ins().iconcat(tag_word, bits)
    }

    fn store_tag_and_bits(
        &mut self,
        base: Value,
        offset: i32,
        tag: Tag,
        bits: Option<Value>,
    ) -> Result<(), CodegenError> {
        let tag_offset =
            offset + i32::try_from(MwlValue::TAG_OFFSET).map_err(|_| internal("a tag past i32"))?;
        let bits_offset = offset
            + i32::try_from(MwlValue::BITS_OFFSET).map_err(|_| internal("a payload past i32"))?;

        // The whole low **word**, not just the tag byte: the seven bytes
        // beside it are a `decimal`'s scale, sign and mantissa-low
        // (`mwl_runtime::decimal`), so `Self::load_value` has to be able to
        // read them back for a tagged slot that turns out to hold one. Writing
        // the word here is what makes them zero for every other tag, which is
        // the invariant that read depends on — and it costs one `I64` store
        // where an `I8` store stood.
        let tag_value = self.b.ins().iconst(types::I64, i64::from(tag as u8));
        self.b.ins().store(trusted(), tag_value, base, tag_offset);
        let bits = match bits {
            Some(bits) => bits,
            None => self.b.ins().iconst(types::I64, 0),
        };
        self.b.ins().store(trusted(), bits, base, bits_offset);
        Ok(())
    }

    /// Reads a native value back out of a [`mwl_runtime::Value`] at
    /// `base + offset`.
    ///
    /// The tag is *not* re-checked: every helper already validates its own
    /// arguments' tags and returns `FATAL` on a mismatch — see
    /// `mwl_runtime::helpers`' own docs for why that check lives there — and a
    /// well-typed program cannot produce one here by construction.
    fn load_value(&mut self, base: Value, offset: i32, ty: Ty) -> Result<Value, CodegenError> {
        let bits_offset = offset
            + i32::try_from(MwlValue::BITS_OFFSET).map_err(|_| internal("a payload past i32"))?;
        Ok(match ty {
            Ty::Bool => {
                let wide = self.b.ins().load(types::I64, trusted(), base, bits_offset);
                self.b.ins().ireduce(types::I8, wide)
            }
            Ty::Float => self.b.ins().load(types::F64, trusted(), base, bits_offset),
            Ty::Void => return Err(internal("reading a value of representation `void`")),
            // Both halves, as the register pair `crate::ty::clif_ty` describes.
            // The tag is read as one **byte** and zero-extended rather than as
            // the whole low word: the seven padding bytes beside it are zero in
            // every `Value` this runtime writes, but reading them would make
            // that a thing to trust rather than a thing that cannot matter.
            // Both halves, as the register pair `crate::ty::clif_ty`
            // describes. The low half is read as a whole **word** rather than
            // as the tag byte alone: a tagged slot may hold a `decimal`, whose
            // scale, sign and mantissa-low live in the seven bytes beside the
            // tag (`mwl_runtime::decimal`). Every producer writes that word in
            // full — `Self::store_tag_and_bits` here, `Value`'s own `repr(C)`
            // in the runtime — so those bytes are zero for every other tag and
            // reading them costs nothing.
            Ty::Tagged | Ty::Decimal => {
                let tag_offset = offset
                    + i32::try_from(MwlValue::TAG_OFFSET)
                        .map_err(|_| internal("a tag past i32"))?;
                let low = self.b.ins().load(types::I64, trusted(), base, tag_offset);
                let high = self.b.ins().load(types::I64, trusted(), base, bits_offset);
                self.join_tagged(low, high)
            }
            _ => self.b.ins().load(types::I64, trusted(), base, bits_offset),
        })
    }

    // -- imports ------------------------------------------------------------

    fn runtime_ref(
        &mut self,
        symbol: &'static str,
        sig: RuntimeSig,
    ) -> Result<codegen::ir::FuncRef, CodegenError> {
        if let Some(reference) = self.frefs.get(symbol) {
            return Ok(*reference);
        }
        let signature = match sig {
            RuntimeSig::Helper => &self.sigs.helper,
            RuntimeSig::HelperVariadic => &self.sigs.helper_variadic,
            RuntimeSig::Safepoint => &self.sigs.safepoint,
            RuntimeSig::StackCheck => &self.sigs.stack_check,
            RuntimeSig::Probe => &self.sigs.probe,
            RuntimeSig::ProbeCall => &self.sigs.probe_call,
            RuntimeSig::ProbeCallExit => &self.sigs.probe_call_exit,
            RuntimeSig::StrConcat => &self.sigs.str_concat,
            RuntimeSig::PtrEq => &self.sigs.ptr_eq,
            RuntimeSig::FloatPow => &self.sigs.float_pow,
            RuntimeSig::Refcount => &self.sigs.refcount,
            RuntimeSig::ValueRefcount => &self.sigs.value_refcount,
            RuntimeSig::PtrToPtr => &self.sigs.ptr_to_ptr,
            RuntimeSig::Raise => &self.sigs.raise,
            RuntimeSig::RaiseNew => &self.sigs.raise_new,
            RuntimeSig::InstanceOf => &self.sigs.instanceof,
            RuntimeSig::ClassMethod => &self.sigs.class_method,
            RuntimeSig::SlotGet => &self.sigs.slot_get,
            RuntimeSig::SlotSet => &self.sigs.slot_set,
            RuntimeSig::ArrayNew => &self.sigs.array_new,
            RuntimeSig::ArraySet => &self.sigs.array_set,
            RuntimeSig::ArraySetIndex => &self.sigs.array_set_index,
            RuntimeSig::ArrayAppend => &self.sigs.array_append,
            RuntimeSig::ArraySpread => &self.sigs.array_spread,
            RuntimeSig::ArrayUnset => &self.sigs.array_unset,
            RuntimeSig::ArrayNextSlot => &self.sigs.array_next_slot,
            RuntimeSig::ArrayKeyAt => &self.sigs.array_key_at,
            RuntimeSig::ArrayValueAt => &self.sigs.array_value_at,
        };
        let id = self
            .module
            .declare_function(symbol, Linkage::Import, signature)
            .map_err(|source| CodegenError::Cranelift {
                function: self.f.name.clone(),
                source: Box::new(source),
            })?;
        let reference = self.module.declare_func_in_func(id, self.b.func);
        self.frefs.insert(symbol, reference);
        Ok(reference)
    }
}

/// Which form a call's target takes: a `FuncRef` Cranelift relocates, or a
/// code address computed at run time.
///
/// The two differ in exactly one instruction. Everything ADR 0002 asks of a
/// call site — the argument slots, both probes, the status check, the landing
/// block — is identical, which is why `Self::emit_invoke_at` takes this rather
/// than there being a second call path.
#[derive(Clone, Copy)]
enum Callee {
    Direct(codegen::ir::FuncRef),
    Indirect(Value),
}

/// Which of [`Signatures`]' shapes a runtime symbol has.
#[derive(Clone, Copy)]
enum RuntimeSig {
    Helper,
    HelperVariadic,
    Safepoint,
    StackCheck,
    Probe,
    ProbeCall,
    ProbeCallExit,
    StrConcat,
    PtrEq,
    FloatPow,
    Refcount,
    ValueRefcount,
    PtrToPtr,
    Raise,
    RaiseNew,
    InstanceOf,
    ClassMethod,
    SlotGet,
    SlotSet,
    ArrayNew,
    ArraySet,
    ArraySetIndex,
    ArrayAppend,
    ArraySpread,
    ArrayUnset,
    ArrayNextSlot,
    ArrayKeyAt,
    ArrayValueAt,
}

/// The symbol name `mwl-runtime` exports for one [`Helper`] tag.
///
/// This mapping is why `mwl-codegen` exists as the crate that depends on both:
/// `mwl-runtime` deliberately does not know `mwl_ir::Helper`, and `mwl-ir`
/// deliberately does not know a symbol name.
fn helper_symbol(helper: Helper) -> Result<&'static str, CodegenError> {
    Ok(match helper {
        Helper::IntToString => "mwl_int_to_string",
        Helper::UintToString => "mwl_uint_to_string",
        Helper::FloatToString => "mwl_float_to_string",
        Helper::BoolToString => "mwl_bool_to_string",
        Helper::IntTruthy => "mwl_int_truthy",
        Helper::UintTruthy => "mwl_uint_truthy",
        Helper::FloatTruthy => "mwl_float_truthy",
        Helper::StrTruthy => "mwl_str_truthy",
        Helper::EchoStr => "mwl_echo_str",
        Helper::Exit => "mwl_exit",
        Helper::LiteralMismatch => "mwl_literal_mismatch",
        Helper::Identical => "mwl_value_identical",
        Helper::NumericEq => "mwl_numeric_eq",
        Helper::NumericLt => "mwl_numeric_lt",
        Helper::NumericCmp => "mwl_numeric_cmp",
        Helper::NumericLtEq => "mwl_numeric_lt_eq",
        Helper::SecretEq => "mwl_secret_eq",
        Helper::CallClosure => "mwl_call_closure",
        Helper::BytesTruthy => "mwl_bytes_truthy",
        Helper::ArrayTruthy => "mwl_array_truthy",
        Helper::ValueTruthy => "mwl_value_truthy",
        Helper::ArrayRowForWrite => "mwl_array_row_for_write",
        Helper::IntToUint => "mwl_int_to_uint",
        Helper::UintToInt => "mwl_uint_to_int",
        Helper::IntToFloat => "mwl_int_to_float",
        Helper::UintToFloat => "mwl_uint_to_float",
        Helper::FloatToInt => "mwl_float_to_int",
        Helper::FloatToUint => "mwl_float_to_uint",
        Helper::StrToInt => "mwl_str_to_int",
        Helper::StrToUint => "mwl_str_to_uint",
        Helper::StrToFloat => "mwl_str_to_float",
        Helper::BytesToString => "mwl_bytes_to_string",
        Helper::ToIntOrNull => "mwl_to_int_or_null",
        Helper::ToUintOrNull => "mwl_to_uint_or_null",
        Helper::ToFloatOrNull => "mwl_to_float_or_null",
        Helper::ToStringOrNull => "mwl_to_string_or_null",
        Helper::ToBytesOrNull => "mwl_to_bytes_or_null",
        Helper::TaggedToString => "mwl_tagged_to_string",
        Helper::TaggedToInt => "mwl_tagged_to_int",
        Helper::TaggedToUint => "mwl_tagged_to_uint",
        Helper::TaggedToFloat => "mwl_tagged_to_float",
        Helper::TaggedToBytes => "mwl_tagged_to_bytes",
        Helper::DecimalTruthy => "mwl_decimal_truthy",
        Helper::DecimalAdd => "mwl_decimal_add",
        Helper::DecimalSub => "mwl_decimal_sub",
        Helper::DecimalMul => "mwl_decimal_mul",
        Helper::DecimalDiv => "mwl_decimal_div",
        Helper::DecimalMod => "mwl_decimal_mod",
        Helper::DecimalNeg => "mwl_decimal_neg",
        Helper::DecimalEq => "mwl_decimal_eq",
        Helper::DecimalLt => "mwl_decimal_lt",
        Helper::DecimalLtEq => "mwl_decimal_lt_eq",
        Helper::DecimalCmp => "mwl_decimal_cmp",
        Helper::ToDecimal => "mwl_to_decimal",
        Helper::ToDecimalOrNull => "mwl_to_decimal_or_null",
        Helper::DecimalToInt => "mwl_decimal_to_int",
        Helper::DecimalToUint => "mwl_decimal_to_uint",
        Helper::DecimalToFloat => "mwl_decimal_to_float",
        Helper::DecimalToString => "mwl_decimal_to_string",
        other => {
            return Err(CodegenError::Unsupported(format!(
                "the runtime helper {other:?}"
            )));
        }
    })
}

/// A human-readable name for an instruction this slice does not lower.
fn describe(kind: &InstKind) -> String {
    let what = match kind {
        InstKind::Call { .. } => "a resolved method call",
        InstKind::New { .. } => "`new`",
        InstKind::FieldGet { .. } => "a property read",
        InstKind::FieldSet { .. } => "a property write",
        InstKind::InstanceOf { .. } => "`instanceof`",
        InstKind::Concat { .. } => "`.` string concatenation",
        _ => "this instruction",
    };
    what.to_owned()
}
