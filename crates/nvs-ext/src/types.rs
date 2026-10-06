//! The Novis types an extension signature may write, parsed from the manifest's text.
//!
//! One parser for both readers of a manifest: the loader, which checks each type against the
//! component's WIT export, and the checker in `nvs-types`, which interns it into a signature.
//! It lives outside the `engine` feature so the checker reads it without linking wasmtime.
//!
//! The types are the structural rows of `rule:packaging/a-value-crosses-as-its-wit-type`'s table:
//! `bool`, `int`, `uint`, `float`, `string`, `bytes`, `mixed`, `array<T>`, `array<K, V>`, `?T` and
//! a shape `{a: T, b?: U}`. `void` is a return, never a type here: [`crate::manifest::Method`]
//! writes it as the absence of one. A named type is outside the table until the manifest carries
//! what it names.

/// A Novis type of an extension signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NovisType {
    /// `bool`.
    Bool,
    /// `int`.
    Int,
    /// `uint`.
    Uint,
    /// `float`.
    Float,
    /// `string`.
    String,
    /// `bytes`.
    Bytes,
    /// `mixed`, which crosses as a handle.
    Mixed,
    /// `array<T>`, a list.
    List(Box<NovisType>),
    /// `array<K, V>`, a list of key and value pairs in order.
    Keyed(Box<NovisType>, Box<NovisType>),
    /// `?T`.
    Optional(Box<NovisType>),
    /// `{a: T, b?: U}`: each field's name, whether it is optional, and its type, in order.
    Shape(Vec<(String, bool, NovisType)>),
}

impl NovisType {
    /// The type `text` writes, or why it is outside the table.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut parser = TypeParser { text, at: 0 };
        let ty = parser.ty();
        parser.skip_space();
        match ty {
            Some(ty) if parser.at == text.len() => Ok(ty),
            _ => Err(format!(
                "the type `{text}` is not one an extension signature can carry"
            )),
        }
    }
}

/// A recursive-descent reader over a type's text.
struct TypeParser<'a> {
    text: &'a str,
    at: usize,
}

impl TypeParser<'_> {
    fn skip_space(&mut self) {
        let rest = &self.text[self.at..];
        self.at += rest.len() - rest.trim_start().len();
    }

    fn eat(&mut self, token: &str) -> bool {
        self.skip_space();
        if self.text[self.at..].starts_with(token) {
            self.at += token.len();
            true
        } else {
            false
        }
    }

    fn word(&mut self) -> Option<&str> {
        self.skip_space();
        let rest = &self.text[self.at..];
        let len = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if len == 0 {
            return None;
        }
        self.at += len;
        Some(&rest[..len])
    }

    fn ty(&mut self) -> Option<NovisType> {
        if self.eat("?") {
            return Some(NovisType::Optional(Box::new(self.ty()?)));
        }
        if self.eat("{") {
            let mut fields = Vec::new();
            if !self.eat("}") {
                loop {
                    let name = self.word()?.to_owned();
                    let optional = self.eat("?");
                    if !self.eat(":") {
                        return None;
                    }
                    fields.push((name, optional, self.ty()?));
                    if self.eat("}") {
                        break;
                    }
                    if !self.eat(",") {
                        return None;
                    }
                }
            }
            return Some(NovisType::Shape(fields));
        }
        Some(match self.word()? {
            "bool" => NovisType::Bool,
            "int" => NovisType::Int,
            "uint" => NovisType::Uint,
            "float" => NovisType::Float,
            "string" => NovisType::String,
            "bytes" => NovisType::Bytes,
            "mixed" => NovisType::Mixed,
            "array" => {
                if !self.eat("<") {
                    return None;
                }
                let first = self.ty()?;
                let ty = if self.eat(",") {
                    NovisType::Keyed(Box::new(first), Box::new(self.ty()?))
                } else {
                    NovisType::List(Box::new(first))
                };
                if !self.eat(">") {
                    return None;
                }
                ty
            }
            _ => return None,
        })
    }
}
