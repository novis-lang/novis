An enum body declares cases, instance and static methods, and constants, and an enum method is a direct
call that takes the case's integer as `$this`. The representation stays the integer constant of
`rule:enums/representation`: an enum has no descriptor, and no case is allocated.

A property in an enum body is **`E0220`**, and so are a nested enum and a method that uses `yield`.
The help for a property names a class for state. A generator keeps its receiver in a state object,
and a case has no object to keep. An `implements` clause is **`E0218`**. A constant may not take a
case's name, because `E::X` reads both: the constant is **`E0304`**, beside the case.

A method on a value typed as one case, `E::A`, resolves as it does on the enum. `$this` in an enum
method is the case, and `self` names the enum. `self::Case`, `self::CONSTANT` and `self::method()`
work in the body, and an enum a class declares sees its owner's other enums by their bare names. A
method reference, `E::A->label(...)` or `E::make`, is a callable over the same function.

An instance method is compiled with the case's integer in parameter 0. A static method takes a `0`
there, which its caller passes and nothing reads, so every call of an enum method is one direct call
with one convention, and no call of one needs a descriptor. Nothing overrides an enum method, so no
call of one is virtual.

An enum declared in another package takes methods and constants through an `extend` block instead
(`rule:classes/an-extend-block-adds-members-to-a-type`), and a block of an enum the program declares
is `E0337`.

**What it spends:** an enum method costs what a static method taking the enum costs. Nothing per case
and nothing per value. A method reference over one stores the case's integer in its callable object,
one word, as it stores an object receiver.

**Declined:**
- **Properties on a case.** A case is an integer constant, and a property would need an object.
- **An `implements` clause.** A case would need a descriptor for dispatch through the interface.
- **Keeping methods on a helper class.** It splits one type's behaviour across two names, and
  `$case->label()` reads as the type's own operation.
- **A generator method.** Its state object would carry an integer receiver beside every object one,
  and a method that returns an array does the same job.
