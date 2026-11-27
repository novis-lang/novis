The runtime calls five methods with a property's name as their first parameter: `initProperty`
(`LateInit`), `onPropertyGet` and `onPropertySet` (`PropertyObserver`), and `beforePropertySet` and
`beforePropertyUnset` (`PropertyGuard`). Inside one of them, in a class that implements its interface,
a string literal compared with the name parameter is a use of the property with that name, on the
method's class or a class above it. A comparison is `==` or `!=` with the parameter on either side,
a `match` on the parameter, whose arm conditions are the literals, and a `switch` on it, whose `case`
values are. A body that writes the name parameter compares nothing the tools read.

The editor's index records each such literal, between its quotes, as an occurrence of the property
(`rule:ide/five-features-are-one-reference-index`). So a rename rewrites it and keeps its quotes, and
references and highlight list it, with no code of their own. A rename of the property is refused, with
the place named, while a callback of the class or a class below it writes the old name in any other
way, such as an array of names or an argument to another call, or writes its name parameter: the
rename would miss that use. A string outside these methods is not tied to the property and is not read.

A compared literal that no property of the class, of a class above it or of a class below it has is
warning `W1039`, at the literal: "`'veiws'` is not a property of `Page`". It is a typo the program
would find only at run time. It is a warning, never an error, because a class below may be declared
in another program.

- **Cost:** tooling and checking time only. Each callback body is read once more when its file is
  indexed or checked, and the spans it gives are dropped with the answer. Nothing runs on the request
  path, and a class with no callback pays nothing.

**Declined:**
- **Every string equal to a property name is a use.** A word such as `name` or `id` is written in
  many strings that have nothing to do with the property, and a rename would change them all.
- **A name in a variable or a constant compared with the parameter.** The tools cannot prove what it
  is, so it refuses nothing and is renamed with nothing.
- **An error for a compared name no property has.** A class below may be declared in a program the
  checker does not see, and it may have that property.
