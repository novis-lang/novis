//! A readable text form of the IR, used by [`crate::lower`]'s snapshot tests
//! today and intended for a future `mwl ir --dump`-style CLI flag once one
//! exists (M3, alongside `mwl run --dump-asm` — see
//! `docs/implementation-plan.md`'s M3 paragraph).
//!
//! Not a serialization format anything round-trips through — there is no
//! parser for this text, on purpose, the same way `--dump-asm` output isn't
//! meant to be read back in.

use std::fmt::Write as _;

use mwl_diagnostics::{SourceFile, Span};

use crate::ids::BlockId;
use crate::ir::{
    BasicBlock, BinOp, Function, Helper, Inst, InstKind, Program, Terminator, ThrowableOp, UnOp,
};
use crate::ty::Ty;

/// Renders every function in `program`, in order, as text.
#[must_use]
pub fn print_program(program: &Program, src: &SourceFile) -> String {
    let mut out = String::new();
    for (i, f) in program.functions.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        print_function_into(&mut out, f, src);
    }
    out
}

/// Renders one function as text — see [`print_program`].
#[must_use]
pub fn print_function(f: &Function, src: &SourceFile) -> String {
    let mut out = String::new();
    print_function_into(&mut out, f, src);
    out
}

fn print_function_into(out: &mut String, f: &Function, src: &SourceFile) {
    let params = f
        .params
        .iter()
        .map(|t| ty_name(*t))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "fn {}({params}) -> {} {{", f.name, ty_name(f.ret));
    for block in &f.blocks {
        print_block(out, block, f, src);
    }
    out.push_str("}\n");
}

fn print_block(out: &mut String, block: &BasicBlock, f: &Function, src: &SourceFile) {
    let _ = writeln!(out, "  bb{}:", block.id.index());
    for inst in &block.insts {
        print_inst(out, inst, f, src);
    }
    print_term(out, &block.term);
}

fn print_inst(out: &mut String, inst: &Inst, f: &Function, src: &SourceFile) {
    if let InstKind::StmtMarker(id) = inst.kind {
        let span = f.stmt_spans[id.index() as usize];
        let _ = writeln!(out, "    ; stmt s{} @{}", id.index(), fmt_span(span, src));
        return;
    }
    if let InstKind::Safepoint = inst.kind {
        let _ = writeln!(out, "    safepoint");
        return;
    }
    if let InstKind::Retain { operand } = inst.kind {
        let _ = writeln!(out, "    retain v{}", operand.index());
        return;
    }
    if let InstKind::Release { operand } = inst.kind {
        let _ = writeln!(out, "    release v{}", operand.index());
        return;
    }
    if let InstKind::FieldSet {
        object,
        ref class,
        ref field,
        value,
    } = inst.kind
    {
        let _ = writeln!(
            out,
            "    field.set v{}, {class}::{field}, v{}",
            object.index(),
            value.index()
        );
        return;
    }
    // The one value-less `HelperCall`: `Helper::EchoStr` is invoked for its
    // effect, so it has no `result` for the general arm below to print.
    if let InstKind::HelperCall {
        helper: Helper::EchoStr,
        ref args,
    } = inst.kind
    {
        let operands: Vec<String> = args.iter().map(|a| format!("v{}", a.index())).collect();
        let _ = writeln!(
            out,
            "    helper.echo_str {}{}",
            operands.join(", "),
            error_edge(inst)
        );
        return;
    }
    let v = inst
        .result
        .expect("every non-marker instruction defines a value");
    let ty = inst.ty.expect("every non-marker instruction has a type");
    let rhs = match &inst.kind {
        InstKind::ConstBool(b) => format!("const.bool {b}"),
        InstKind::ConstInt(n) => format!("const.int {n}"),
        InstKind::ConstUint(n) => format!("const.uint {n}"),
        InstKind::ConstFloat(n) => format!("const.float {n}"),
        InstKind::ConstStr(s) => format!("const.str {s:?}"),
        InstKind::Param(i) => format!("param {i}"),
        InstKind::BinOp { op, lhs, rhs } => {
            format!("{} v{}, v{}", bin_op_name(*op), lhs.index(), rhs.index())
        }
        InstKind::UnOp { op, operand } => format!("{} v{}", un_op_name(*op), operand.index()),
        InstKind::Phi { incoming } => {
            let parts: Vec<String> = incoming
                .iter()
                .map(|(b, v)| format!("{}: v{}", block_name(*b), v.index()))
                .collect();
            format!("phi [{}]", parts.join(", "))
        }
        InstKind::Call {
            target,
            receiver,
            args,
        } => {
            let mut parts: Vec<String> = receiver
                .iter()
                .map(|r| format!("this: v{}", r.index()))
                .collect();
            parts.extend(args.iter().map(|a| format!("v{}", a.index())));
            format!("call {target}({})", parts.join(", "))
        }
        InstKind::New { class, ctor, args } => {
            let parts: Vec<String> = args.iter().map(|a| format!("v{}", a.index())).collect();
            match ctor {
                Some(ctor) => format!("new {class} via {ctor}({})", parts.join(", ")),
                None => format!("new {class}({})", parts.join(", ")),
            }
        }
        InstKind::FieldGet {
            object,
            class,
            field,
        } => format!("field.get v{}, {class}::{field}", object.index()),
        InstKind::Concat { lhs, rhs } => format!("concat v{}, v{}", lhs.index(), rhs.index()),
        InstKind::HelperCall { helper, args } => {
            let parts: Vec<String> = args.iter().map(|a| format!("v{}", a.index())).collect();
            format!("helper.{} {}", helper_name(*helper), parts.join(", "))
        }
        InstKind::ArrayNew { entries } => {
            let parts: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{k:?}: v{}", v.index()))
                .collect();
            format!("array.new [{}]", parts.join(", "))
        }
        InstKind::ArrayGet { array, key } => {
            format!("array.get v{}, v{}", array.index(), key.index())
        }
        InstKind::ArraySet { array, key, value } => format!(
            "array.set v{}, v{}, v{}",
            array.index(),
            key.index(),
            value.index()
        ),
        InstKind::ArrayAppend { array, value } => {
            format!("array.append v{}, v{}", array.index(), value.index())
        }
        InstKind::ArrayUnset { array, key } => {
            format!("array.unset v{}, v{}", array.index(), key.index())
        }
        InstKind::ArrayNextSlot { array, from } => {
            format!("array.next_slot v{}, v{}", array.index(), from.index())
        }
        InstKind::ArrayKeyAt { array, slot } => {
            format!("array.key_at v{}, v{}", array.index(), slot.index())
        }
        InstKind::ArrayValueAt { array, slot } => {
            format!("array.value_at v{}, v{}", array.index(), slot.index())
        }
        InstKind::TakeThrown => "take.thrown".to_owned(),
        InstKind::Throwable { op, operand } => {
            format!("throwable.{} v{}", throwable_op_name(*op), operand.index())
        }
        InstKind::StmtMarker(_)
        | InstKind::Safepoint
        | InstKind::Retain { .. }
        | InstKind::Release { .. }
        | InstKind::FieldSet { .. } => {
            unreachable!("returned above")
        }
    };
    let _ = writeln!(
        out,
        "    v{} = {rhs}{}  ; {}",
        v.index(),
        error_edge(inst),
        ty_name(ty)
    );
}

