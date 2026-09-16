//! `rule:core-classes/schema-is-a-value`'s
//! closed vocabulary: a database schema as a value, with nothing opaque inside
//! it.
//!
//! A [`Schema`] is tables; a [`Table`] is columns, a primary key, unique
//! constraints and indexes; a [`Column`] is a name, a [`ScalarType`], whether it
//! is nullable, whether it is the table's identity, and a [`ColumnDefault`].
//! That is the whole vocabulary and there is deliberately no way to put a
//! fragment of SQL inside one — § 2's rule, and the reason the diff over these
//! types can be **total**: nothing here is a string the difference engine cannot
//! reason about.
//!
//! # The write direction of `rule:core-classes/db-column-types`
//!
//! [`ScalarType`] is [`crate::ColumnType`] plus the parameters a `CREATE TABLE`
//! has to state. That is on purpose and it is one table used twice rather than
//! two tables to keep in step: [`ScalarType::describes`] is the join, it is
//! total, and it never answers [`ColumnType::Other`] — a type outside the
//! vocabulary is a column this crate can *read* and cannot *write*, which is
//! exactly what a closed vocabulary means.
//!
//! # Why the validation is here and not above
//!
//! An identifier is **validated, never delimited**
//! (`rule:security/tainted-qualifier`; DDL has no
//! parameters to bind a name through), and
//! [`is_bare_identifier`] is that judgement's one home — `Core\Db::quoteIdentifier`
//! calls it rather than restating it. Everything else a construction can get
//! wrong is refused here too, so that a [`Schema`] value in hand is one the
//! emitters may assume is coherent: a key naming a column the table has not got,
//! a nullable primary key, a default no backend will accept. An emitter that
//! re-checked would be a second answer to the same question.
//!
//! # The canonical form
//!
//! § 1 makes the array form canonical — two schemas are the same schema exactly
//! when their array forms agree — and [`Schema::to_array`] is where that form
//! is defined, over [`Node`] rather than over a Novis value, because this crate
//! is sans-io. It is **ordered**: declaration order for columns, name order for
//! tables, constraints and indexes. `Schema::from_array` reads it back through
//! the same builders a program calls, so a file cannot say anything a program
//! could not have built.
//!
//! # § 11's exclusions are not represented, and are not added casually
//!
//! Foreign keys, partial and expression indexes, index types, collations,
//! check constraints and the rest are out of v1 because they have no portable
//! spelling, and the vocabulary grows only when a construct exists on every
//! backend *and* something needs it.
//!
//! # Known gaps
//!
//! 1. **Two constructs this vocabulary holds are not portable, and nothing
//!    here refuses either.** Both were found by applying a schema to all five
//!    servers — `catalog`'s `an_applied_schema_introspects_back_to_an_empty_plan_on_*`
//!    is that walk, and its fixture's own doc is where each is written down.
//!    An **index over unbounded text** is refused outright by SQL Server, whose
//!    key column may not be `NVARCHAR(MAX)`, and taken by MySQL only as
//!    [`crate::ddl`]'s prefix key. An **identifier a backend reserves** —
//!    `RANK` is a keyword on MySQL 8 — is a `CREATE TABLE` that server will not
//!    parse, and no emitter here can prevent it, because
//!    `rule:core-classes/schema-is-a-value` validates an identifier rather than
//!    delimiting it. Closing either is a builder that refuses the construct or
//!    an emitter that has a spelling for it, and which one is a decision the
//!    milestone that needs it takes.
//!    Decided: Emitter always quotes identifiers; builder refuses an index over unbounded text —
//!    Quoting is standard and cheap; the text-index refusal is honest because there is no portable
//!    spelling.
//!    — owner: unowned-closures

use std::fmt;

use crate::conn::ColumnType;

/// The longest identifier every backend accepts.
///
/// PostgreSQL truncates at 63 bytes (`NAMEDATALEN - 1`) and does it *silently*,
/// which is worse than refusing: two names differing after the 63rd byte become
/// one object, and the diff would then converge forever on a table it believes
/// is missing. MySQL stops at 64 and SQL Server at 128, so PostgreSQL's floor is
/// the portable one.
pub const MAX_IDENTIFIER: usize = 63;

/// Whether `name` is an identifier every backend reads the same way, and which
/// carries nothing into the statement text it is written into.
///
/// ASCII on purpose. Every backend also accepts some set of non-ASCII letters,
/// and no two of those sets are the same — `char::is_alphabetic` would accept a
/// name PostgreSQL takes and SQL Server folds differently, which is exactly the
/// dialect dependence validating instead of delimiting exists to avoid having.
///
/// This is the length-free half of the rule, because it is also
/// `Core\Db::quoteIdentifier`'s whole rule: that member laundering a name for a
/// statement someone else wrote has no table to compare a length against, where
/// [`Ident::new`] is building a schema object and does.
#[must_use]
pub fn is_bare_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// A validated identifier: a table, column, constraint or index name.
///
/// Comparison is case-**insensitive**, because the backends disagree about
/// folding and a schema that means different things on two of them is not
/// portable. Two columns named `id` and `ID` are one column here, and declaring
/// both is refused.
#[derive(Debug, Clone)]
pub struct Ident(String);

impl Ident {
    /// The identifier, or why this string is not one.
    ///
    /// # Errors
    ///
    /// [`SchemaError::NotAnIdentifier`] for anything [`is_bare_identifier`]
    /// refuses, and [`SchemaError::IdentifierTooLong`] past [`MAX_IDENTIFIER`].
    pub fn new(name: &str) -> Result<Ident, SchemaError> {
        if !is_bare_identifier(name) {
            return Err(SchemaError::NotAnIdentifier(name.to_owned()));
        }
        if name.len() > MAX_IDENTIFIER {
            return Err(SchemaError::IdentifierTooLong(name.to_owned()));
        }
        Ok(Ident(name.to_owned()))
    }

    /// The name as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Ident {
    fn eq(&self, other: &Ident) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for Ident {}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// How wide an integer column is, since the widths are not one type on any
/// backend and a narrowing is a table rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntWidth {
    /// `SMALLINT`, 16 bits.
    Small,
    /// `INTEGER`, 32 bits.
    Normal,
    /// `BIGINT`, 64 bits.
    Big,
}

impl IntWidth {
    /// How many bits wide, which is also what the canonical spelling counts.
    #[must_use]
    pub fn bits(self) -> u16 {
        match self {
            IntWidth::Small => 16,
            IntWidth::Normal => 32,
            IntWidth::Big => 64,
        }
    }
}

/// Which of the two binary floating-point columns, which every backend spells
/// with two names and no more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatWidth {
    /// `REAL`, 32 bits.
    Single,
    /// `DOUBLE PRECISION`, 64 bits.
    Double,
}

impl FloatWidth {
    /// How many bits wide, which is also what the canonical spelling counts.
    #[must_use]
    pub fn bits(self) -> u16 {
        match self {
            FloatWidth::Single => 32,
            FloatWidth::Double => 64,
        }
    }
}

