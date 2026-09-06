//! [ADR 0145](/docs/adr/0145-a-schema-is-a-value-core-db-schema-converges-a-closed.md)'s
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
//! # The write direction of ADR 0067 § 9
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
//! ([ADR 0024](/docs/adr/0024-taint-tracking-for-injection-sinks.md); DDL has no
//! parameters to bind a name through), and
//! [`is_bare_identifier`] is that judgement's one home — `Core\Db::quoteIdentifier`
//! calls it rather than restating it. Everything else a construction can get
//! wrong is refused here too, so that a [`Schema`] value in hand is one the
//! emitters may assume is coherent: a key naming a column the table has not got,
//! a nullable primary key, a default no backend will accept. An emitter that
//! re-checked would be a second answer to the same question.
//!
//! # Known gaps
//!
//! 1. **No canonical array form yet.** § 1 makes the array form canonical and
//!    `toArray(fromArray(a)) == a` the property that holds the three spellings
//!    together; these types carry no serialization at all, and the ordering rule
//!    (declaration order for columns, name order for everything else) is stated
//!    in that section and enforced nowhere.
//! 2. **No emitters and no introspectors.** §§ 4 and 8 are the next two, and
//!    both are `Dialect`-keyed and sans-io like everything else here.
//! 3. **§ 11's exclusions are not represented and must not be added casually.**
//!    Foreign keys, partial and expression indexes, index types, collations,
//!    check constraints and the rest are out of v1 because they have no portable
//!    spelling, and the vocabulary grows only when a construct exists on all
//!    five backends *and* something needs it.

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
/// Comparison is case-**insensitive**, because the five backends disagree about
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

/// How wide an integer column is, since the three widths are not one type on
/// any backend and a narrowing is a table rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntWidth {
    /// `SMALLINT`, 16 bits.
    Small,
    /// `INTEGER`, 32 bits.
    Normal,
    /// `BIGINT`, 64 bits.
    Big,
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

/// A column's type: [ADR 0067 § 9](/docs/adr/0067-core-db.md)'s map in the
/// write direction, with the parameters a `CREATE TABLE` must carry.
///
/// Where § 9's read direction is many-to-one — `SMALLINT`, `INTEGER` and
/// `BIGINT` all read as `int` — this keeps the distinction, because the writer
/// chose it and the introspector can recover it. [`ScalarType::describes`] is
/// the arrow back onto the read direction's own enum, and the two cannot drift
/// while that function is total.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarType {
    /// A signed integer of one of the three portable widths.
    Int(IntWidth),
    /// An unsigned integer. MySQL and MariaDB spell it `UNSIGNED`; the other
    /// three have no unsigned integer and take the next width up with a check
    /// the emitter writes.
    Uint(IntWidth),
    /// Binary floating point.
    Float(FloatWidth),
    /// `DECIMAL(p, s)` — the exact type [ADR 0054](/docs/adr/0054-decimal-scalar-type.md)
    /// makes a Novis `decimal`.
    Decimal {
        /// Total significant digits, 1 to 38 — SQL Server's ceiling, which is
        /// the lowest of the five.
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
    /// them by construction. `a_written_type_describes_as_its_own_read_case`
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

    /// Whether this is one of the three integer types, which is what an
    /// identity column may be.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(self, ScalarType::Int(_) | ScalarType::Uint(_))
    }

    fn check(&self) -> Result<(), SchemaError> {
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

/// § 2's closed set of defaults: a literal of a vocabulary scalar type, or the
/// current timestamp.
///
/// There are no expression defaults, and that one rule closes two holes at
/// once. An expression is an unbindable string reaching a DDL sink, and it is
/// also the value a server is most likely to spell back differently — which is
/// the normalization difficulty § 5 exists to survive.
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
    /// spelling all five share.
    Now,
}

impl ColumnDefault {
    /// Whether this literal may be written as `ty`'s default on all five
    /// backends.
    ///
    /// Two refusals here are portability rather than typing, and both are
    /// MySQL's: a `BLOB`, a `TEXT` and a `JSON` column take no literal default
    /// at all before 8.0.13 and only a parenthesized expression after, and an
    /// expression is not in this set. So an unbounded [`ScalarType::Text`],
    /// every [`ScalarType::Bytes`], [`ScalarType::Json`] and
    /// [`ScalarType::Uuid`] carry no default, and a `VARCHAR(n)` carries one
    /// happily.
    fn fits(&self, ty: &ScalarType) -> bool {
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
    /// takes on all five backends — [`ColumnDefault::fits`] says which pairs
    /// those are and why two of them are portability rather than typing.
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

/// Why a construction was refused.
///
/// Every one of these is a schema no backend would accept, or one two backends
/// would accept differently. This crate holds no Novis-facing name — ADR 0132
/// § 1 — so these carry the facts and `nvs-stdlib` writes the fault that names
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
        }
    }
}

impl std::error::Error for SchemaError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0145 § 2's "one table used twice": the write direction's every case
    /// describes as a read case of its own, and never as the total one.
    ///
    /// All three properties at once, because they are one claim — total,
    /// injective, and `Other`-free. A fourteenth write case sharing a read case
    /// with another would be a column the introspector could not tell apart
    /// from the type it was written as, which is § 5's empty-plan property
    /// failing at the first `dump`.
    #[test]
    fn a_written_type_describes_as_its_own_read_case() {
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
    #[test]
    fn an_identifier_is_ascii_bare_and_no_longer_than_postgresqls_floor() {
        for good in ["id", "_x", "nvs_jobs", "A1"] {
            assert!(Ident::new(good).is_ok(), "{good} is a bare identifier");
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
        // Case is not a distinction, because the five backends disagree about
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

    /// § 2's closed default set, and the two refusals that are portability
    /// rather than typing.
    #[test]
    fn a_default_the_five_backends_do_not_share_is_refused() {
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
    }

    /// An identity column is an integer, there is at most one, and it is the
    /// primary key — the three rules the four spellings of the construct share.
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

    /// ADR 0084 § 2's jobs table, said in the vocabulary that will replace its
    /// four hand-written DDL lists.
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
}
