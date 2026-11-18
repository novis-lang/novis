use nvs_diagnostics::{Diagnostics, SourceMap};

/// What [`compile_source`] made of one source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compiled {
    /// The front end reported an error, the first of which is here, so
    /// nothing was lowered.
    Diagnosed(String),
    /// The program checked with no error and code generation built it.
    Built,
}

/// The whole compiler over one source text: the parser, the name resolution,
/// the type checker, the lowering and code generation.
///
/// Code generation writes an object file and maps nothing, so no code is run
/// and no page is left behind. Two properties hold on every input: nothing
/// panics, and a program the checker passed with no error is built. A
/// [`nvs_codegen::CodegenError`] on such a program is a shape the checker let
/// through and the backend cannot lower, which is a bug in one of the two.
///
/// The input is the source text as it stands, so any `.nvs` file is a seed.
/// Bytes that are not UTF-8 are no source at all, and are skipped.
pub fn compile(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Err(error) = compile_source(text) {
        panic!("a program the checker passed failed code generation: {error}");
    }
}

/// Runs `text` through every phase [`compile`] names, and gives what came of
/// it. A panic in any phase is not caught here.
///
/// # Errors
///
/// The code generation error of a program the checker passed with no error.
pub fn compile_source(text: &str) -> Result<Compiled, nvs_codegen::CodegenError> {
    match lower_source(text) {
        Err(first) => Ok(Compiled::Diagnosed(first)),
        Ok(program) => {
            nvs_codegen::compile_object(&program)?;
            Ok(Compiled::Built)
        }
    }
}

/// Runs the front end and the lowering over `text`, and gives the program or
/// the first error the front end reported. A panic in any phase is not caught
/// here.
pub(crate) fn lower_source(text: &str) -> Result<nvs_ir::Program, String> {
    let mut map = SourceMap::new();
    let id = map.add("fuzz.nvs", text.to_owned());
    let src = map.file(id);

    let mut diags = Diagnostics::new();
    let stmts = nvs_syntax::parse_file(src, &mut diags);
    let module = nvs_hir::resolve_file(&stmts, src, &mut diags);
    let mut interner = nvs_types::TypeInterner::new();
    let mut exprs = nvs_types::ExprTypeTable::new();
    let files = [nvs_types::ProgramFile { src, stmts: &stmts }];
    let enums = nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
    if diags.has_errors() {
        let first = diags
            .iter()
            .find(|d| d.severity == nvs_diagnostics::Severity::Error)
            .map(|d| match d.code {
                Some(code) => format!("{code}: {}", d.message),
                None => d.message.clone(),
            })
            .unwrap_or_default();
        return Err(first);
    }

    let layouts = nvs_types::build_class_layouts(&files, &module.graph);
    Ok(nvs_ir::lower::lower_file(
        nvs_ir::lower::ENTRY_SCRIPT_LABEL,
        &stmts,
        src,
        &exprs,
        &interner,
        &enums,
        &layouts,
    ))
}