/// A column's type: [ADR 0067 § 9](/docs/decisions/0067.md)'s map in the
/// write direction, with the parameters a `CREATE TABLE` must carry.
///
/// Where § 9's read direction is many-to-one — `SMALLINT`, `INTEGER` and
/// `BIGINT` all read as `int` — this keeps the distinction, because the writer
/// chose it and the introspector can recover it. [`ScalarType::describes`] is
/// the arrow back onto the read direction's own enum, and the two cannot drift
/// while that function is total.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarType {
    /// A signed integer of one of the portable widths.
    Int(IntWidth),
    /// An unsigned integer. MySQL and MariaDB spell it `UNSIGNED`; the others
    /// have no unsigned integer and take the next width up with a check the
    /// emitter writes.
    Uint(IntWidth),
    /// Binary floating point.
    Float(FloatWidth),
    /// `DECIMAL(p, s)` — the exact type `rule:types/decimal`
    /// makes a Novis `decimal`.
    Decimal {
        /// Total significant digits, 1 to 38 — SQL Server's ceiling, which is
        /// the lowest any backend imposes.
        precision: u8,
        /// Digits after the point, no greater than `precision`.
        scale: u8,
    },
    /// Text. `Some(n)` is `VARCHAR(n)` and `None` is the backend's unbounded
    /// text type, which is a different type on every backend and is why the
    /// distinction is in the vocabulary rather than in the emitter.
    Text {
        /// The declared length in characters, or `None` for unbounded.
        max: Option<u32>,
    },
    /// Binary. `Some(n)` is `VARBINARY(n)`, `None` the unbounded blob type.
    Bytes {
        /// The declared length in bytes, or `None` for unbounded.
        max: Option<u32>,
    },
    /// A boolean, spelled `BIT` on SQL Server and `TINYINT(1)` nowhere — § 9
    /// reads `TINYINT(1)` as an `int`, so writing one would not round-trip.
    Bool,
    /// A calendar date.
    Date,
    /// A time of day.
    Time,
    /// A zone-less date and time, read in the zone the connection declares.
    DateTime,
    /// A date and time carrying its own offset — `TIMESTAMPTZ`,
    /// `datetimeoffset`.
    Instant,
    /// A UUID, where the backend has a type for one.
    Uuid,
    /// A JSON document, where the backend has a type for one. The value still
    /// reads as a `tainted string`: § 9's rule that JSON is never auto-decoded
    /// is untouched.
    Json,
}

impl ScalarType {
    /// The read direction's case for a column written as this type.
    ///
    /// Total, injective, and never [`ColumnType::Other`] — that case is every
    /// type with no Novis type of its own, and the vocabulary contains none of
    /// them by construction.
    /// `the_column_type_vocabulary_is_adr_0067_section_9s_map_in_the_write_direction`
    /// holds all three properties.
    #[must_use]
    pub fn describes(&self) -> ColumnType {
        match self {
            ScalarType::Int(_) => ColumnType::Int,
            ScalarType::Uint(_) => ColumnType::Uint,
            ScalarType::Float(_) => ColumnType::Float,
            ScalarType::Decimal { .. } => ColumnType::Decimal,
            ScalarType::Text { .. } => ColumnType::Text,
            ScalarType::Bytes { .. } => ColumnType::Bytes,
            ScalarType::Bool => ColumnType::Bool,
            ScalarType::Date => ColumnType::Date,
            ScalarType::Time => ColumnType::Time,
            ScalarType::DateTime => ColumnType::DateTime,
            ScalarType::Instant => ColumnType::Instant,
            ScalarType::Uuid => ColumnType::Uuid,
            ScalarType::Json => ColumnType::Json,
        }
    }

    /// Whether this is an integer type, which is what an identity column may
    /// be.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(self, ScalarType::Int(_) | ScalarType::Uint(_))
    }

    /// The bounds every backend agrees on, checked wherever a type is built
    /// from text — [`ScalarType::from_spelling`]'s canonical names and
    /// [`crate::catalog::scalar_type`]'s catalog spellings alike.
    pub(crate) fn check(&self) -> Result<(), SchemaError> {
        match *self {
            ScalarType::Decimal { precision, scale } => {
                if precision == 0 || precision > 38 || scale > precision {
                    return Err(SchemaError::BadPrecision { precision, scale });
                }
            }
            ScalarType::Text { max: Some(0) } | ScalarType::Bytes { max: Some(0) } => {
                return Err(SchemaError::ZeroWidth);
            }
            _ => {}
        }
        Ok(())
    }
}

/// § 2's closed set of defaults: a literal of a vocabulary scalar type, the
/// current timestamp, or — read off a server and never written — the words of
/// a default this set cannot say.
///
/// A program writes no expression defaults, and that one rule closes two holes
/// at once. An expression is an unbindable string reaching a DDL sink, and it is
/// also the value a server is most likely to spell back differently — which is
/// the normalization difficulty § 5 exists to survive. [`ColumnDefault::Opaque`]
/// is the reading half of that rule and not an exception to it: it carries such
/// a string out of a server and into a comparison, and there is no path from it
/// back into statement text.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnDefault {
    /// A signed integer literal.
    Int(i64),
    /// An unsigned integer literal.
    Uint(u64),
    /// A binary floating-point literal.
    Float(f64),
    /// An exact decimal literal, as digits with an optional sign and point.
    Decimal(String),
    /// A text literal, emitted as a quoted string.
    Text(String),
    /// A boolean literal.
    Bool(bool),
    /// The server's current timestamp at insert — `CURRENT_TIMESTAMP`, the one
    /// spelling every backend shares.
    Now,
    /// A default a server reported that no case above can hold, kept verbatim
    /// in the server's own words and compared as text.
    ///
    /// Read-only, in two separate halves. Nothing emits it: [`crate::ddl`]'s
    /// `literal` has an arm for it only so that match stays exhaustive, and a
    /// step carries the *wanted* schema's default, which a program wrote.
    /// Nothing outside a catalog read builds one either, because the canonical
    /// form has no key it can arrive under — [`ColumnDefault::from_node`]
    /// refuses the key [`ColumnDefault::to_node`] writes.
    ///
    /// It exists so that a column whose default this vocabulary cannot say
    /// still compares equal to itself across two reads. Reading such a default
    /// as nothing at all is the alternative, and it makes § 5's plan propose
    /// the same step on every run, which no number of applies converges.
    Opaque(String),
}

impl ColumnDefault {
    /// Whether this literal may be written as `ty`'s default on every backend.
    ///
    /// The refusals here that are portability rather than typing are MySQL's:
    /// a `BLOB`, a `TEXT` and a `JSON` column take no literal default
    /// at all before 8.0.13 and only a parenthesized expression after, and an
    /// expression is not in this set. So an unbounded [`ScalarType::Text`],
    /// every [`ScalarType::Bytes`], [`ScalarType::Json`] and
    /// [`ScalarType::Uuid`] carry no default, and a `VARCHAR(n)` carries one
    /// happily.
    ///
    /// This is also the rule [`crate::catalog::column_default`] reads a
    /// server's own default against, so a literal that could not have been
    /// written is not one an introspection invents either.
    ///
    /// [`ColumnDefault::Opaque`] fits every type, and that is not a hole in the
    /// same rule: the question here is whether a literal may be *written* on
    /// every backend, and that case is never written anywhere. What it holds is
    /// the text one server already has on that column, so the only backend it
    /// has to be portable across is the one it was read from.
    pub(crate) fn fits(&self, ty: &ScalarType) -> bool {
        matches!(
            (self, ty),
            (ColumnDefault::Int(_), ScalarType::Int(_))
                | (ColumnDefault::Uint(_), ScalarType::Uint(_))
                | (ColumnDefault::Float(_), ScalarType::Float(_))
                | (ColumnDefault::Decimal(_), ScalarType::Decimal { .. })
                | (ColumnDefault::Bool(_), ScalarType::Bool)
                | (ColumnDefault::Text(_), ScalarType::Text { max: Some(_) })
                | (
                    ColumnDefault::Now,
                    ScalarType::DateTime | ScalarType::Instant
                )
                | (ColumnDefault::Opaque(_), _)
        )
    }
}

/// One column of a table.
///
/// Built rather than constructed: the fields are private because a value that
/// exists is one every rule above has already passed, and a mutable field would
/// let a caller produce a column no emitter is prepared for.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    name: Ident,
    ty: ScalarType,
    nullable: bool,
    identity: bool,
    default: Option<ColumnDefault>,
}

impl Column {
    /// A `NOT NULL` column with no default and no identity.
    ///
    /// # Errors
    ///
    /// The identifier's own refusals, and [`SchemaError::BadPrecision`] or
    /// [`SchemaError::ZeroWidth`] for a type parameter no backend accepts.
    pub fn new(name: &str, ty: ScalarType) -> Result<Column, SchemaError> {
        ty.check()?;
        Ok(Column {
            name: Ident::new(name)?,
            ty,
            nullable: false,
            identity: false,
            default: None,
        })
    }

