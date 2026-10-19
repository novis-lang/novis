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
         `__DIR__`, write `Core\\Path::thisFile()` and `Core\\Path::thisDir()`.",
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
    /// A doc comment's `@see` naming something that does not resolve — a class
    /// nothing declares, a member no class in the chain has, or nothing at all
    /// after the tag. `rule:tooling/doc-comment-tags-are-see-and-example`
    /// keeps the tag set at two by making each one buy a check, and this is
    /// `@see`'s: a cross-reference that resolves where it is written cannot
    /// rot into a link to a member that has since been renamed. Resolution
    /// rather than the parser, because the question is what a name means, and
    /// the same one [`E_UNDEFINED_MEMBER`] asks of a `Class::member` in code.
    pub const E_DOC_SEE_UNRESOLVED: Code = Code::new("E0323").card(
        "A `@see` tag in a doc comment names a class or member that does not exist. Fix the name, \
         or delete the tag if the thing it named was removed.",
    );
    /// A doc comment's `@example` naming a path that is in a directory the
    /// test corpus walks but holds no file. The other half of
    /// `rule:tooling/doc-comment-tags-are-see-and-example`'s check on the tag
    /// is [`E_DOC_EXAMPLE_NOT_WALKED`], split from this one because the two
    /// carry opposite repairs: write the file, or move it.
    pub const E_DOC_EXAMPLE_NOT_FOUND: Code = Code::new("E0324").card(
        "An `@example` tag in a doc comment names a file that does not exist. The path is \
         relative to the file that contains the comment. Create the file, or fix the path.",
    );
    /// A doc comment's `@example` naming a path in no directory the test
    /// corpus walks. An example nothing compiles is one that rots in a page
    /// while the member it documents moves on, which is the whole reason
    /// `rule:tooling/doc-comment-tags-are-see-and-example` keeps the tag: a
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
        "Your class cannot extend a `Core` class, and cannot implement one. To add methods, \
         write your own class and give it a property that contains the `Core` value. Your own \
         error classes extend `RuntimeError`, `LogicError` or another error class that is not \
         under `Core`.",
    );
}
