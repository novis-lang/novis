//! The Novis types an extension signature may write, parsed from the manifest's text.
//!
//! One parser for both readers of a manifest: the loader, which checks each type against the
//! component's WIT export, and the checker in `nvs-types`, which interns it into a signature.
//! It lives outside the `engine` feature so the checker reads it without linking wasmtime.
//!
//! The types are the rows of `rule:packaging/a-value-crosses-as-its-wit-type`'s table but
//! `resource`: `bool`, `int`, `uint`, `float`, `string`, `bytes`, `mixed`, `array<T>`,
//! `array<K, V>`, `?T`, a shape `{a: T, b?: U}`, and three named rows. `void` is a return, never a
//! type here: [`crate::manifest::Method`] writes it as the absence of one.
//!
//! **A named type is resolved while it is parsed**, so a [`NovisType`] carries everything its
//! conversion reads and nothing has to look a name up again. A `Core` value class is a name in
//! [`CORE_CLASSES`], written in full, `Core\Time\Instant`. An enum or a closed union of shapes is
//! a name the manifest declares (`crate::manifest::Manifest::novis_type`), written as its short
//! name. [`NovisType::parse`] reads a text with no manifest around it, so it resolves the `Core`
//! classes and refuses every other name.

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
    Shape(Vec<Field>),
    /// An enum the manifest declares: its name and its cases, in order. It crosses as a WIT `enum`
    /// whose cases are the same names in kebab-case.
    Enum {
        /// The enum's short name.
        name: String,
        /// Its cases' Novis names.
        cases: Vec<String>,
    },
    /// A closed union of shapes the manifest declares: its name, and each case's name and shape,
    /// in order. It crosses as a WIT `variant` with one case per shape.
    Union {
        /// The union's short name.
        name: String,
        /// Each case's Novis name and the fields of its shape.
        cases: Vec<(String, Vec<Field>)>,
    },
    /// A `Core` value class, which crosses as its record in `nvs:ext/types`.
    Core(&'static CoreRecord),
}

/// One field of a shape: its name, whether it is optional, and its type.
pub type Field = (String, bool, NovisType);

/// A `Core` value class that crosses, and the record of `nvs:ext/types` it crosses as.
#[derive(Debug, PartialEq, Eq)]
pub struct CoreRecord {
    /// The class's full name.
    pub class: &'static str,
    /// The record's WIT name.
    pub record: &'static str,
    /// The record's fields, in order, each with the Novis type of its value.
    pub fields: &'static [(&'static str, NovisType)],
}

/// Every `Core` value class that crosses, with its record. The list is `wit/nvs-ext/types.wit`'s,
/// and `crates/nvs-stdlib/tests/ext_world.rs` holds the two equal and the rest of the `Core` value
/// classes outside it.
pub static CORE_CLASSES: &[CoreRecord] = &[
    CoreRecord {
        class: "Core\\BigInt",
        record: "big-int",
        fields: &[
            ("negative", NovisType::Bool),
            ("magnitude", NovisType::Bytes),
        ],
    },
    CoreRecord {
        class: "Core\\Uuid",
        record: "uuid",
        fields: &[("high", NovisType::Uint), ("low", NovisType::Uint)],
    },
    CoreRecord {
        class: "Core\\Uri",
        record: "uri",
        fields: &[("text", NovisType::String)],
    },
    CoreRecord {
        class: "Core\\Crypto\\PublicKey",
        record: "public-key",
        fields: &[("spki", NovisType::Bytes)],
    },
    CoreRecord {
        class: "Core\\Time\\Instant",
        record: "instant",
        fields: &[("seconds", NovisType::Int), ("nanos", NovisType::Int)],
    },
    CoreRecord {
        class: "Core\\Time\\DateTime",
        record: "date-time",
        fields: &[
            ("seconds", NovisType::Int),
            ("nanos", NovisType::Int),
            ("zone", NovisType::String),
        ],
    },
    CoreRecord {
        class: "Core\\Time\\Date",
        record: "date",
        fields: &[
            ("year", NovisType::Int),
            ("month", NovisType::Int),
            ("day", NovisType::Int),
        ],
    },
    CoreRecord {
        class: "Core\\Time\\TimeOfDay",
        record: "time-of-day",
        fields: &[
            ("hour", NovisType::Int),
            ("minute", NovisType::Int),
            ("second", NovisType::Int),
            ("nanos", NovisType::Int),
        ],
    },
    CoreRecord {
        class: "Core\\Time\\Duration",
        record: "duration",
        fields: &[("nanos", NovisType::Int)],
    },
    CoreRecord {
        class: "Core\\Time\\Zone",
        record: "zone",
        fields: &[("id", NovisType::String)],
    },
];

impl NovisType {
    /// The type `text` writes, or why it is outside the table. A name other than a `Core` value
    /// class is outside it: only a manifest declares one.
    pub fn parse(text: &str) -> Result<Self, String> {
        Self::parse_with(text, &|_| None)
    }

    /// The type `text` writes, where `named` resolves a name the manifest declares.
    pub fn parse_with(text: &str, named: &dyn Fn(&str) -> Option<Self>) -> Result<Self, String> {
        let mut parser = TypeParser { text, at: 0, named };
        let ty = parser.ty();
        parser.skip_space();
        match ty {
            Some(ty) if parser.at == text.len() => Ok(ty),
            _ => Err(format!(
                "the type `{text}` is not one an extension signature can carry"
            )),
        }
    }

    /// The fields of the shape `text` writes, or why it is not a shape.
    pub fn parse_shape(
        text: &str,
        named: &dyn Fn(&str) -> Option<Self>,
    ) -> Result<Vec<Field>, String> {
        match Self::parse_with(text, named)? {
            Self::Shape(fields) => Ok(fields),
            _ => Err(format!("the type `{text}` is not a shape")),
        }
    }
}

/// A recursive-descent reader over a type's text.
struct TypeParser<'a> {
    text: &'a str,
    at: usize,
    named: &'a dyn Fn(&str) -> Option<NovisType>,
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

    /// The next run of characters `part` accepts.
    fn take(&mut self, part: fn(char) -> bool) -> Option<&str> {
        self.skip_space();
        let rest = &self.text[self.at..];
        let len = rest.find(|c: char| !part(c)).unwrap_or(rest.len());
        if len == 0 {
            return None;
        }
        self.at += len;
        Some(&rest[..len])
    }

    fn word(&mut self) -> Option<&str> {
        self.take(|c| c.is_ascii_alphanumeric() || c == '_')
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
        let named = self.named;
        Some(
            match self.take(|c| c.is_ascii_alphanumeric() || c == '_' || c == '\\')? {
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
                name => match CORE_CLASSES.iter().find(|core| core.class == name) {
                    Some(core) => NovisType::Core(core),
                    None => named(name)?,
                },
            },
        )
    }
}