    /// The same column, nullable.
    #[must_use]
    pub fn null(mut self) -> Column {
        self.nullable = true;
        self
    }

    /// The same column as the table's identity — `GENERATED AS IDENTITY`,
    /// `AUTO_INCREMENT`, `IDENTITY(1,1)`, `INTEGER PRIMARY KEY AUTOINCREMENT`,
    /// one spelling per dialect.
    ///
    /// An identity column is never nullable and never carries a default: the
    /// server supplies the value, which is the whole construct.
    ///
    /// # Errors
    ///
    /// [`SchemaError::IdentityNotInteger`] for anything but an integer type.
    /// A table's own rules — at most one, and it must be the primary key — are
    /// [`Table`]'s, because they are not answerable from one column.
    pub fn identity(mut self) -> Result<Column, SchemaError> {
        if !self.ty.is_integer() {
            return Err(SchemaError::IdentityNotInteger(self.name));
        }
        self.identity = true;
        self.nullable = false;
        self.default = None;
        Ok(self)
    }

    /// The same column with a default.
    ///
    /// # Errors
    ///
    /// [`SchemaError::DefaultDoesNotFit`] when the literal is not one this type
    /// takes on every backend — [`ColumnDefault::fits`] says which pairs those
    /// are and why some of them are portability rather than typing.
    pub fn default(mut self, value: ColumnDefault) -> Result<Column, SchemaError> {
        if !value.fits(&self.ty) {
            return Err(SchemaError::DefaultDoesNotFit(self.name));
        }
        self.default = Some(value);
        Ok(self)
    }

    /// The column's name.
    #[must_use]
    pub fn name(&self) -> &Ident {
        &self.name
    }

    /// The column's type.
    #[must_use]
    pub fn ty(&self) -> &ScalarType {
        &self.ty
    }

    /// Whether the column accepts null.
    #[must_use]
    pub fn is_nullable(&self) -> bool {
        self.nullable
    }

    /// Whether the server supplies this column's value.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.identity
    }

    /// The column's default, if it has one.
    #[must_use]
    pub fn default_value(&self) -> Option<&ColumnDefault> {
        self.default.as_ref()
    }
}

/// A named list of columns: a unique constraint, or an index.
///
/// One type for both, because they differ in what a dialect emits and in
/// nothing a schema value has to say. **An index is never unique** — a unique
/// key is the canonical spelling, and admitting both would make one schema
/// expressible two ways, which is precisely the phantom difference § 5 spends
/// its normalization budget removing.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    name: Ident,
    columns: Vec<Ident>,
}

impl Key {
    /// The key's name, which is unique across the whole schema.
    #[must_use]
    pub fn name(&self) -> &Ident {
        &self.name
    }

    /// Its columns, in the order the index is built on.
    #[must_use]
    pub fn columns(&self) -> &[Ident] {
        &self.columns
    }
}

/// One table: its columns in declaration order, its primary key, its unique
/// constraints and its indexes.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    name: Ident,
    columns: Vec<Column>,
    primary_key: Vec<Ident>,
    unique: Vec<Key>,
    indexes: Vec<Key>,
}

impl Table {
    /// A table with columns and nothing else.
    ///
    /// # Errors
    ///
    /// [`SchemaError::NoColumns`] for an empty list, [`SchemaError::Duplicate`]
    /// for two columns whose names differ only in case, and
    /// [`SchemaError::TwoIdentities`] for a second identity column.
    pub fn new(name: &str, columns: Vec<Column>) -> Result<Table, SchemaError> {
        let name = Ident::new(name)?;
        if columns.is_empty() {
            return Err(SchemaError::NoColumns(name));
        }
        for (at, column) in columns.iter().enumerate() {
            if columns[..at].iter().any(|prior| prior.name == column.name) {
                return Err(SchemaError::Duplicate {
                    what: "column",
                    name: column.name.clone(),
                });
            }
        }
        if columns.iter().filter(|column| column.identity).count() > 1 {
            return Err(SchemaError::TwoIdentities(name));
        }
        Ok(Table {
            name,
            columns,
            primary_key: Vec::new(),
            unique: Vec::new(),
            indexes: Vec::new(),
        })
    }

    /// The same table with a primary key over `columns`.
    ///
    /// # Errors
    ///
    /// [`SchemaError::EmptyKey`], [`SchemaError::UnknownColumn`] for a name the
    /// table has not got, [`SchemaError::Duplicate`] for a column named twice
    /// in one key, and [`SchemaError::NullableInKey`] for a nullable one — no
    /// backend admits null into a primary key, so refusing here is the same
    /// answer arriving before the server is asked.
    pub fn primary_key(mut self, columns: &[&str]) -> Result<Table, SchemaError> {
        self.primary_key = self.resolve(columns, "primary key")?;
        for name in &self.primary_key {
            if self.column(name).is_some_and(Column::is_nullable) {
                return Err(SchemaError::NullableInKey(name.clone()));
            }
        }
        Ok(self)
    }

    /// The same table with a unique constraint.
    ///
    /// # Errors
    ///
    /// [`Table::primary_key`]'s, minus the nullability rule — a unique
    /// constraint over a nullable column is legal everywhere, and what a null
    /// collides with differs by backend without differing in the schema.
    pub fn unique(mut self, name: &str, columns: &[&str]) -> Result<Table, SchemaError> {
        let key = Key {
            name: Ident::new(name)?,
            columns: self.resolve(columns, "unique constraint")?,
        };
        self.unique.push(key);
        Ok(self)
    }

    /// The same table with a non-unique index.
    ///
    /// # Errors
    ///
    /// [`Table::unique`]'s.
    pub fn index(mut self, name: &str, columns: &[&str]) -> Result<Table, SchemaError> {
        let key = Key {
            name: Ident::new(name)?,
            columns: self.resolve(columns, "index")?,
        };
        self.indexes.push(key);
        Ok(self)
    }

    /// The table's name.
    #[must_use]
    pub fn name(&self) -> &Ident {
        &self.name
    }

    /// Its columns, in declaration order — which is the order a `CREATE TABLE`
    /// must reproduce, and so part of the value.
    #[must_use]
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Its primary key's columns, empty for a table that declares none.
    #[must_use]
    pub fn primary_key_columns(&self) -> &[Ident] {
        &self.primary_key
    }

    /// Its unique constraints.
    #[must_use]
    pub fn unique_keys(&self) -> &[Key] {
        &self.unique
    }

    /// Its non-unique indexes.
    #[must_use]
    pub fn indexes(&self) -> &[Key] {
        &self.indexes
    }

    /// The column of that name.
    #[must_use]
    pub fn column(&self, name: &Ident) -> Option<&Column> {
        self.columns.iter().find(|column| &column.name == name)
    }

    /// Validate the names in a key against the table's own columns.
    fn resolve(&self, columns: &[&str], what: &'static str) -> Result<Vec<Ident>, SchemaError> {
        if columns.is_empty() {
            return Err(SchemaError::EmptyKey(self.name.clone()));
        }
        let mut resolved: Vec<Ident> = Vec::with_capacity(columns.len());
        for name in columns {
            let name = Ident::new(name)?;
            if self.column(&name).is_none() {
                return Err(SchemaError::UnknownColumn {
                    table: self.name.clone(),
                    column: name,
                });
            }
            if resolved.contains(&name) {
                return Err(SchemaError::Duplicate { what, name });
            }
            resolved.push(name);
        }
        Ok(resolved)
    }

    /// The rules no single builder call can answer.
    fn check(&self) -> Result<(), SchemaError> {
        let Some(identity) = self.columns.iter().find(|column| column.identity) else {
            return Ok(());
        };
        if !self.primary_key.contains(&identity.name) {
            return Err(SchemaError::IdentityNotInKey(identity.name.clone()));
        }
        Ok(())
    }
}

