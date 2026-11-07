// The code of the extension. Novis calls `tokens` as `Example\Highlight\Highlighter::tokens`
// and `languages` as `Example\Highlight\Highlighter::languages`.

// This macro reads the WIT files in `wit/` and writes the Rust types for them.
// `generate_all` also writes the types of the `nvs:ext` world and of WASI.
wit_bindgen::generate!({
    path: "wit",
    world: "highlight",
    generate_all,
});

use std::sync::LazyLock;

use exports::example::highlight::api::{Error, Guest, Token};
use syntect::easy::ScopeRangeIterator;
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxSet};

// The grammars of every language. They are loaded once, at the first call.
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

// Each kind of token, and the scopes that give it. A grammar gives each piece of
// text a list of scopes, such as `keyword.control.rust`. The first scope in this
// list that starts a scope of the text gives its kind.
static KINDS: LazyLock<Vec<(Scope, &'static str)>> = LazyLock::new(|| {
    [
        ("comment", "comment"),
        ("string", "string"),
        ("constant.numeric", "number"),
        ("constant", "constant"),
        ("keyword", "keyword"),
        ("storage", "keyword"),
        ("entity.name.function", "function"),
        ("support.function", "function"),
        ("entity.name", "type"),
        ("support.type", "type"),
        ("support.class", "type"),
        ("variable", "variable"),
    ]
    .into_iter()
    .map(|(scope, kind)| (Scope::new(scope).expect("a valid scope"), kind))
    .collect()
});

struct Highlight;

impl Guest for Highlight {
    // `tokens` splits the code into tokens. Two tokens of the same kind next to
    // each other are joined, so the result is short.
    fn tokens(language: String, code: String) -> Result<Option<Vec<Token>>, Error> {
        let Some(syntax) = SYNTAXES.find_syntax_by_token(&language) else {
            return Ok(None);
        };
        let mut state = ParseState::new(syntax);
        let mut scopes = ScopeStack::new();
        let mut tokens: Vec<Token> = Vec::new();
        for line in code.split_inclusive('\n') {
            let ops = state
                .parse_line(line, &SYNTAXES)
                .map_err(|e| Error::Runtime(e.to_string()))?;
            for (range, op) in ScopeRangeIterator::new(&ops, line) {
                scopes
                    .apply(op)
                    .map_err(|e| Error::Runtime(format!("{e:?}")))?;
                if range.is_empty() {
                    continue;
                }
                let kind = kind_of(&scopes);
                match tokens.last_mut() {
                    Some(last) if last.kind == kind => last.text.push_str(&line[range]),
                    _ => tokens.push(Token {
                        text: line[range].to_string(),
                        kind: kind.to_string(),
                    }),
                }
            }
        }
        Ok(Some(tokens))
    }

    // `languages` returns the names of the languages, such as `Rust` and `Python`.
    fn languages() -> Result<Vec<String>, Error> {
        Ok(SYNTAXES
            .syntaxes()
            .iter()
            .filter(|syntax| !syntax.hidden)
            .map(|syntax| syntax.name.clone())
            .collect())
    }
}

// The kind of the innermost scope that has one, or an empty kind.
fn kind_of(scopes: &ScopeStack) -> &'static str {
    for scope in scopes.as_slice().iter().rev() {
        if let Some((_, kind)) = KINDS.iter().find(|(prefix, _)| prefix.is_prefix_of(*scope)) {
            return kind;
        }
    }
    ""
}

export!(Highlight);