/// ` ! bbN` for an instruction carrying [`Inst::on_error`], the empty string
/// for one that cannot fail — see that field's own doc comment.
fn error_edge(inst: &Inst) -> String {
    match inst.on_error {
        Some(landing) => format!(" ! {}", block_name(landing)),
        None => String::new(),
    }
}

fn print_term(out: &mut String, term: &Terminator) {
    match term {
        Terminator::Return(Some(v)) => {
            let _ = writeln!(out, "    return v{}", v.index());
        }
        Terminator::Return(None) => {
            let _ = writeln!(out, "    return");
        }
        Terminator::Jump(b) => {
            let _ = writeln!(out, "    jump {}", block_name(*b));
        }
        Terminator::Throw { value, landing } => {
            let _ = writeln!(
                out,
                "    throw v{} -> {}",
                value.index(),
                block_name(*landing)
            );
        }
        Terminator::Propagate { frame } => {
            let _ = writeln!(out, "    propagate {frame:?}");
        }
        Terminator::Catch { handler } => {
            let _ = writeln!(out, "    catch -> {}", block_name(*handler));
        }
        Terminator::Branch {
            cond,
            then_block,
            then_edge,
            else_block,
            else_edge,
        } => {
            let _ = writeln!(
                out,
                "    branch v{} -> {} (e{}), {} (e{})",
                cond.index(),
                block_name(*then_block),
                then_edge.index(),
                block_name(*else_block),
                else_edge.index()
            );
        }
    }
}

fn block_name(id: BlockId) -> String {
    format!("bb{}", id.index())
}

fn fmt_span(span: Span, src: &SourceFile) -> String {
    let (l1, c1) = src.line_col(span.start);
    let (l2, c2) = src.line_col(span.end);
    format!("{}:{}-{}:{}", l1 + 1, c1 + 1, l2 + 1, c2 + 1)
}

fn ty_name(ty: Ty) -> &'static str {
    match ty {
        Ty::Bool => "bool",
        Ty::Int => "int",
        Ty::Uint => "uint",
        Ty::Float => "float",
        Ty::Void => "void",
        Ty::Object => "object",
        Ty::Str => "string",
        Ty::Bytes => "bytes",
        Ty::Array => "array",
        Ty::Throwable => "throwable",
        Ty::Mixed => "mixed",
    }
}

fn bin_op_name(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        BinOp::Mul => "mul",
        BinOp::Div => "div",
        BinOp::Mod => "mod",
        BinOp::Eq => "eq",
        BinOp::NotEq => "ne",
        BinOp::Lt => "lt",
        BinOp::LtEq => "le",
        BinOp::Gt => "gt",
        BinOp::GtEq => "ge",
    }
}

fn un_op_name(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "neg",
        UnOp::Not => "not",
    }
}

fn throwable_op_name(op: ThrowableOp) -> &'static str {
    match op {
        ThrowableOp::New => "new",
        ThrowableOp::Message => "message",
        ThrowableOp::TraceAsString => "trace_as_string",
    }
}

fn helper_name(h: Helper) -> &'static str {
    match h {
        Helper::IntToString => "int_to_string",
        Helper::UintToString => "uint_to_string",
        Helper::FloatToString => "float_to_string",
        Helper::BoolToString => "bool_to_string",
        Helper::IntTruthy => "int_truthy",
        Helper::UintTruthy => "uint_truthy",
        Helper::FloatTruthy => "float_truthy",
        Helper::StrTruthy => "str_truthy",
        Helper::ArrayTruthy => "array_truthy",
        Helper::EchoStr => "echo_str",
    }
}