/// A whole schema: the tables a program says should exist.
///
/// § 7's rule is what makes this a *should* and not a *must*: a table the
/// server has and this value has not is reported and never dropped, because an
/// application shares its database with the queue, with a reporting view and
/// with whatever an operator put there.
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    tables: Vec<Table>,
}

impl Schema {
    /// The schema, or the first rule one of its tables breaks.
    ///
    /// # Errors
    ///
    /// [`SchemaError::Duplicate`] for two tables of one name, or for a
    /// constraint or index name used twice **anywhere** in the schema —
    /// PostgreSQL's index names are schema-scoped where MySQL's are
    /// table-scoped, so the portable floor is the stricter of the two.
    /// [`SchemaError::IdentityNotInKey`] for an identity column outside its
    /// table's primary key.
    pub fn new(tables: Vec<Table>) -> Result<Schema, SchemaError> {
        let mut keys: Vec<Ident> = Vec::new();
        for (at, table) in tables.iter().enumerate() {
            table.check()?;
            if tables[..at].iter().any(|prior| prior.name == table.name) {
                return Err(SchemaError::Duplicate {
                    what: "table",
                    name: table.name.clone(),
                });
            }
            for key in table.unique.iter().chain(&table.indexes) {
                if keys.contains(&key.name) {
                    return Err(SchemaError::Duplicate {
                        what: "key",
                        name: key.name.clone(),
                    });
                }
                keys.push(key.name.clone());
            }
        }
        Ok(Schema { tables })
    }

    /// The tables.
    #[must_use]
    pub fn tables(&self) -> &[Table] {
        &self.tables
    }

    /// The table of that name.
    #[must_use]
    pub fn table(&self, name: &Ident) -> Option<&Table> {
        self.tables.iter().find(|table| &table.name == name)
    }
}

/// One node of [ADR 0145 § 1](/docs/decisions/0145.md)'s
/// canonical array form.
///
/// The array form is what makes a built schema, a saved file and an
/// introspected database *one* value: two schemas are the same schema exactly
/// when their array forms agree. That comparison has to happen somewhere, and
/// it happens here — this crate is sans-io and holds no Novis value, so the
/// form needs a small ordered node of its own rather than a second
/// serialization at each end. `nvs-stdlib` turns this into a Novis array and
/// back, once, and nothing else converts anything.
///
/// A [`Node::Map`] is a `Vec` of pairs rather than a map type on purpose:
/// `rule:expressions/object-identity-equality` makes key order part of an array's value, so key order is data here too
/// and nothing on the way past may reorder it.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A string.
    Text(String),
    /// A signed integer.
    Int(i64),
    /// An unsigned integer, which `rule:core-classes/db-column-types` makes a Novis type of its own.
    Uint(u64),
    /// A binary floating-point number.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// An ordered list.
    List(Vec<Node>),
    /// An ordered map.
    Map(Vec<(String, Node)>),
}

impl Node {
    /// The value at `key`, or `None` for a missing key or a node that is not a
    /// map at all.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Node> {
        match self {
            Node::Map(pairs) => pairs.iter().find(|(at, _)| at == key).map(|(_, node)| node),
            _ => None,
        }
    }

    /// The string at `key`, which every reader below requires.
    fn text(&self, key: &'static str) -> Result<&str, SchemaError> {
        match self.get(key) {
            Some(Node::Text(text)) => Ok(text),
            _ => Err(SchemaError::BadArray {
                key,
                wanted: "a string",
            }),
        }
    }

    /// The list at `key`, where a missing key is the empty list — so a table
    /// with no indexes may leave the key out and still read back.
    fn list(&self, key: &'static str) -> Result<&[Node], SchemaError> {
        match self.get(key) {
            Some(Node::List(items)) => Ok(items),
            None => Ok(&[]),
            Some(_) => Err(SchemaError::BadArray {
                key,
                wanted: "a list",
            }),
        }
    }

    /// The list of names at `key`, in the order it states them.
    fn names(&self, key: &'static str) -> Result<Vec<&str>, SchemaError> {
        self.list(key)?
            .iter()
            .map(|node| match node {
                Node::Text(name) => Ok(name.as_str()),
                _ => Err(SchemaError::BadArray {
                    key,
                    wanted: "a list of names",
                }),
            })
            .collect()
    }

    /// The flag at `key`, where a missing key is `false`.
    fn flag(&self, key: &'static str) -> Result<bool, SchemaError> {
        match self.get(key) {
            Some(Node::Bool(set)) => Ok(*set),
            None => Ok(false),
            Some(_) => Err(SchemaError::BadArray {
                key,
                wanted: "true or false",
            }),
        }
    }
}

fn pair(key: &str, node: Node) -> (String, Node) {
    (key.to_string(), node)
}

fn name_node(name: &Ident) -> Node {
    Node::Text(name.as_str().to_string())
}

fn name_list(names: &[Ident]) -> Node {
    Node::List(names.iter().map(name_node).collect())
}

impl fmt::Display for ScalarType {
    /// The canonical spelling, which is the only place a type is named in the
    /// array form — one string rather than a map of a case and its parameters,
    /// because this is also what a human writes into a schema file.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScalarType::Int(width) => write!(f, "int{}", width.bits()),
            ScalarType::Uint(width) => write!(f, "uint{}", width.bits()),
            ScalarType::Float(width) => write!(f, "float{}", width.bits()),
            ScalarType::Decimal { precision, scale } => write!(f, "decimal({precision},{scale})"),
            ScalarType::Text { max: Some(max) } => write!(f, "text({max})"),
            ScalarType::Text { max: None } => f.write_str("text"),
            ScalarType::Bytes { max: Some(max) } => write!(f, "bytes({max})"),
            ScalarType::Bytes { max: None } => f.write_str("bytes"),
            ScalarType::Bool => f.write_str("bool"),
            ScalarType::Date => f.write_str("date"),
            ScalarType::Time => f.write_str("time"),
            ScalarType::DateTime => f.write_str("datetime"),
            ScalarType::Instant => f.write_str("instant"),
            ScalarType::Uuid => f.write_str("uuid"),
            ScalarType::Json => f.write_str("json"),
        }
    }
}

impl ScalarType {
    /// The type a canonical spelling names.
    ///
    /// Whitespace inside the parentheses is accepted, so a spelling a person
    /// typed reads the same as one [`ScalarType`]'s [`fmt::Display`] wrote.
    /// This is not a SQL parser and must not become one — it reads this
    /// vocabulary's own closed set of names, and a spelling outside it is
    /// refused rather than passed through.
    ///
    /// # Errors
    ///
    /// [`SchemaError::UnknownType`] for a name the vocabulary has not got or a
    /// parameter that is not a number, and the parameter refusals
    /// [`SchemaError::BadPrecision`] and [`SchemaError::ZeroWidth`] for one
    /// that is a number no backend accepts.
    pub fn from_spelling(text: &str) -> Result<ScalarType, SchemaError> {
        fn unknown(text: &str) -> SchemaError {
            SchemaError::UnknownType(text.to_string())
        }
        let (head, args) = match (text.find('('), text.strip_suffix(')')) {
            (Some(open), Some(body)) => (&text[..open], Some(body[open + 1..].trim())),
            (None, None) => (text, None),
            _ => return Err(unknown(text)),
        };
        let number = |digits: &str| digits.trim().parse::<u32>().map_err(|_| unknown(text));
        let ty = match (head, args) {
            ("int16", None) => ScalarType::Int(IntWidth::Small),
            ("int32", None) => ScalarType::Int(IntWidth::Normal),
            ("int64", None) => ScalarType::Int(IntWidth::Big),
            ("uint16", None) => ScalarType::Uint(IntWidth::Small),
            ("uint32", None) => ScalarType::Uint(IntWidth::Normal),
            ("uint64", None) => ScalarType::Uint(IntWidth::Big),
            ("float32", None) => ScalarType::Float(FloatWidth::Single),
            ("float64", None) => ScalarType::Float(FloatWidth::Double),
            ("decimal", Some(body)) => {
                let (precision, scale) = body.split_once(',').ok_or_else(|| unknown(text))?;
                ScalarType::Decimal {
                    precision: u8::try_from(number(precision)?).map_err(|_| unknown(text))?,
                    scale: u8::try_from(number(scale)?).map_err(|_| unknown(text))?,
                }
            }
            ("text", None) => ScalarType::Text { max: None },
            ("text", Some(body)) => ScalarType::Text {
                max: Some(number(body)?),
            },
            ("bytes", None) => ScalarType::Bytes { max: None },
            ("bytes", Some(body)) => ScalarType::Bytes {
                max: Some(number(body)?),
            },
            ("bool", None) => ScalarType::Bool,
            ("date", None) => ScalarType::Date,
            ("time", None) => ScalarType::Time,
            ("datetime", None) => ScalarType::DateTime,
            ("instant", None) => ScalarType::Instant,
            ("uuid", None) => ScalarType::Uuid,
            ("json", None) => ScalarType::Json,
            _ => return Err(unknown(text)),
        };
        ty.check()?;
        Ok(ty)
    }
}

