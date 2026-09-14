//! A readable text form of the IR, used by [`crate::lower`]'s snapshot tests
//! today and intended for a future `nvs ir --dump`-style CLI flag once one
//! exists (M3, alongside `nvs run --dump-asm` — see
//! `docs/implementation-plan.md`'s M3 paragraph).
//!
//! Not a serialization format anything round-trips through — there is no
//! parser for this text, on purpose, the same way `--dump-asm` output isn't
//! meant to be read back in.

use std::fmt::Write as _;

use nvs_diagnostics::{SourceFile, Span};

use crate::ids::BlockId;
use crate::ir::{
    AbsentKey, BasicBlock, BinOp, Function, Helper, Inst, InstKind, Program, Terminator,
    TestedClass, UnOp,
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
    if let InstKind::RefStore { slot, value } = inst.kind {
        let _ = writeln!(out, "    ref.store v{}, v{}", slot.index(), value.index());
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
    if let InstKind::StaticSet {
        ref class,
        ref name,
        value,
    } = inst.kind
    {
        let _ = writeln!(out, "    static.set {class}::${name}, v{}", value.index());
        return;
    }
    // A value-less `HelperCall` is invoked purely for an effect:
    // `Helper::EchoStr` writes and returns nothing, `Helper::LiteralMismatch`
    // never returns at all. Such a call has no `result` for the general arm
    // below to print, so it prints here.
    if let InstKind::HelperCall { helper, ref args } = inst.kind
        && inst.result.is_none()
    {
        let operands: Vec<String> = args.iter().map(|a| format!("v{}", a.index())).collect();
        let _ = writeln!(
            out,
            "    helper.{} {}{}",
            helper_name(helper),
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
        InstKind::ConstDecimal {
            negative,
            mantissa,
            scale,
        } => format!(
            "const.decimal {}{mantissa}e-{scale}",
            if *negative { "-" } else { "" }
        ),
        InstKind::ConstNull => "const.null".to_owned(),
        InstKind::ConstUnset => "const.unset".to_owned(),
        InstKind::ConstStr(s) => format!("const.str {s:?}"),
        InstKind::ConstBytes(b) => format!("const.bytes {b:?}"),
        InstKind::ConstMarkup(s) => format!("const.markup {s:?}"),
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
        InstKind::ClassDescConst { class } => format!("class.desc {class}"),
        InstKind::ShapeCodecConst { shape } => match shape {
            Some(key) => format!("shape.codec {key}"),
            None => "shape.codec none".to_owned(),
        },
        InstKind::SourceConst { source } => match source {
            Some(source) => match &source.member {
                Some(member) => format!("source {}:{} {member}", source.file, source.line),
                None => format!("source {}:{}", source.file, source.line),
            },
            None => "source none".to_owned(),
        },
        InstKind::ClassDescOf { object } => format!("class.of v{}", object.index()),
        InstKind::ClassDescIn { subject, base } => {
            format!("class.in v{} {base}", subject.index())
        }
        InstKind::CallVirtual {
            lsb,
            method,
            fallback,
            receiver,
            args,
        } => {
            let mut parts: Vec<String> = receiver
                .iter()
                .map(|r| format!("this: v{}", r.index()))
                .collect();
            parts.extend(args.iter().map(|a| format!("v{}", a.index())));
            let fallback = fallback.as_deref().unwrap_or("<abstract>");
            format!(
                "call.virtual v{}::{method} else {fallback}({})",
                lsb.index(),
                parts.join(", ")
            )
        }
        InstKind::NewDynamic { desc, ctor, args } => {
            let parts: Vec<String> = args.iter().map(|a| format!("v{}", a.index())).collect();
            match ctor {
                Some(ctor) => format!(
                    "new.dynamic v{} via {ctor}({})",
                    desc.index(),
                    parts.join(", ")
                ),
                None => format!("new.dynamic v{}({})", desc.index(), parts.join(", ")),
            }
        }
        InstKind::FieldGet {
            object,
            class,
            field,
        } => format!("field.get v{}, {class}::{field}", object.index()),
        InstKind::StaticGet { class, name } => format!("static.get {class}::${name}"),
        InstKind::SlotGet {
            object,
            field,
            slot,
            absent,
        } => {
            // Only the guarded read is spelled out, exactly as `array.get`
            // does it below: an unqualified `slot.get` is the one written in
            // source, which throws.
            let suffix = match absent {
                AbsentKey::Throws => "",
                AbsentKey::Null => ".ornull",
            };
            format!("slot.get{suffix} v{}, {field} @{slot}", object.index())
        }
        InstKind::SlotSet {
            object,
            field,
            slot,
            value,
        } => format!(
            "slot.set v{}, {field} @{slot}, v{}",
            object.index(),
            value.index()
        ),
        InstKind::KeyGet { object, key } => {
            format!("key.get v{}, v{}", object.index(), key.index())
        }
        InstKind::KeySet { object, key, value } => format!(
            "key.set v{}, v{}, v{}",
            object.index(),
            key.index(),
            value.index()
        ),
        InstKind::InstanceOf { value, class } => match class {
            TestedClass::Named(class) => format!("instanceof v{}, {class}", value.index()),
            TestedClass::Descriptor(desc) => {
                format!("instanceof v{}, v{}", value.index(), desc.index())
            }
        },
        InstKind::Concat { pieces } => {
            let parts: Vec<String> = pieces.iter().map(|p| format!("v{}", p.index())).collect();
            format!("concat {}", parts.join(", "))
        }
        InstKind::StrAppend { target, suffix } => {
            format!("str.append v{}, v{}", target.index(), suffix.index())
        }
        InstKind::Reinterpret { operand } => format!("reinterpret v{}", operand.index()),
        InstKind::Tag { operand } => format!("tag v{}", operand.index()),
        InstKind::Untag { operand } => format!("untag v{}", operand.index()),
        InstKind::IsNull { operand } => format!("is.null v{}", operand.index()),
        InstKind::TagIs { operand, repr } => {
            format!("is.tag v{}, {repr:?}", operand.index())
        }
        InstKind::Clone { object } => format!("clone v{}", object.index()),
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
        InstKind::ArrayGet { array, key, absent } => {
            // Only the guarded read is spelled out: an unqualified `array.get`
            // is the one written in source, which throws.
            let suffix = match absent {
                AbsentKey::Throws => "",
                AbsentKey::Null => ".ornull",
            };
            format!("array.get{suffix} v{}, v{}", array.index(), key.index())
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
        InstKind::ArraySpread { array, subject } => {
            format!("array.spread v{}, v{}", array.index(), subject.index())
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
        InstKind::CoreCall { symbol, args } => {
            let parts: Vec<String> = args.iter().map(|a| format!("v{}", a.index())).collect();
            format!("core.call {symbol}({})", parts.join(", "))
        }
        InstKind::TakeThrown => "take.thrown".to_owned(),
        InstKind::RefSlot { init } => format!("ref.slot v{}", init.index()),
        InstKind::RefLoad { slot } => format!("ref.load v{}", slot.index()),
        InstKind::StmtMarker(_)
        | InstKind::Safepoint
        | InstKind::Retain { .. }
        | InstKind::Release { .. }
        | InstKind::RefStore { .. }
        | InstKind::FieldSet { .. }
        | InstKind::StaticSet { .. } => {
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
        Terminator::Throw {
            value,
            source,
            landing,
        } => {
            let at = source.map_or_else(String::new, |v| format!(" at v{}", v.index()));
            let _ = writeln!(
                out,
                "    throw v{}{at} -> {}",
                value.index(),
                block_name(*landing)
            );
        }
        Terminator::Propagate { frame } => {
            let _ = writeln!(out, "    propagate {frame:?}");
        }
        Terminator::Catch { handler, onward } => {
            let _ = writeln!(
                out,
                "    catch -> {} else {}",
                block_name(*handler),
                block_name(*onward)
            );
        }
        Terminator::Switch {
            value,
            arms,
            default,
            default_edge,
        } => {
            let _ = write!(out, "    switch v{} ->", value.index());
            for (case, target, edge) in arms {
                let _ = write!(out, " {case}: {} (e{}),", block_name(*target), edge.index());
            }
            let _ = writeln!(
                out,
                " default: {} (e{})",
                block_name(*default),
                default_edge.index()
            );
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
        Ty::Decimal => "decimal",
        Ty::Void => "void",
        Ty::Object => "object",
        Ty::Str => "string",
        Ty::Bytes => "bytes",
        Ty::Array => "array",
        Ty::Tagged => "tagged",
        Ty::Null => "null",
        Ty::Enum(crate::ty::EnumRepr::Int) => "enum:int",
        Ty::Enum(crate::ty::EnumRepr::Uint) => "enum:uint",
        Ty::ClassDesc => "classdesc",
        Ty::Ref => "ref",
    }
}

fn bin_op_name(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        BinOp::Mul => "mul",
        BinOp::Div => "div",
        BinOp::Mod => "mod",
        BinOp::Pow => "pow",
        BinOp::BitAnd => "band",
        BinOp::BitOr => "bor",
        BinOp::BitXor => "bxor",
        BinOp::Shl => "shl",
        BinOp::Shr => "shr",
        BinOp::Eq => "eq",
        BinOp::NotEq => "ne",
        BinOp::Lt => "lt",
        BinOp::LtEq => "le",
        BinOp::Gt => "gt",
        BinOp::GtEq => "ge",
        BinOp::Cmp => "cmp",
    }
}

fn un_op_name(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "neg",
        UnOp::Not => "not",
        UnOp::BitNot => "bnot",
    }
}

fn helper_name(h: Helper) -> &'static str {
    match h {
        Helper::IntToString => "int_to_string",
        Helper::UintToString => "uint_to_string",
        Helper::FloatToString => "float_to_string",
        Helper::BoolToString => "bool_to_string",
        Helper::ClassDescName => "class_desc_name",
        Helper::IntTruthy => "int_truthy",
        Helper::UintTruthy => "uint_truthy",
        Helper::FloatTruthy => "float_truthy",
        Helper::StrTruthy => "str_truthy",
        Helper::BytesTruthy => "bytes_truthy",
        Helper::ArrayTruthy => "array_truthy",
        Helper::ValueTruthy => "value_truthy",
        Helper::ArrayRowForWrite => "array_row_for_write",
        Helper::DecimalTruthy => "decimal_truthy",
        Helper::DecimalAdd => "decimal_add",
        Helper::DecimalSub => "decimal_sub",
        Helper::DecimalMul => "decimal_mul",
        Helper::DecimalDiv => "decimal_div",
        Helper::DecimalMod => "decimal_mod",
        Helper::DecimalNeg => "decimal_neg",
        Helper::DecimalEq => "decimal_eq",
        Helper::DecimalLt => "decimal_lt",
        Helper::DecimalLtEq => "decimal_lt_eq",
        Helper::DecimalCmp => "decimal_cmp",
        Helper::IntToUint => "int_to_uint",
        Helper::UintToInt => "uint_to_int",
        Helper::IntToFloat => "int_to_float",
        Helper::UintToFloat => "uint_to_float",
        Helper::FloatToInt => "float_to_int",
        Helper::FloatToUint => "float_to_uint",
        Helper::StrToInt => "str_to_int",
        Helper::StrToUint => "str_to_uint",
        Helper::StrToFloat => "str_to_float",
        Helper::BytesToString => "bytes_to_string",
        Helper::ToIntOrNull => "to_int_or_null",
        Helper::ToUintOrNull => "to_uint_or_null",
        Helper::ToFloatOrNull => "to_float_or_null",
        Helper::ToStringOrNull => "to_string_or_null",
        Helper::ToBytesOrNull => "to_bytes_or_null",
        Helper::ToDecimal => "to_decimal",
        Helper::ToDecimalOrNull => "to_decimal_or_null",
        Helper::DecimalToInt => "decimal_to_int",
        Helper::DecimalToUint => "decimal_to_uint",
        Helper::DecimalToFloat => "decimal_to_float",
        Helper::DecimalToString => "decimal_to_string",
        Helper::TaggedToString => "tagged_to_string",
        Helper::TaggedToInt => "tagged_to_int",
        Helper::TaggedToUint => "tagged_to_uint",
        Helper::TaggedToFloat => "tagged_to_float",
        Helper::TaggedToBytes => "tagged_to_bytes",
        Helper::ToArrayOf => "to_array_of",
        Helper::ToArrayOfOrNull => "to_array_of_or_null",
        Helper::EchoStr => "echo_str",
        Helper::EchoValue => "echo_value",
        Helper::Exit => "exit",
        Helper::LiteralMismatch => "literal_mismatch",
        Helper::CloneOperandNotAnObject => "clone_not_an_object",
        Helper::Identical => "identical",
        Helper::NumericEq => "numeric_eq",
        Helper::NumericLt => "numeric_lt",
        Helper::NumericCmp => "numeric_cmp",
        Helper::NumericLtEq => "numeric_lt_eq",
        Helper::ValueLt => "value_lt",
        Helper::ValueLtEq => "value_lt_eq",
        Helper::ValueCmp => "value_cmp",
        Helper::ValueAdd => "value_add",
        Helper::ValueSub => "value_sub",
        Helper::ValueMul => "value_mul",
        Helper::ValueDiv => "value_div",
        Helper::ValueMod => "value_mod",
        Helper::ValuePow => "value_pow",
        Helper::ValueBitAnd => "value_bit_and",
        Helper::ValueBitOr => "value_bit_or",
        Helper::ValueBitXor => "value_bit_xor",
        Helper::ValueShl => "value_shl",
        Helper::ValueShr => "value_shr",
        Helper::ValueNeg => "value_neg",
        Helper::ValueBitNot => "value_bit_not",
        Helper::ValueIndexGet => "value_index_get",
        Helper::ValueIndexOptionalGet => "value_index_optional_get",
        Helper::SecretEq => "secret_eq",
        Helper::CallClosure => "call_closure",
        Helper::CallClosureProven => "call_closure_proven",
        Helper::CallClosureArray => "call_closure_array",
        Helper::CallErasedMethod => "call_erased_method",
    }
}
