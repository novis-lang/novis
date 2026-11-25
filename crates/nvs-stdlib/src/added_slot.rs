//! Which `Core` classes reserve the added slot
//! (`rule:classes/an-added-property-lives-in-one-hidden-slot`).
//!
//! A stored property an `extend` block adds lives in one hidden slot of its
//! object, after the class's declared slots, so no `Core` member's slot index
//! moves. Every `Core` class with instances records a decision here, by name,
//! and [`decision`] reads it. The test below fails on a class with instances
//! and no decision, so a new class cannot take the slot, or go without it,
//! by accident.
//!
//! A class goes without the slot for one of two reasons, each a constant
//! below:
//!
//! - [`VALUE`]: a `Core\Time` value compares by value, and every operation
//!   returns a new value. An added value would be lost on the first operation.
//! - [`IMMORTAL`]: an instance may be an immortal constant or a unit's
//!   description, which lives in read-only memory and is shared by every
//!   request. An added value there would be written to memory no request owns.
//!
//! A class whose instance is only a handle to state the request keeps
//! elsewhere, such as `Core\Socket`, still reserves the slot, because the
//! object itself belongs to the request.
//!
//! A `Core\Ast` production has `Core\Ast\Node`'s layout slot for slot, so
//! all of them reserve the slot at the same index.
//!
//! **What it spends:** nothing here. The rule fragment states the per-object
//! cost once the slot is given.

/// A decision for one `Core` class with instances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddedSlot {
    /// The class reserves the slot after its declared slots.
    Reserved,
    /// The class has no slot, for the reason given.
    None(&'static str),
}

/// The reason for a `Core\Time` value class.
pub const VALUE: &str = "a `Core\\Time` value compares by value, and every operation returns a new \
                         value";

/// The reason for a class whose instances can be immortal.
pub const IMMORTAL: &str = "an instance can be immortal and shared by every request";

/// The classes with no slot, and why.
const NO_SLOT: &[(&str, &str)] = &[
    (r"Core\Time\Date", VALUE),
    (r"Core\Time\TimeOfDay", VALUE),
    (r"Core\Time\Instant", VALUE),
    (r"Core\Time\DateTime", VALUE),
    (r"Core\Time\Duration", VALUE),
    (r"Core\Time\Zone", VALUE),
    (r"Core\Html\Markup", IMMORTAL),
    (r"Core\Reflect\ClassInfo", IMMORTAL),
    (r"Core\Reflect\MethodInfo", IMMORTAL),
    (r"Core\Reflect\ParameterInfo", IMMORTAL),
    (r"Core\Reflect\PropertyInfo", IMMORTAL),
    (r"Core\Reflect\ConstantInfo", IMMORTAL),
    (r"Core\Reflect\AttributeInfo", IMMORTAL),
    (r"Core\Reflect\EnumInfo", IMMORTAL),
];