impl ColumnDefault {
    /// A default as one key mapped to one value.
    ///
    /// [`ColumnDefault::Now`] carries `true` rather than nothing, so that every
    /// default in the form has the same shape and a reader never has to tell a
    /// key with no value from a missing one.
    ///
    /// [`ColumnDefault::Opaque`] is shown under a key [`ColumnDefault::from_node`]
    /// does not accept. A dump of a schema read off a server says what that
    /// server has, including the defaults this vocabulary cannot say; feeding
    /// the dump back in is where asking for one is refused.
    fn to_node(&self) -> Node {
        let (key, value) = match self {
            ColumnDefault::Int(value) => ("int", Node::Int(*value)),
            ColumnDefault::Uint(value) => ("uint", Node::Uint(*value)),
            ColumnDefault::Float(value) => ("float", Node::Float(*value)),
            ColumnDefault::Decimal(digits) => ("decimal", Node::Text(digits.clone())),
            ColumnDefault::Text(text) => ("text", Node::Text(text.clone())),
            ColumnDefault::Bool(value) => ("bool", Node::Bool(*value)),
            ColumnDefault::Now => ("now", Node::Bool(true)),
            ColumnDefault::Opaque(text) => ("opaque", Node::Text(text.clone())),
        };
        Node::Map(vec![pair(key, value)])
    }

    fn from_node(node: &Node) -> Result<ColumnDefault, SchemaError> {
        let bad = SchemaError::BadArray {
            key: "default",
            wanted: "one of `int`, `uint`, `float`, `decimal`, `text`, `bool` or `now`",
        };
        let Node::Map(pairs) = node else {
            return Err(bad);
        };
        let [(key, value)] = &pairs[..] else {
            return Err(bad);
        };
        Ok(match (key.as_str(), value) {
            ("int", Node::Int(value)) => ColumnDefault::Int(*value),
            ("uint", Node::Uint(value)) => ColumnDefault::Uint(*value),
            ("float", Node::Float(value)) => ColumnDefault::Float(*value),
            ("decimal", Node::Text(digits)) => ColumnDefault::Decimal(digits.clone()),
            ("text", Node::Text(text)) => ColumnDefault::Text(text.clone()),
            ("bool", Node::Bool(value)) => ColumnDefault::Bool(*value),
            ("now", Node::Bool(true)) => ColumnDefault::Now,
            // `opaque` is deliberately absent from the keys above, and from the
            // wanted list the refusal names. It is what a server said, never
            // what a program may ask for, and a default a program could ask for
            // in a server's own words is the expression hole § 2 closes.
            _ => return Err(bad),
        })
    }
}

impl Column {
    /// The column as a node: its name, its type's spelling, both flags, and a
    /// default only when it has one.
    fn to_node(&self) -> Node {
        let mut pairs = vec![
            pair("name", name_node(&self.name)),
            pair("type", Node::Text(self.ty.to_string())),
            pair("null", Node::Bool(self.nullable)),
            pair("identity", Node::Bool(self.identity)),
        ];
        if let Some(default) = &self.default {
            pairs.push(pair("default", default.to_node()));
        }
        Node::Map(pairs)
    }

    fn from_node(node: &Node) -> Result<Column, SchemaError> {
        let mut column = Column::new(
            node.text("name")?,
            ScalarType::from_spelling(node.text("type")?)?,
        )?;
        if node.flag("null")? {
            column = column.null();
        }
        if node.flag("identity")? {
            column = column.identity()?;
        }
        if let Some(default) = node.get("default") {
            column = column.default(ColumnDefault::from_node(default)?)?;
        }
        Ok(column)
    }
}

impl Key {
    fn to_node(&self) -> Node {
        Node::Map(vec![
            pair("name", name_node(&self.name)),
            pair("columns", name_list(&self.columns)),
        ])
    }
}

impl Table {
    /// The table as a node.
    ///
    /// Columns keep **declaration order**, because a `CREATE TABLE` has to
    /// reproduce it; the primary key and every key's own columns keep the order
    /// they were declared in, because an index is not the same index under a
    /// different column order. Constraints and indexes are emitted in **name
    /// order**, since nothing observable depends on the order they were added
    /// in and an introspector answering the server's catalog order would
    /// otherwise fail the round trip against a database that is not wrong in
    /// any way.
    fn to_node(&self) -> Node {
        let sorted = |keys: &[Key]| {
            let mut keys: Vec<&Key> = keys.iter().collect();
            keys.sort_by(|left, right| left.name.as_str().cmp(right.name.as_str()));
            Node::List(keys.into_iter().map(Key::to_node).collect())
        };
        Node::Map(vec![
            pair("name", name_node(&self.name)),
            pair(
                "columns",
                Node::List(self.columns.iter().map(Column::to_node).collect()),
            ),
            pair("primary_key", name_list(&self.primary_key)),
            pair("unique", sorted(&self.unique)),
            pair("indexes", sorted(&self.indexes)),
        ])
    }

    fn from_node(node: &Node) -> Result<Table, SchemaError> {
        let columns = node
            .list("columns")?
            .iter()
            .map(Column::from_node)
            .collect::<Result<Vec<Column>, SchemaError>>()?;
        let mut table = Table::new(node.text("name")?, columns)?;
        let key = node.names("primary_key")?;
        if !key.is_empty() {
            table = table.primary_key(&key)?;
        }
        for unique in node.list("unique")? {
            table = table.unique(unique.text("name")?, &unique.names("columns")?)?;
        }
        for index in node.list("indexes")? {
            table = table.index(index.text("name")?, &index.names("columns")?)?;
        }
        Ok(table)
    }
}

impl Schema {
    /// This schema in the canonical array form, with its tables in name order.
    ///
    /// `Schema::from_array(&a.to_array())` is `a` for every schema, and
    /// `a.to_array()` is what two schemas are compared by — the diff normalizes
    /// into this and nothing compares SQL text.
    #[must_use]
    pub fn to_array(&self) -> Node {
        let mut tables: Vec<&Table> = self.tables.iter().collect();
        tables.sort_by(|left, right| left.name.as_str().cmp(right.name.as_str()));
        Node::Map(vec![pair(
            "tables",
            Node::List(tables.into_iter().map(Table::to_node).collect()),
        )])
    }

    /// The schema an array form describes.
    ///
    /// Every construction rule is the builders' — this reads the form and hands
    /// it to them, so a file cannot say anything a program could not have built.
    ///
    /// # Errors
    ///
    /// [`SchemaError::BadArray`] for a key that is missing or holds the wrong
    /// kind of value, [`SchemaError::UnknownType`] for a type spelling outside
    /// the vocabulary, and every refusal the builders state for a schema that
    /// reads but is not coherent.
    pub fn from_array(node: &Node) -> Result<Schema, SchemaError> {
        let tables = node
            .list("tables")?
            .iter()
            .map(Table::from_node)
            .collect::<Result<Vec<Table>, SchemaError>>()?;
        Schema::new(tables)
    }
}

