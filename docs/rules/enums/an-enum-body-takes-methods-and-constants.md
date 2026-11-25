An enum body declares cases, instance and static methods, and constants, and an enum method is a direct
call that takes the case's integer as `$this`. The representation stays the integer constant of
`rule:enums/representation`: an enum has no descriptor, and no case is allocated.

A property in an enum body is still **`E0220`**, and so is a nested enum. Its help names a class for
state. An `implements` clause is still **`E0218`**. A method on a value typed as one case, `E::A`,
resolves as it does on the enum. `$this` in an enum method is the case, and `self` names the enum.
`self::Case` and `self::CONSTANT` work in the body.

An enum declared in another package takes methods and constants through an `extend` block instead
(`rule:classes/an-extend-block-adds-members-to-a-type`), and a block of an enum the program declares
is `E0337`.

**What it spends:** an enum method costs what a static method taking the enum costs. Nothing per case
and nothing per value.

**Declined:**
- **Properties on a case.** A case is an integer constant, and a property would need an object.
- **An `implements` clause.** A case would need a descriptor for dispatch through the interface.
- **Keeping methods on a helper class.** It splits one type's behaviour across two names, and
  `$case->label()` reads as the type's own operation.
