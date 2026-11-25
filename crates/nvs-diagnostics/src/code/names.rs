//! The codes of name resolution, `E03xx`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E03xx` fails the
//! crate's tests.

codes! {
    range = "E03";

    /// A variable read before anything was assigned to it.
    pub const E_UNDEFINED_VARIABLE: Code = Code::new("E0301").card(
        "A variable is used before it is declared, or before a value is assigned to it. Declare \
         it with its type first, as in `int $count = 0;`.",
    );
    // `E0302` is retired and is never reused: Novis has no free functions, so
    // a call to one that does not exist is `E0320`.
    /// A reference to a class, interface or enum that does not exist.
    pub const E_UNDEFINED_CLASS: Code = Code::new("E0303").card(
        "This class, interface or enum is not declared anywhere the program can see. Check the \
         name for a typo. Then import it with `use`, or `require` the file that declares it.",
    );
    /// Two declarations with the same name in the same scope.
    pub const E_DUPLICATE_DECLARATION: Code = Code::new("E0304").card(
        "Two declarations in the same scope have the same name. Rename one of them, or delete \
         the one you do not need.",
    );
    /// A class hierarchy (`extends`/`implements`) that forms a cycle.
    pub const E_CIRCULAR_INHERITANCE: Code = Code::new("E0305").card(
        "A class or interface inherits from itself through a chain of `extends` or `implements`. \
         Remove one `extends` or `implements` so that the chain has an end.",
    );
    /// A `use` statement that resolves to nothing.
    pub const E_UNRESOLVED_IMPORT: Code = Code::new("E0306").card(
        "A `use` statement names a class, interface or enum that is not declared. Check the name \
         for a typo, and check the namespace it is declared in.",
    );
    /// A `type` alias whose expression is nothing but one bare class,
    /// interface or enum atom — `use … as …` in disguise; see
    /// `rule:types/alias-is-never-a-bare-class`.
    pub const E_TYPE_ALIAS_ALIASES_CLASS: Code = Code::new("E0307").card(
        "A `type` alias cannot be one class, interface or enum on its own. Use the class name \
         directly. An alias is for a union, an intersection or an `array<...>` type.",
    );
    /// A `Class::member` reference (a static call, a class constant, an
    /// enum case, or a static property) names nothing declared on that class
    /// or any of its `extends`/`implements` ancestors — `rule:classes/no-free-functions-or-constants`'s "every
    /// callable and constant is a class member" has no bare-name fallback to
    /// fall into instead.
    pub const E_UNDEFINED_MEMBER: Code = Code::new("E0309").card(
        "The class has no method, constant, enum case or static property with this name. Its \
         parent classes and interfaces do not have one either. Check the name for a typo.",
    );
    /// A `type` alias whose expansion, followed far enough, refers back to
    /// itself — `type A = B; type B = A;` or any longer cycle; see
    /// `rule:types/type-alias`.
    pub const E_TYPE_ALIAS_CYCLE: Code = Code::new("E0310").card(
        "A `type` alias refers back to itself, directly or through other aliases, as in \
         `type A = B; type B = A;`. Change one alias so that it ends in a real type, such as \
         `int` or a class.",
    );
    /// A `require` whose path is a string literal, resolved statically per
    /// `rule:statements/require-is-the-only-inclusion-construct`, but does not name a file that can be loaded as source (it
    /// does not exist, or is not valid UTF-8).
    pub const E_REQUIRE_TARGET_NOT_FOUND: Code = Code::new("E0311").card(
        "The file that `require` names cannot be loaded. It does not exist, or it is not valid \
         UTF-8 text. Check the path.",
    );
    /// A `require` whose path is not statically known per
    /// `rule:statements/require-is-the-only-inclusion-construct`: a variable, a call, an
    /// interpolation, or a class constant no file read so far declares. No file is loaded for
    /// it while compiling, so it could only ever throw where it is reached.
    pub const E_REQUIRE_PATH_NOT_STATIC: Code = Code::new("E0330").card(
        "The path after `require` must be known before the program runs. Write it as text, \
         such as `require './autoload.nvs';`. A relative path starts from the folder of the file \
         that contains the `require`. You can also join text and class constants with `.`.",
    );
    /// A `require` chain whose statically-resolved string-literal paths lead back
    /// to a file already being resolved — `require`'s own semantics (same
    /// frame, runs every time reached) give this no other resolution than a
    /// diagnostic, the same way `crate::hierarchy`'s `extends`/trait-use
    /// cycle and `crate::aliases`'s `type` alias cycle are both handled.
    pub const E_CIRCULAR_REQUIRE: Code = Code::new("E0312").card(
        "Two or more files `require` each other in a circle. Remove one of these `require` \
         statements so that the chain has an end.",
    );
    /// `$this->name` where `name` is not declared on the enclosing class or
    /// any `extends`/`implements`/trait-use ancestor — `rule:classes/no-dynamic-properties`'s "no
    /// `__get`/`__set` fallback" for the one receiver shape resolvable
    /// without a type checker.
    pub const E_UNDEFINED_PROPERTY: Code = Code::new("E0313").card(
        "`$this->name` uses a property that this class and its parents do not declare. Novis \
         does not add properties to an object while the program runs. Declare the property in \
         the class, or fix the name.",
    );
    /// A `require` whose string-literal path resolves on disk only because the
    /// filesystem is case-insensitive — `require 'mailer.nvs';` finding
    /// `Mailer.nvs`. Reported on Windows/macOS so the same source is not a
    /// `E_REQUIRE_TARGET_NOT_FOUND` on Linux; see `rule:programs/path-case`,
    /// which extends `rule:programs/autoload`'s exact-name rule from
    /// `autoload` to `require`.
    pub const E_REQUIRE_PATH_CASE_MISMATCH: Code = Code::new("E0314").card(
        "The file name in `require` uses different upper and lower case letters than the file on \
         disk. This works on Windows and macOS but fails on Linux. Write the name exactly as it \
         is on disk.",
    );
    /// Two `autoload` declarations in one program claim the same namespace
    /// prefix — `rule:programs/autoload`'s "one prefix has one home". An explicit prefix deliberately does
    /// *not* collide with a `discover` glob that would produce the same one:
    /// there the glob skips the name, which is what makes a vendor override
    /// work.
    pub const E_DUPLICATE_AUTOLOAD_PREFIX: Code = Code::new("E0315").card(
        "Two `autoload` declarations use the same namespace prefix. A prefix can load from only \
         one place, so delete one of them.",
    );
    /// An `autoload` declaration in a file that was itself reached through
    /// the autoload map — `rule:programs/autoload`, which honors a declaration only in a
    /// file reachable by `require` from the entry point, since otherwise the
    /// map would depend on itself.
    pub const E_AUTOLOAD_IN_AUTOLOADED_FILE: Code = Code::new("E0316").card(
        "An `autoload` declaration is in a file that autoload itself loaded. Move it to a file \
         that the program's entry point loads with `require`.",
    );
    /// A file reached through an autoload root that does not hold exactly one
    /// top-level declaration named after it — `rule:programs/one-declaration-per-autoloaded-file`. Without the rule,
    /// whether a name exists in the program depends on what was resolved
    /// first, which makes the build non-reproducible and the cache unkeyable.
    pub const E_AUTOLOAD_FILE_SHAPE: Code = Code::new("E0317").card(
        "A file that autoload loads must contain exactly one declaration, and its name must match \
         the file name. Move each other declaration into a file of its own.",
    );
    /// An `autoload discover` glob that is not one `*` occupying a whole path
    /// segment, or whose base directory does not exist — `rule:programs/autoload`. A
    /// *matched* directory with an unusable name is skipped in silence (a
    /// glob over a filesystem always sweeps `.git` and `vendor`); the glob
    /// itself is diagnosed, since one that silently discovers nothing is the
    /// worst outcome on offer.
    pub const E_AUTOLOAD_GLOB_SHAPE: Code = Code::new("E0318").card(
        "An `autoload discover` pattern needs exactly one `*`, and the `*` must be a whole \
         directory name, as in `vendor/*`. The directory before the `*` must also exist.",
    );
    /// A bare name used where a value is expected — `PHP_EOL`, `MY_LIMIT` —
    /// which in PHP would be a global constant fetch.
    /// `rule:classes/no-free-functions-or-constants` leaves no such storage
    /// row: a constant is always a class constant, so there is no name for this to resolve against and nothing
    /// below the resolver to lower it to.
    pub const E_NO_GLOBAL_CONSTANT: Code = Code::new("E0319").card(
        "A bare name such as `MY_LIMIT` is not a constant, because Novis has no global constants. \
         Every constant belongs to a class, so write `Class::NAME`. For a PHP constant, use its \
         `Core` replacement, for example `Core\\Math::PI` for `M_PI`. For `__FILE__` and \
         `__DIR__`, write `Core\\Path::thisFile()` and `Core\\Path::thisDirectory()`.",
    );
    /// A bare name called as a function — `strlen($s)` — which in PHP would
    /// be a global function call. `rule:classes/no-free-functions-or-constants`: every callable is a method,
    /// with no exception for built-ins, which live under the reserved `Core`
    /// namespace. Split from [`E_NO_GLOBAL_CONSTANT`] because the two carry
    /// different replacements even though the callee is the same node.
    pub const E_NO_FREE_FUNCTION: Code = Code::new("E0320").card(
        "A bare function call such as `strlen($s)` is not allowed, because Novis has no global \
         functions. Every function is a method of a class. The built-in functions are in the \
         `Core` namespace, for example `Core\\Str::length($s)`.",
    );
    /// `self`, `static` or `parent` written where a value is expected, rather
    /// than on the left of a `::`. Each of the three names a *class*, and a
    /// class is not a value in Novis — there is no class-object reflection
    /// handle (`rule:classes/no-free-functions-or-constants`
    /// puts every reflective question on `Core\Reflect` instead).
    pub const E_CLASS_NAME_NOT_A_VALUE: Code = Code::new("E0321").card(
        "`self`, `static` and `parent` name a class, and a class is not a value. Write `::` and \
         a member after the name, as in `self::create()` or `static::NAME`. To inspect a class, \
         use `Core\\Reflect`.",
    );
    /// A qualified name that does not resolve, but which PHP's rule would have
    /// resolved relative to the enclosing namespace — `Models\User` written
    /// inside `namespace App;`, meaning `App\Models\User`. This is the one
    /// construct
    /// `rule:statements/a-qualified-name-is-absolute`
    /// changes the meaning of, so it gets a diagnostic naming the absolute
    /// spelling rather than the generic [`E_UNDEFINED_CLASS`] a reader would
    /// otherwise have to work backwards from. A qualified name that resolves
    /// neither way is that generic error, not this one.
    pub const E_RELATIVE_QUALIFIED_NAME: Code = Code::new("E0322").card(
        "A name that contains a `\\` always starts from the top namespace. Inside \
         `namespace App;`, the name `Models\\User` does not mean `App\\Models\\User`. Write the \
         full name, or import it with `use App\\Models\\User;`.",
    );
    /// A doc comment's `@see` naming something that does not resolve. A
    /// member target names a class nothing declares, a member no class in the
    /// chain has, or nothing at all after the tag. A file target names no
    /// file, a path that leaves the project, a heading the Markdown file does
    /// not have, or an anchor on a file that is not Markdown.
    /// `rule:tooling/doc-comment-tags-are-closed-and-checked` makes each tag
    /// buy a check, and this is `@see`'s: a cross-reference that resolves where
    /// it is written cannot rot into a link to something since renamed. A miss
    /// carries the closest member or heading as its fix.
    pub const E_DOC_SEE_UNRESOLVED: Code = Code::new("E0323").card(
        "A `@see` tag in a doc comment names something that does not exist. The tag names a \
         class or a member, such as `@see Shop\\Cart::total`. It can also name a file in the \
         project, such as `@see docs/guide.md#usage`. The path is relative to the file that \
         contains the comment. The part after `#` must match a heading of the Markdown file. \
         Fix the name or the path, or delete the tag.",
    );
    /// A doc comment's `@example` naming a path that is in a directory the
    /// test corpus walks but holds no file. The other half of
    /// `rule:tooling/doc-comment-tags-are-closed-and-checked`'s check on the tag
    /// is [`E_DOC_EXAMPLE_NOT_WALKED`], split from this one because the two
    /// carry opposite repairs: write the file, or move it.
    pub const E_DOC_EXAMPLE_NOT_FOUND: Code = Code::new("E0324").card(
        "An `@example` tag in a doc comment names a file that does not exist. The path is \
         relative to the file that contains the comment. Create the file, or fix the path.",
    );
    /// A doc comment's `@example` naming a path in no directory the test
    /// corpus walks. An example nothing compiles is one that rots in a page
    /// while the member it documents moves on, which is the whole reason
    /// `rule:tooling/doc-comment-tags-are-closed-and-checked` keeps the tag: a
    /// path outside `examples/` and `tests/` buys no check.
    pub const E_DOC_EXAMPLE_NOT_WALKED: Code = Code::new("E0325").card(
        "An `@example` tag names a file that the tests never compile. Move the example under \
         `examples/` or `tests/`, so that a change which breaks it is found.",
    );
    /// A public member carrying no doc comment, reported only under
    /// `nvs check --strict-docs`. `rule:tooling/strict-docs` keeps it silent
    /// everywhere else and no autofix can satisfy it: with no `@param` and no
    /// `@return` to fill in, a generated `///` would be empty, so the only way
    /// to clear it is to write a sentence. A method that overrides or
    /// implements a documented method inherits that comment and is not
    /// reported.
    pub const E_DOC_MISSING: Code = Code::new("E0326").card(
        "A public member has no doc comment, and `nvs check --strict-docs` needs one. Write a \
         `///` line above it, or make the member `private`. A method that overrides or implements \
         a documented method uses that method's comment.",
    );
    /// An `autoload` prefix that no name can ever match: a segment that is
    /// not a `PascalCase` namespace segment, or a `{..}` directory segment
    /// whose directory name is not one — `rule:programs/autoload`. A PSR-4
    /// wildcard such as `'App\*'` is the usual way to reach it, and without
    /// this code it failed only later, as an `E0303` far from the declaration.
    pub const E_AUTOLOAD_PREFIX_SHAPE: Code = Code::new("E0327").card(
        "Each part of an `autoload` prefix must be a `PascalCase` namespace name, or one `{..}` \
         that names a directory. A wildcard such as `App\\*` is not allowed. To load many \
         directories, use `autoload discover`.",
    );
    /// An `extends`/`implements` clause naming a `Core` interface that only
    /// `Core` classes implement, such as `Core\Db\Queryable`
    /// (`rule:core-classes/db-transactions`). The name exists, so this is not
    /// [`E_UNDEFINED_CLASS`]: a program may write it as a type and test a
    /// value with `is`, and its members read state only a `Core` class has.
    pub const E_CORE_INTERFACE_NOT_IMPLEMENTABLE: Code = Code::new("E0328").card(
        "This `Core` interface is implemented only by `Core` classes. Your class cannot \
         implement it. You can still use it as a type for a parameter or a variable, and test \
         a value with `is`.",
    );
    /// An `extends`/`implements` clause naming a `Core` class: a registry
    /// class such as `Core\Test\Response`, or one of the exception classes
    /// under `Core` such as `Core\Db\DbError`
    /// (`rule:core-api/core-classes-are-sealed`). A `Core` member reads its
    /// receiver's slots by index, so a program's subclass would hand it slots
    /// it never wrote.
    pub const E_CORE_CLASS_NOT_EXTENDABLE: Code = Code::new("E0329").card(
        "Your class cannot extend a `Core` class, and cannot implement one. To add methods to a \
         `Core` class, write an `extend` block: `extend Core\\Time\\Date as Name { … }`. Your \
         own error classes extend `RuntimeError`, `LogicError` or another error class that is \
         not under `Core`.",
    );
    /// A doc comment's `@param $name` naming no parameter of the declaration
    /// below it (`rule:tooling/doc-comment-tags-are-closed-and-checked`). Where
    /// exactly one parameter has no `@param`, the fix renames the tag to it,
    /// which is the repair after a parameter was renamed by hand. Otherwise the
    /// fix deletes the line.
    pub const E_DOC_PARAM_UNKNOWN: Code = Code::new("E0335").card(
        "An `@param` tag names a parameter that the function does not have. Write the name of \
         one of its parameters, or delete the line.",
    );
    /// A doc comment that documents one thing twice: a parameter with two
    /// `@param` lines, two `@return` lines, or one class in two `@throws`
    /// lines (`rule:tooling/doc-comment-tags-are-closed-and-checked`). The fix
    /// deletes the second line.
    pub const E_DOC_TAG_REPEATED: Code = Code::new("E0331").card(
        "A doc comment documents the same parameter, return value or error twice. Keep one of \
         the two lines and delete the other.",
    );
    /// The first word of an `@param` or `@return` sentence is the declared
    /// type (`rule:tooling/doc-comment-tags-are-closed-and-checked`). The
    /// signature is the one place a type is written, and the fix deletes the
    /// word. `null` alone is never reported, since it starts normal sentences.
    pub const E_DOC_TAG_REPEATS_TYPE: Code = Code::new("E0332").card(
        "A doc comment does not repeat a type. The type is already in the function's signature. \
         Start the sentence with what the value means, for example \
         `@param $cents the amount, in cents`.",
    );
    /// An `@param`, `@return` or `@throws` on a declaration that does not take
    /// it (`rule:tooling/doc-comment-tags-are-closed-and-checked`): any of the
    /// three on a class, a property, a constant, an enum case or a `type`
    /// alias, and `@return` on a constructor or on a method that returns
    /// `void` or `never`. The fix deletes the line.
    pub const E_DOC_TAG_MISPLACED: Code = Code::new("E0333").card(
        "`@param` and `@throws` go on a function, a method or a constructor. `@return` goes on a \
         function or method that returns a value. It does not go on a constructor, or on a \
         method that returns `void` or `never`. Delete the line, or move it to the right \
         declaration.",
    );
    /// An `@throws Name` whose name resolves to no class, or to a class that
    /// is not a `Throwable` (`rule:tooling/doc-comment-tags-are-closed-and-checked`).
    /// The name resolves like a type name in that file, `use` lines included.
    /// The body is not checked against the tag: Novis has no checked
    /// exceptions. The fix deletes the line.
    pub const E_DOC_THROWS_NOT_THROWABLE: Code = Code::new("E0334").card(
        "An `@throws` tag names a class that does not exist, or a class that is not an error. \
         Write the name of a class that extends `Throwable`, such as `RuntimeError`, or delete \
         the line.",
    );
    /// An `extend` block whose target is a declared name that is not a
    /// class, an interface or an enum: a `type` alias, an attribute or another
    /// block (`rule:classes/an-extend-block-adds-members-to-a-type`). A name
    /// nothing declares is [`E_UNDEFINED_CLASS`], which carries the `use`
    /// line that would resolve it.
    pub const E_EXTEND_TARGET_NOT_A_TYPE: Code = Code::new("E0336").card(
        "An `extend` block adds methods and constants to a class, an interface or an enum. \
         The name after `extend` must be one of these. A `type` alias, an attribute or another \
         `extend` block cannot be extended.",
    );
    /// An `extend` block of a type its own package declares, or, in the
    /// application, of a type the application declares
    /// (`rule:classes/an-extend-block-adds-members-to-a-type`). Every type a
    /// project owns keeps one place for each member.
    pub const E_EXTEND_OWN_TYPE: Code = Code::new("E0337").card(
        "An `extend` block adds methods to a type from `Core` or from another package. This \
         type is declared in your own code, so add the method to the type itself. An enum \
         takes methods and constants in its own body.",
    );
    /// A member an `extend` block cannot add: a constructor, a `protected`,
    /// `abstract` or `final` member, a `static` property, a new enum case,
    /// `readonly` on an added property, a stored property with no default,
    /// `static::` or `parent::` in the block's body, or `yield` in an added
    /// static method or in a block on an enum
    /// (`rule:classes/an-extend-block-adds-members-to-a-type`). The help names
    /// what to write instead.
    pub const E_EXTEND_MEMBER_NOT_ADDABLE: Code = Code::new("E0338").card(
        "An `extend` block can add `public` and `private` methods, static methods, constants \
         and properties. It cannot add a constructor, a `protected`, `abstract` or `final` \
         member, or a `static` property. Write a static method that creates the object in \
         place of a constructor.",
    );
}