/// Why a construction was refused.
///
/// Every one of these is a schema no backend would accept, or one two backends
/// would accept differently. This crate holds no Novis-facing name — `rule:core-classes/db-crate-boundary` — so these carry the facts and `nvs-stdlib` writes the fault that names
/// the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// Not a letter or `_` followed by letters, digits or `_`.
    NotAnIdentifier(String),
    /// Longer than [`MAX_IDENTIFIER`], which PostgreSQL would silently truncate.
    IdentifierTooLong(String),
    /// A table with no columns.
    NoColumns(Ident),
    /// A name used twice where one is required.
    Duplicate {
        /// What kind of thing was declared twice.
        what: &'static str,
        /// The name that repeats.
        name: Ident,
    },
    /// A key over no columns at all.
    EmptyKey(Ident),
    /// A key naming a column its table has not got.
    UnknownColumn {
        /// The table the key belongs to.
        table: Ident,
        /// The name that resolved to nothing.
        column: Ident,
    },
    /// A nullable column in a primary key.
    NullableInKey(Ident),
    /// `DECIMAL(p, s)` outside `1 <= p <= 38, s <= p`.
    BadPrecision {
        /// The precision asked for.
        precision: u8,
        /// The scale asked for.
        scale: u8,
    },
    /// A `VARCHAR(0)` or `VARBINARY(0)`.
    ZeroWidth,
    /// An identity column that is not an integer.
    IdentityNotInteger(Ident),
    /// A second identity column in one table.
    TwoIdentities(Ident),
    /// An identity column outside its table's primary key.
    IdentityNotInKey(Ident),
    /// A default no backend takes on that type.
    DefaultDoesNotFit(Ident),
    /// A type spelling the closed vocabulary does not contain.
    UnknownType(String),
    /// A node of the array form that is missing, or is not the kind of value
    /// that place takes.
    BadArray {
        /// The key the reading stopped at.
        key: &'static str,
        /// What that key takes.
        wanted: &'static str,
    },
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::NotAnIdentifier(name) => write!(
                f,
                "`{name}` is not a bare identifier — a letter or `_`, then letters, digits or `_`"
            ),
            SchemaError::IdentifierTooLong(name) => write!(
                f,
                "`{name}` is longer than {MAX_IDENTIFIER} bytes, which PostgreSQL truncates silently"
            ),
            SchemaError::NoColumns(table) => write!(f, "table `{table}` declares no columns"),
            SchemaError::Duplicate { what, name } => {
                write!(f, "`{name}` is declared twice as a {what}")
            }
            SchemaError::EmptyKey(table) => {
                write!(f, "a key on table `{table}` names no columns")
            }
            SchemaError::UnknownColumn { table, column } => {
                write!(f, "table `{table}` has no column `{column}`")
            }
            SchemaError::NullableInKey(column) => write!(
                f,
                "column `{column}` is nullable and no backend admits null into a primary key"
            ),
            SchemaError::BadPrecision { precision, scale } => write!(
                f,
                "`decimal({precision}, {scale})` is outside the portable range: 1 to 38 digits, \
                 with a scale no greater than the precision"
            ),
            SchemaError::ZeroWidth => f.write_str("a declared length of zero is not a column"),
            SchemaError::IdentityNotInteger(column) => {
                write!(
                    f,
                    "column `{column}` is not an integer, so it cannot be an identity"
                )
            }
            SchemaError::TwoIdentities(table) => {
                write!(f, "table `{table}` declares two identity columns")
            }
            SchemaError::IdentityNotInKey(column) => write!(
                f,
                "identity column `{column}` is not in its table's primary key, which SQLite \
                 requires and the other four assume"
            ),
            SchemaError::DefaultDoesNotFit(column) => write!(
                f,
                "column `{column}` cannot carry that default on all five backends"
            ),
            SchemaError::UnknownType(spelling) => write!(
                f,
                "`{spelling}` is not one of the column types the vocabulary contains"
            ),
            SchemaError::BadArray { key, wanted } => {
                write!(f, "the array form's `{key}` wants {wanted}")
            }
        }
    }
}

