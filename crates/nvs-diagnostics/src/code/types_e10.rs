//! The type checker's codes, continued from `types_e08.rs`: `E10xx`. The
//! `E08xx` range filled at `E0899`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E10xx` fails the
//! crate's tests.

codes! {
    range = "E10";

    /// A method's route attributes that do not pair:
    /// `#[Core\Router\Params]` with no `#[Core\Router\Target]` in the same
    /// method, or a second of either
    /// (`rule:routing/a-route-target-parameter-is-checked-like-a-link`). A
    /// `Params` array is checked against the route its `Target` names, so one
    /// with no `Target`, or with two to choose from, has nothing to be checked
    /// against.
    pub const E_ROUTE_ATTRIBUTES_DO_NOT_PAIR: Code = Code::new("E1000").card(
        "A method can have one `#[Core\\Router\\Target]` parameter and one \
         `#[Core\\Router\\Params]` parameter. The `Params` array is checked against the route \
         that the `Target` parameter gets, so a method with `Params` also needs a `Target`. \
         Delete the extra attribute, or add the missing `#[Core\\Router\\Target]`.",
    );
    /// `implements Revocable` on a class that has a parent class
    /// (`rule:classes/revocable-objects`). Only a root class opts in, so an
    /// access through a class type that is not revocable needs no check. The
    /// help names the root class, which is often in another file, so the code
    /// has no quick fix.
    pub const E_REVOCABLE_BELOW_ROOT: Code = Code::new("E1001").card(
        "Only a class with no parent class can implement `Revocable`. Every subclass of that \
         class is then revocable too. Write `implements Revocable` on the root class. The root \
         class is the class at the top of the hierarchy. Delete it from the subclass.",
    );
    /// An interface that extends `Revocable`
    /// (`rule:classes/revocable-objects`). An enum that implements it already
    /// takes `rule:enums/no-class-machinery`'s error, and gets no second one.
    pub const E_REVOCABLE_NOT_A_CLASS: Code = Code::new("E1002").card(
        "Only a class can implement `Revocable`. An interface cannot extend it. Write \
         `implements Revocable` on the root class of each class that should be revocable.",
    );
    /// A class that declares `revoke` or `isRevoked` while it is revocable
    /// (`rule:classes/revocable-objects`). The runtime runs both methods, and
    /// the one exception to `rule:classes/interface-default-methods`'
    /// override is that a class cannot replace them.
    pub const E_REVOCABLE_METHOD_DECLARED: Code = Code::new("E1003").card(
        "A revocable class cannot declare `revoke()` or `isRevoked()`. `Revocable` provides \
         both methods, and a class cannot replace them. Rename your method, or delete it and \
         use the one `Revocable` provides.",
    );
    /// An `attribute` declaration in a program whose `on` list names a site no
    /// retrieval reads: `interface`, `enum`, `hook`, `constant` or `case`
    /// (`rule:attributes/an-attribute-is-declared-with-its-sites`). Reported
    /// once at the declaration, so no use of it is refused one by one. It
    /// replaces `E0871`.
    pub const E_ATTRIBUTE_SITE_NOT_READ: Code = Code::new("E1004").card(
        "An attribute declared in a program can list only `class`, `method`, `property` and \
         `parameter` after `on`. `Core\\Attributes` reads an attribute only on those four. \
         Delete the other word from the `on` list.",
    );
    /// A name after `#[` that is a class, an interface, an enum or a `type`
    /// alias, not an `attribute` declaration
    /// (`rule:attributes/an-attribute-is-declared-with-its-sites`). It
    /// replaces `E0726`.
    pub const E_NOT_AN_ATTRIBUTE_DECLARATION: Code = Code::new("E1005").card(
        "Only the name of an attribute declaration can follow `#[`. This name is a class, an \
         interface, an enum or a `type` alias. Declare the attribute with \
         `attribute Name on method { field: string }`, and use that name.",
    );
    /// An attribute declared in a class or interface body, used outside that
    /// type's descendants
    /// (`rule:attributes/an-attribute-is-declared-with-its-sites`). The owner
    /// itself is not one of its descendants.
    pub const E_ATTRIBUTE_OUTSIDE_ITS_OWNER: Code = Code::new("E1006").card(
        "This attribute is declared inside a class or an interface. It can be used only on a \
         class that extends or implements that type, and on the methods, properties and \
         parameters of such a class. Move the attribute, or declare it outside the type.",
    );
    /// A short attribute name that the owner chain and the namespace or a
    /// `use` import both give
    /// (`rule:attributes/an-attribute-is-declared-with-its-sites`). Neither is
    /// chosen silently, and the message names both qualified forms.
    pub const E_ATTRIBUTE_NAME_IN_TWO_SCOPES: Code = Code::new("E1007").card(
        "Two attribute declarations have this short name. One is declared in a parent class or \
         interface, and one in the namespace or in a `use` import. Write the full name of the \
         one you mean, such as `Storable::Index` or `Shop\\Index`.",
    );
    /// A `Core\Attributes::get<T>` or `::all<T>`, a
    /// `Core\Router\Match::attribute<T>` or a
    /// `Core\Program::implementingWith<I, T>` whose `T` is not an `attribute`
    /// declaration
    /// (`rule:attributes/a-retrieval-matches-the-declaration-it-names`). It
    /// replaces the retired `E0729`.
    pub const E_ATTRIBUTE_TYPE_ARG_NOT_DECLARED: Code = Code::new("E1008").card(
        "The type argument that names an attribute must be the name of an attribute \
         declaration. This is the `T` of `Core\\Attributes::get`, `Core\\Attributes::all`, \
         `Core\\Router\\Match::attribute` and `Core\\Program::implementingWith`. A shape, a class \
         or a `type` alias is not allowed there. Write the name of the attribute you want to find.",
    );
    /// A `#[Core\Query]` parameter whose name a capture of the route's path
    /// also names
    /// (`rule:routing/a-query-parameter-is-declared-like-a-capture`). One
    /// parameter gets one value, so it is filled by the path or by the query
    /// string, never both. With this refused, every key of
    /// `Core\Router\Match::parameters()` names exactly one capture or one query
    /// value.
    pub const E_ROUTE_PARAM_CAPTURED_AND_QUERIED: Code = Code::new("E1009").card(
        "A route method's parameter gets its value from the path or from the query string, not \
         from both. This parameter has `#[Core\\Query]`, and the route's path also has a capture \
         with its name, such as `{id}` for `$id`. Delete `#[Core\\Query]` to read the value from \
         the path. Or delete the capture from the path to read it from the query string.",
    );

    /// A value written in a `Core\Router::url` call's `$params` array literal
    /// whose static type cannot be the type of the capture or `#[Core\Query]`
    /// parameter its key names (`rule:routing/link-target-and-params-are-checked`):
    /// a `Date` for a `uint`, an `int` for a `Date`, one `Parses` class for
    /// another. A `string` or `mixed` value passes, and so does a scalar whose
    /// text the parameter's conversion reads. `E0772` stays the code for a
    /// literal the conversion refuses.
    pub const E_ROUTE_LINK_VALUE_WRONG_TYPE: Code = Code::new("E1010").card(
        "A value in the `$params` of `Core\\Router::url` has a type that the route parameter \
         cannot take, such as a `Core\\Time\\Date` for a `uint` parameter. The route could not \
         read the link, so the program does not compile. Give a value of the parameter's type. \
         A `string` value is also allowed: the route checks its text when the link is followed.",
    );

    /// A member an `extend` block adds whose name is already a member of the
    /// same kind on its type or an ancestor, is added by another block to a
    /// related type, or is declared by a class or interface of the program that
    /// extends or implements the type
    /// (`rule:classes/an-extend-block-adds-members-to-a-type`). An added member
    /// never replaces or overrides anything, so a call reaches the same member
    /// whatever the receiver's static type is.
    pub const E_ADDED_MEMBER_COLLIDES: Code = Code::new("E1011").card(
        "An `extend` block adds a member whose name is already used. The type or one of its \
         parent types has a member with this name, or another `extend` block adds it, or a \
         class that extends the type declares it. An added member cannot replace another \
         member. Give the added member a different name.",
    );

    /// A method an `extend` block adds whose name is a member of a
    /// compiler-declared interface, such as `compareTo` or `toString`
    /// (`rule:classes/an-extend-block-adds-members-to-a-type`). An operator, a
    /// conversion, `foreach` or a route calls such a method by its name on an
    /// object of any class, and an added method is reached only by a call the
    /// compiler resolves. The roster is `nvs_hir::interfaces::RESERVED`, and the
    /// names are the members `nvs_types::iter_lib` seeds for it.
    pub const E_ADDED_NAME_RUNTIME_CALLS: Code = Code::new("E1012").card(
        "An `extend` block adds a method with a name that Novis itself calls, such as \
         `compareTo` or `toString`. Novis calls these methods by name, for example for `<` \
         or for a conversion to `string`. It never calls an added method this way, so the \
         method would not work as expected. Give the added method a different name.",
    );

    /// A method called through a union of classes where an `extend` block
    /// adds it to some classes but not from one block to every class
    /// (`rule:classes/an-added-member-never-changes-a-type`). Which member runs
    /// would otherwise depend on the class the value has, and an added
    /// method is never dispatched by class.
    pub const E_UNION_ADDED_MEMBER_NOT_SHARED: Code = Code::new("E1013").card(
        "A method is called on a value whose type is a union of classes, such as `A|B`. An \
         `extend` block adds this method to some of the classes, but no one block adds it to \
         every class. Test the value with `is` first, and call the method on one class. A block \
         that extends an interface all the classes implement adds the method to every class.",
    );
}
