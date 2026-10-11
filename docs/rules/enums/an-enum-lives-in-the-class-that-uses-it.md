An enum exactly one class takes, returns or holds is declared in that class; an enum several classes
use, or none, is declared in their namespace. The rule binds every `Core` enum, every built-in
component's enum and every published program, and it is the default a program's own enums follow.

The test is mechanical, so two readers place one enum the same way. A class *uses* an enum when one
of its methods takes it as a parameter or returns it, or one of its properties or constants has its
type. An enum used by one class moves into it. An enum used by two or more, or only by an attribute
(`Core\Audience`, used only by `#[Access]`), stays at namespace scope. When a later member makes an
owned enum shared, the enum moves back to its namespace in the slice that adds the member.
`crates/nvs-stdlib/tests/enum_owners.rs` holds the rule for `Core`, and a reviewer's judgement does
not override it.

**A rename drops the owner's word** where the old name repeated it: `Core\Xml\NodeKind` becomes
`Core\Xml\Node::Kind`, never `Core\Xml\Node::NodeKind`. A name that does not parse as a member, because
it clashes with a keyword, keeps its old word and drops the owner's prefix the other way; the table
below says which, and today no entry needs it.

The `Core` enums that move:

| Was | Is |
|---|---|
| `Core\Order` | `Core\Arr::Order` |
| `Core\SetOn` | `Core\Arr::SetOn` |
| `Core\NormalForm` | `Core\Str::NormalForm` |
| `Core\Digest` | `Core\Hash::Digest` |
| `Core\Codec` | `Core\Compress::Codec` |
| `Core\Weekday` | `Core\Time\DateTime::Weekday` |
| `Core\Crypto\Cipher` | `Core\Crypto::Cipher` |
| `Core\Crypto\KeyFormat` | `Core\Crypto\PublicKey::Format` |
| `Core\Mime\Type` | `Core\Mime::Type` |
| `Core\Xml\NodeKind` | `Core\Xml\Node::Kind` |
| `Core\Log\Level` | `Core\Log::Level` |
| `Core\Env\Mode` | `Core\Env::Mode` |
| `Core\Response\Redirect` | `Core\Response::Redirect` |
| `Core\Response\SameSite` | `Core\Response::SameSite` |
| `Core\Cli\Stream` | `Core\Cli::Stream` |
| `Core\Cli\ColorDepth` | `Core\Cli::ColorDepth` |
| `Core\Cli\Shell` | `Core\Command::Shell` |
| `Core\Reflect\TypeKind` | `Core\Reflect::TypeKind` |
| `Core\IO\FileMode` | `Core\IO::FileMode` |
| `Core\Cldr\PluralCategory` | `Core\Cldr::PluralCategory` |
| `Core\Script\ExitReason` | `Core\Script\ExitReport::Reason` |
| `Core\Db\Tls` | `Core\Db::Tls` |
| `Core\Db\ErrorKind` | `Core\Db\DbError::Kind` |
| `Core\Db\ColumnType` | `Core\Db\Column::Type` |
| `Core\Db\Plan\Grade` | `Core\Db\Plan\Step::Grade` |
| `Core\Ldap\Scope` | `Core\Ldap\Connection::Scope` |
| `Core\Ldap\Tls` | `Core\Ldap::Tls` |
| `Core\Ldap\ErrorKind` | `Core\Ldap\LdapError::Kind` |
| `Core\Ldap\Ad\AccountType` | `Core\Ldap\Entry::AccountType` |
| `Core\Queue\State` | `Core\Queue::State` |

The `Core` enums that stay, each with the classes that share it: `Core\RoundMode` (`Math`,
`Decimal`), `Core\Charset` (`Encoding`, `IO`), `Core\Unit` (`DateTime`, `Date`, `TimeOfDay`),
`Core\Crypto\KeyKind` (four classes), `Core\Http\Method` (seven classes and `#[Route]`),
`Core\Db\Driver` (`Db`, `Db\Connection`), `Core\Db\Isolation` (`Db\Connection`,
`Db\Transaction`) and `Core\Audience` (`#[Access]` alone). A use site this table missed is placed by
the rule, and the table is corrected here. `crates/nvs-stdlib/tests/enum_owners.rs` is what decides:
a class uses an enum when a parameter, an option, a field of a settings shape, a return type or a
constant of one of its members names it.

The built-in components follow the same rule. In `Novis\Image`, every enum but `Format` moves into
`Image` (`Gravity`, `Fit`, `Filter`, `Axis`, `Blend`, `Align`, `Output`, `HashKind`,
`PlaceholderKind`), and `QrLevel` becomes `QrCode::Level`. `Format` is shared and stays. In
`Novis\Intl`, the enums become `Collator::Strength`, `Collator::CaseFirst`, `NumberFormat::Style`,
`NumberFormat::CurrencyDisplay`, `NumberFormat::CompactDisplay`, `PluralRules::Kind`,
`PluralRules::Category`, `DateFormat::Length`, `DateFormat::ZoneStyle`, `RelativeTime::Unit`,
`RelativeTime::Numeric`, `ListFormat::Type` and `Locale::Service`. `Width` is shared and stays.

A published program — an example, an attack, a bench, a website snippet, a reference chapter's
listing, a template `nvs new` writes — places its enums the same way. An enum two classes share stays
at file scope, and the program says so in one line where it is declared. `tests/conformance/` is the
exception: its cases pin enum behaviour at file scope and stay there.

The rule exists so a reader finds an enum where the class it describes is declared, and so the
surface is one language. A `Core` class's card and stub show its enums, and a program written against
the reference reads like the reference. An old `Core` name is gone, not deprecated: there is no
release a program could depend on, and a second spelling would be a second name
(`rule:statements/nothing-gets-a-second-name`).

What it trades. Performance and memory: nothing, since placement changes a name and not a value.
Usability: every `Core` enum a program names changes spelling once, before the first release, and
`Core\Arr::sort($rows, {order: Core\Arr::Order::Desc})` reads with its class. Simplicity: one rule a
test enforces, in place of a judgement each contributor makes.

**Declined:**
- **Placement left to taste.** Two contributors would place one enum two ways, and the surface would
  drift one member at a time. A rule a test reads does not drift.
- **Every enum in a class, a shared one in whichever class came first.** The other users would name
  it through a class they do not otherwise touch, and the owner would be an accident of history.
- **Deprecated aliases for the old `Core` names.** Each is a second name for one enum, kept for
  programs that do not exist yet.
- **Moving the conformance cases too.** They pin the file-scope form, which stays a supported place
  for a shared enum, and the new form has cases of its own.