impl std::error::Error for SchemaError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:core-classes/schema-vocabulary-is-closed`'s "one table used twice": the write direction's every case
    /// describes as a read case of its own, and never as the total one.
    ///
    /// All three properties at once, because they are one claim — total,
    /// injective, and `Other`-free. A fourteenth write case sharing a read case
    /// with another would be a column the introspector could not tell apart
    /// from the type it was written as, which is § 5's empty-plan property
    /// failing at the first `dump`.
    #[test]
    fn the_column_type_vocabulary_is_adr_0067_section_9s_map_in_the_write_direction() {
        let written = [
            ScalarType::Int(IntWidth::Small),
            ScalarType::Uint(IntWidth::Big),
            ScalarType::Float(FloatWidth::Double),
            ScalarType::Decimal {
                precision: 10,
                scale: 2,
            },
            ScalarType::Text { max: Some(40) },
            ScalarType::Bytes { max: None },
            ScalarType::Bool,
            ScalarType::Date,
            ScalarType::Time,
            ScalarType::DateTime,
            ScalarType::Instant,
            ScalarType::Uuid,
            ScalarType::Json,
        ];
        let mut seen: Vec<ColumnType> = Vec::new();
        for ty in &written {
            let case = ty.describes();
            assert_ne!(
                case,
                ColumnType::Other,
                "{ty:?} describes as the total case"
            );
            assert!(
                !seen.contains(&case),
                "{ty:?} shares a read case with an earlier write type"
            );
            seen.push(case);
        }
        // Thirteen of `ColumnType`'s fourteen: `Other` is the one a schema
        // cannot ask for, which is what "closed vocabulary" means.
        assert_eq!(seen.len(), 13);
    }

    /// The identifier rule is `Core\Db::quoteIdentifier`'s, and this crate is
    /// its one home.
    ///
    /// Asserted as an **agreement** and not only as a list of answers: an
    /// [`Ident`] is exactly what [`is_bare_identifier`] admits, within
    /// [`MAX_IDENTIFIER`], because that function is the judgement the member
    /// one crate up calls rather than a second one written to match.
    #[test]
    fn an_identifier_is_validated_by_the_judgement_quote_identifier_states() {
        for good in ["id", "_x", "nvs_jobs", "A1"] {
            assert!(Ident::new(good).is_ok(), "{good} is a bare identifier");
        }
        let at_the_limit = "a".repeat(MAX_IDENTIFIER);
        for name in [
            "id",
            "_x",
            "A1",
            "",
            "1st",
            "a-b",
            "a b",
            "naïve",
            &at_the_limit,
        ] {
            assert_eq!(
                Ident::new(name).is_ok(),
                is_bare_identifier(name) && name.len() <= MAX_IDENTIFIER,
                "`{name}` is judged one way by `Ident::new` and another by \
                 the judgement `quoteIdentifier` states"
            );
        }
        for bad in ["", "1st", "a-b", "a b", "a\"b", "naïve", "a;drop"] {
            assert!(matches!(
                Ident::new(bad),
                Err(SchemaError::NotAnIdentifier(_))
            ));
        }
        let long = "a".repeat(MAX_IDENTIFIER + 1);
        assert!(matches!(
            Ident::new(&long),
            Err(SchemaError::IdentifierTooLong(_))
        ));
        // Case is not a distinction, because the backends disagree about
        // folding and a schema that means two things is not portable.
        assert_eq!(Ident::new("Id").unwrap(), Ident::new("id").unwrap());
    }

    /// A key is checked against the table it is on, so an emitter never has to.
    #[test]
    fn a_key_naming_a_column_the_table_lacks_is_refused() {
        let table = Table::new(
            "t",
            vec![Column::new("id", ScalarType::Int(IntWidth::Big)).unwrap()],
        )
        .unwrap();
        assert!(matches!(
            table.clone().index("t_by_name", &["name"]),
            Err(SchemaError::UnknownColumn { .. })
        ));
        assert!(matches!(
            table.clone().primary_key(&[]),
            Err(SchemaError::EmptyKey(_))
        ));
        assert!(matches!(
            table.primary_key(&["id", "id"]),
            Err(SchemaError::Duplicate { .. })
        ));
    }

    /// Two rules no backend disagrees about, refused before the server is asked.
    #[test]
    fn a_nullable_primary_key_and_a_duplicate_column_are_refused() {
        let nullable = Table::new(
            "t",
            vec![
                Column::new("id", ScalarType::Int(IntWidth::Big))
                    .unwrap()
                    .null(),
            ],
        )
        .unwrap()
        .primary_key(&["id"]);
        assert!(matches!(nullable, Err(SchemaError::NullableInKey(_))));

        let twice = Table::new(
            "t",
            vec![
                Column::new("id", ScalarType::Int(IntWidth::Big)).unwrap(),
                Column::new("ID", ScalarType::Int(IntWidth::Big)).unwrap(),
            ],
        );
        assert!(matches!(twice, Err(SchemaError::Duplicate { .. })));
    }

    /// § 2's closed default set, and the refusals that are portability rather
    /// than typing.
    ///
    /// The last assertion is the "and nothing else": an expression default is
    /// not a thing the canonical form can *say*, so the DDL-injection hole and
    /// the normalization hole § 5 spends its budget on are both closed by the
    /// vocabulary rather than by a check someone has to remember to run.
    #[test]
    fn a_default_is_a_vocabulary_scalar_or_now_and_nothing_else() {
        let varchar = Column::new("name", ScalarType::Text { max: Some(40) }).unwrap();
        assert!(
            varchar
                .clone()
                .default(ColumnDefault::Text("x".into()))
                .is_ok()
        );

        let text = Column::new("body", ScalarType::Text { max: None }).unwrap();
        assert!(matches!(
            text.default(ColumnDefault::Text("x".into())),
            Err(SchemaError::DefaultDoesNotFit(_))
        ));
        let blob = Column::new("raw", ScalarType::Bytes { max: Some(16) }).unwrap();
        assert!(matches!(
            blob.default(ColumnDefault::Text("x".into())),
            Err(SchemaError::DefaultDoesNotFit(_))
        ));
        // `CURRENT_TIMESTAMP` is a default on the two temporal types that carry
        // one everywhere, and on nothing else.
        assert!(
            Column::new("at", ScalarType::Instant)
                .unwrap()
                .default(ColumnDefault::Now)
                .is_ok()
        );
        assert!(matches!(
            Column::new("d", ScalarType::Date)
                .unwrap()
                .default(ColumnDefault::Now),
            Err(SchemaError::DefaultDoesNotFit(_))
        ));
        // A literal of the wrong family is the ordinary typing refusal.
        assert!(matches!(
            varchar.default(ColumnDefault::Int(1)),
            Err(SchemaError::DefaultDoesNotFit(_))
        ));
        // And the set is closed at the form as well as at the type: there is no
        // key an expression could arrive under.
        assert!(matches!(
            ColumnDefault::from_node(&Node::Map(vec![(
                "expression".to_string(),
                Node::Text("now()".to_string()),
            )])),
            Err(SchemaError::BadArray { .. })
        ));
    }

    /// `rule:core-classes/schema-is-a-value`: the read-only case is one-way — a
    /// dump shows it, and the canonical form has no key that builds one.
    ///
    /// The column it is attached to is an integer here on purpose: an opaque
    /// default is whatever the server has on whatever column, so the fit that
    /// every other case is checked for is not a question this one answers.
    #[test]
    fn an_opaque_default_is_dumped_and_never_read_back() {
        let default = ColumnDefault::Opaque("nextval('s'::regclass)".to_owned());
        let node = default.to_node();
        assert_eq!(
            node,
            Node::Map(vec![pair(
                "opaque",
                Node::Text("nextval('s'::regclass)".to_owned())
            )])
        );
        assert!(matches!(
            ColumnDefault::from_node(&node),
            Err(SchemaError::BadArray { .. })
        ));
        assert!(
            Column::new("id", ScalarType::Int(IntWidth::Big))
                .unwrap()
                .default(default)
                .is_ok()
        );
    }

    /// An identity column is an integer, there is at most one, and it is the
    /// primary key — the rules every spelling of the construct shares.
    #[test]
    fn an_identity_column_is_an_integer_and_the_primary_key() {
        assert!(matches!(
            Column::new("name", ScalarType::Text { max: Some(8) })
                .unwrap()
                .identity(),
            Err(SchemaError::IdentityNotInteger(_))
        ));

        let id = || {
            Column::new("id", ScalarType::Int(IntWidth::Big))
                .unwrap()
                .identity()
                .unwrap()
        };
        let unkeyed = Table::new("t", vec![id()]).unwrap();
        assert!(matches!(
            Schema::new(vec![unkeyed]),
            Err(SchemaError::IdentityNotInKey(_))
        ));

        let two = Table::new(
            "t",
            vec![id(), {
                Column::new("other", ScalarType::Int(IntWidth::Big))
                    .unwrap()
                    .identity()
                    .unwrap()
            }],
        );
        assert!(matches!(two, Err(SchemaError::TwoIdentities(_))));

        let keyed = Table::new("t", vec![id()])
            .unwrap()
            .primary_key(&["id"])
            .unwrap();
        assert!(Schema::new(vec![keyed]).is_ok());
    }

    /// A key name is unique across the schema, not across its table.
    #[test]
    fn one_key_name_serves_the_whole_schema() {
        let table = |name: &str| {
            Table::new(
                name,
                vec![Column::new("queue", ScalarType::Text { max: Some(64) }).unwrap()],
            )
            .unwrap()
            .index("by_queue", &["queue"])
            .unwrap()
        };
        assert!(matches!(
            Schema::new(vec![table("a"), table("b")]),
            Err(SchemaError::Duplicate { what: "key", .. })
        ));
    }

    /// `rule:core-classes/queue-storage-is-a-table`'s jobs table, said in the vocabulary that will replace its
    /// hand-written DDL lists.
    ///
    /// The one construct it needs and this vocabulary has not got is
    /// PostgreSQL's partial unique index on `dedupe_key` — § 11 keeps partial
    /// indexes out of v1 for want of a portable spelling, and the retirement
    /// owes that index an answer the vocabulary can hold.
    #[test]
    fn the_queue_tables_are_sayable_in_the_vocabulary() {
        let bigint = || ScalarType::Int(IntWidth::Big);
        let text = || ScalarType::Text { max: None };
        let jobs = Table::new(
            "nvs_jobs",
            vec![
                Column::new("id", bigint()).unwrap().identity().unwrap(),
                Column::new("queue", ScalarType::Text { max: Some(64) }).unwrap(),
                Column::new("script", text()).unwrap(),
                Column::new("args", text()).unwrap().null(),
                Column::new("state", ScalarType::Int(IntWidth::Small)).unwrap(),
                Column::new("attempts", ScalarType::Int(IntWidth::Normal)).unwrap(),
                Column::new("max_attempts", ScalarType::Int(IntWidth::Normal)).unwrap(),
                Column::new("backoff_ms", bigint()).unwrap(),
                Column::new("run_at", bigint()).unwrap(),
                Column::new("dedupe_key", ScalarType::Text { max: Some(255) })
                    .unwrap()
                    .null(),
                Column::new("created_at", bigint()).unwrap(),
                Column::new("claimed_at", bigint()).unwrap().null(),
            ],
        )
        .unwrap()
        .primary_key(&["id"])
        .unwrap()
        .index("nvs_jobs_due", &["queue", "state", "run_at"])
        .unwrap();

        let schema = Schema::new(vec![jobs]).unwrap();
        let jobs = schema.table(&Ident::new("nvs_jobs").unwrap()).unwrap();
        assert_eq!(jobs.columns().len(), 12);
        assert!(jobs.columns()[0].is_identity());
        assert_eq!(jobs.primary_key_columns(), [Ident::new("id").unwrap()]);
        assert_eq!(jobs.indexes().len(), 1);
        assert!(jobs.unique_keys().is_empty());
    }

    /// The names of a list of named nodes, in the order the form states them.
    fn names_in(node: &Node, key: &str) -> Vec<String> {
        let Some(Node::List(items)) = node.get(key) else {
            panic!("`{key}` is not a list");
        };
        items
            .iter()
            .map(|item| match item.get("name") {
                Some(Node::Text(name)) => name.clone(),
                _ => panic!("an item of `{key}` has no name"),
            })
            .collect()
    }

    /// `rule:core-classes/schema-is-a-value`'s property, over one schema using every construct the
    /// vocabulary has: `to_array(from_array(a)) == a`.
    ///
    /// Asserted with the *order* rule and not only the structure, because the
    /// form is canonical and an introspector answering a server's own catalog
    /// order would otherwise round-trip into a different value and make § 5's
    /// plan report a change against a database that is not wrong in any way.
    /// So everything below is declared in the order the canonical form does not
    /// use: the tables backwards by name, the keys backwards by name, and the
    /// columns in the one order that *is* kept.
    #[test]
    fn a_schema_value_round_trips_through_its_array_form() {
        let notes = Table::new(
            "notes",
            vec![
                Column::new("id", ScalarType::Int(IntWidth::Big))
                    .unwrap()
                    .identity()
                    .unwrap(),
                Column::new("title", ScalarType::Text { max: Some(200) })
                    .unwrap()
                    .default(ColumnDefault::Text("untitled".to_string()))
                    .unwrap(),
                Column::new("body", ScalarType::Text { max: None })
                    .unwrap()
                    .null(),
                Column::new(
                    "rating",
                    ScalarType::Decimal {
                        precision: 4,
                        scale: 2,
                    },
                )
                .unwrap()
                .null(),
                Column::new("created_at", ScalarType::Instant)
                    .unwrap()
                    .default(ColumnDefault::Now)
                    .unwrap(),
            ],
        )
        .unwrap()
        .primary_key(&["id"])
        .unwrap()
        .unique("notes_title", &["title"])
        .unwrap()
        .index("notes_recent", &["created_at", "title"])
        .unwrap()
        .index("notes_by_body", &["body"])
        .unwrap();
        let authors = Table::new(
            "authors",
            vec![
                Column::new("id", ScalarType::Uint(IntWidth::Normal)).unwrap(),
                Column::new("active", ScalarType::Bool)
                    .unwrap()
                    .default(ColumnDefault::Bool(true))
                    .unwrap(),
            ],
        )
        .unwrap()
        .primary_key(&["id"])
        .unwrap();

        let schema = Schema::new(vec![notes, authors]).unwrap();
        let array = schema.to_array();

        // Name order for tables and for indexes; declaration order for columns,
        // and for the columns *inside* one index, where a different order is a
        // different index.
        assert_eq!(names_in(&array, "tables"), ["authors", "notes"]);
        let Some(Node::List(tables)) = array.get("tables") else {
            unreachable!()
        };
        assert_eq!(
            names_in(&tables[1], "columns"),
            ["id", "title", "body", "rating", "created_at"]
        );
        assert_eq!(
            names_in(&tables[1], "indexes"),
            ["notes_by_body", "notes_recent"]
        );
        assert_eq!(
            tables[1].get("indexes").unwrap(),
            &Node::List(vec![
                Node::Map(vec![
                    ("name".to_string(), Node::Text("notes_by_body".to_string())),
                    (
                        "columns".to_string(),
                        Node::List(vec![Node::Text("body".to_string())])
                    ),
                ]),
                Node::Map(vec![
                    ("name".to_string(), Node::Text("notes_recent".to_string())),
                    (
                        "columns".to_string(),
                        Node::List(vec![
                            Node::Text("created_at".to_string()),
                            Node::Text("title".to_string()),
                        ])
                    ),
                ]),
            ])
        );

        // The property itself. Read back through the builders, and written out
        // again to the byte-identical form -- which is the comparison § 5's
        // diff normalizes into.
        let read = Schema::from_array(&array).unwrap();
        assert_eq!(read.to_array(), array);
        assert_eq!(read.tables().len(), 2);
    }

    /// Every case of the vocabulary has a canonical spelling, they are all
    /// distinct, and each one reads back as itself.
    ///
    /// A count, not a row-by-row reading: a fourteenth type added with no
    /// spelling, or a parameter dropped on the way through, fails here rather
    /// than in an emitter that then writes a column the introspector cannot
    /// answer with.
    #[test]
    fn every_column_type_in_the_vocabulary_round_trips() {
        let every = [
            ScalarType::Int(IntWidth::Small),
            ScalarType::Int(IntWidth::Normal),
            ScalarType::Int(IntWidth::Big),
            ScalarType::Uint(IntWidth::Small),
            ScalarType::Uint(IntWidth::Normal),
            ScalarType::Uint(IntWidth::Big),
            ScalarType::Float(FloatWidth::Single),
            ScalarType::Float(FloatWidth::Double),
            ScalarType::Decimal {
                precision: 38,
                scale: 6,
            },
            ScalarType::Text { max: Some(255) },
            ScalarType::Text { max: None },
            ScalarType::Bytes { max: Some(16) },
            ScalarType::Bytes { max: None },
            ScalarType::Bool,
            ScalarType::Date,
            ScalarType::Time,
            ScalarType::DateTime,
            ScalarType::Instant,
            ScalarType::Uuid,
            ScalarType::Json,
        ];

        let mut spellings: Vec<String> = Vec::new();
        for ty in &every {
            let spelling = ty.to_string();
            assert!(
                !spellings.contains(&spelling),
                "`{spelling}` spells two types"
            );
            assert_eq!(
                &ScalarType::from_spelling(&spelling).unwrap(),
                ty,
                "`{spelling}` did not read back as {ty:?}"
            );
            spellings.push(spelling);
        }
        // Thirteen read cases, twenty write cases: the seven extra are the
        // widths and the two lengths, which is exactly what the write direction
        // keeps and the read direction does not.
        assert_eq!(spellings.len(), 20);
        let mut read_cases: Vec<ColumnType> = Vec::new();
        for case in every.iter().map(ScalarType::describes) {
            if !read_cases.contains(&case) {
                read_cases.push(case);
            }
        }
        assert_eq!(read_cases.len(), 13);

        // And through a whole schema, where the spelling is the only place a
        // type is written down.
        let columns = every
            .iter()
            .enumerate()
            .map(|(at, ty)| Column::new(&format!("c{at}"), ty.clone()).unwrap())
            .collect();
        let schema = Schema::new(vec![Table::new("every", columns).unwrap()]).unwrap();
        let array = schema.to_array();
        assert_eq!(Schema::from_array(&array).unwrap().to_array(), array);

        // A spelling outside the closed set is refused, and so is one inside it
        // carrying a parameter no backend accepts. There is no fallback and no
        // pass-through: that is what makes the vocabulary closed.
        for outside in ["smallint", "int24", "varchar(20)", "text(", "decimal(10)"] {
            assert!(
                matches!(
                    ScalarType::from_spelling(outside),
                    Err(SchemaError::UnknownType(_))
                ),
                "`{outside}` is not a spelling this vocabulary has"
            );
        }
        assert_eq!(
            ScalarType::from_spelling("text(0)"),
            Err(SchemaError::ZeroWidth)
        );
        assert!(matches!(
            ScalarType::from_spelling("decimal(39,2)"),
            Err(SchemaError::BadPrecision { .. })
        ));
    }
}