/// The classes that reserve the slot.
const RESERVED: &[&str] = &[
    r"Core\BigInt",
    r"Core\Regex\Match",
    r"Core\Regex\Pattern",
    r"Core\Json\Elements",
    r"Core\Json\Pieces",
    r"Core\IO\Lines",
    r"Core\IO\StdinStream",
    r"Core\IO\Walk",
    r"Core\IO\File",
    r"Core\IO\Metadata",
    r"Core\Process\Result",
    r"Core\Process\Handle",
    r"Core\ObjectMap",
    r"Core\ObjectSet",
    r"Core\Heap",
    r"Core\Random\Seeded",
    r"Core\Uuid",
    r"Core\Hash\Stream",
    r"Core\Uri",
    r"Core\Router\Match",
    r"Core\Csv\Rows",
    r"Core\Test\Response",
    r"Core\Test\RunResult",
    r"Core\Test\LogRecord",
    r"Core\Test\SentMail",
    r"Core\Test\SentRequest",
    r"Core\Test\Outcome",
    r"Core\Test\Summary",
    r"Core\Task\Channel",
    r"Core\Script\Handle",
    r"Core\Script\ExitReport",
    r"Core\Cli\Text",
    r"Core\Cli\Color",
    r"Core\Cli\Style",
    r"Core\Cli\Live",
    r"Core\Cli\Progress",
    r"Core\Request\Mount",
    r"Core\Request\BodyStream",
    r"Core\Request\Files",
    r"Core\Request\Part",
    r"Core\Request\PartContent",
    r"Core\Response\Stream",
    r"Core\Socket",
    r"Core\Socket\Message",
    r"Core\Sse",
    r"Core\Sse\Message",
    r"Core\Crypto\PublicKey",
    r"Core\Crypto\KeyPair",
    r"Core\Passkey\Credential",
    r"Core\WebPush\Subscription",
    r"Core\WebPush\Sent",
    r"Core\Jwt\KeySet",
    r"Core\Jwe\Key",
    r"Core\Markdown\Groups",
    r"Core\Markdown\Live",
    r"Core\Markdown\Update",
    r"Core\Xml\Node",
    r"Core\Xml\Reader",
    r"Core\Xml\Writer",
    r"Core\Compress\Compressor",
    r"Core\Compress\Decompressor",
    r"Core\Http\Target",
    r"Core\Http\Identity",
    r"Core\Http\Response",
    r"Core\Http\TlsInfo",
    r"Core\Http\Socket",
    r"Core\Http\Stream",
    r"Core\Http\Event",
    r"Core\Http\Events",
    r"Core\Http\Lines",
    r"Core\Http\Chunks",
    r"Core\Http\Part",
    r"Core\Net\Stream",
    r"Core\Net\Listener",
    r"Core\Net\Datagram",
    r"Core\Net\Datagram\Message",
    r"Core\Cache\Store",
    r"Core\Cache\SecretEntry",
    r"Core\RateLimit\Decision",
    r"Core\Ast\Node",
    r"Core\Db\Connection",
    r"Core\Db\Transaction",
    r"Core\Db\Rows",
    r"Core\Db\Stream",
    r"Core\Db\Row",
    r"Core\Db\Write",
    r"Core\Db\Column",
    r"Core\Db\InList",
    r"Core\Db\Schema",
    r"Core\Db\Plan",
    r"Core\Db\Plan\Step",
    r"Core\Ldap\Connection",
    r"Core\Ldap\Entries",
    r"Core\Ldap\Changes",
    r"Core\Ldap\Entry",
    r"Core\Ldap\Filter",
    r"Core\Ldap\Change",
    r"Core\Ldap\Dn",
    r"Core\Ldap\Sid",
    r"Core\Ldap\Ad\AccountFlags",
    r"Core\Ldap\Ad\GroupType",
    r"Core\Queue\Id",
    r"Core\Queue\Stats",
];

/// The decision recorded for the `Core` class `name`, or `None` for a name
/// with no decision: a namespace class, or a name that is not a `Core` class.
#[must_use]
pub fn decision(name: &str) -> Option<AddedSlot> {
    if RESERVED.iter().any(|class| class.eq_ignore_ascii_case(name)) {
        return Some(AddedSlot::Reserved);
    }
    NO_SLOT
        .iter()
        .find(|(class, _)| class.eq_ignore_ascii_case(name))
        .map(|&(_, reason)| AddedSlot::None(reason))
}

#[cfg(test)]
mod tests {
    use super::{NO_SLOT, RESERVED, decision};
    use crate::classes::CLASSES;

    #[test]
    fn added_slot_every_core_class_with_instances_has_a_decision() {
        let missing: Vec<&str> = CLASSES
            .iter()
            .filter(|class| crate::instance::has_instances(class))
            .filter(|class| decision(class.name).is_none())
            .map(|class| class.name)
            .collect();
        assert!(
            missing.is_empty(),
            "these `Core` classes have instances and no added-slot decision in \
             `added_slot.rs`: {missing:#?}"
        );
        // Every decision names a class with instances, and only once.
        let named: Vec<&str> = RESERVED
            .iter()
            .copied()
            .chain(NO_SLOT.iter().map(|&(class, _)| class))
            .collect();
        for name in &named {
            assert!(
                CLASSES
                    .iter()
                    .any(|class| class.name == *name && crate::instance::has_instances(class)),
                "`{name}` has an added-slot decision and is not a `Core` class with instances"
            );
            assert_eq!(
                named.iter().filter(|other| *other == name).count(),
                1,
                "`{name}` has two added-slot decisions"
            );
        }
    }
}
