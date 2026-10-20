//! `rule:classes/a-covariant-type-is-checked-where-a-value-enters`'s surface,
//! at the parser: the contextual `covariant` before a parameter's or a
//! property's type, in every form a declaration writes it, and the same word
//! still an identifier everywhere else.
//!
//! Part of [`super`]'s test suite, split to mirror the grammar modules
//! themselves; the helpers every module here calls are in [`super`].

use super::*;

/// The members of the single class in a one-class file.
fn members(src: &str) -> Vec<ClassMember> {
    let stmts = parse_file_ok(src);
    let [stmt] = &stmts[..] else {
        panic!("expected one statement: {stmts:?}");
    };
    let StmtKind::ClassDecl(decl) = &stmt.kind else {
        panic!("expected a class: {stmt:?}");
    };
    decl.members.clone()
}

/// The one parameter of the single method in a one-class file.
fn only_param(src: &str) -> Param {
    let members = members(src);
    let [member] = &members[..] else {
        panic!("expected one member: {members:?}");
    };
    let ClassMemberKind::Method(m) = &member.kind else {
        panic!("expected a method: {member:?}");
    };
    let [param] = &m.params[..] else {
        panic!("expected one parameter: {m:?}");
    };
    param.clone()
}

/// The one property of a one-class file.
fn only_property(src: &str) -> PropertyMember {
    let members = members(src);
    let [member] = &members[..] else {
        panic!("expected one member: {members:?}");
    };
    let ClassMemberKind::Property(p) = &member.kind else {
        panic!("expected a property: {member:?}");
    };
    p.clone()
}

/// The class name a parameter's or a property's type names.
fn named(ty: &Type, src: &str) -> String {
    let at = ty.span.start as usize..ty.span.end as usize;
    src[at].to_string()
}

#[test]
fn covariant_parses_before_a_parameters_type_in_each_form() {
    let src = "<?nvs class A { public function feed(covariant Dog $a): void {} }";
    let p = only_param(src);
    assert!(p.covariant);
    assert_eq!(named(p.ty.as_ref().unwrap(), src), "Dog");

    let p = only_param("<?nvs class A { public function feed(inout covariant Dog $a): void {} }");
    assert!(p.covariant && p.inout);

    let p = only_param("<?nvs class A { public function feed(covariant Dog ...$all): void {} }");
    assert!(p.covariant && p.variadic);

    let p = only_param("<?nvs class A { public function feed(covariant ?Dog $a = null): void {} }");
    assert!(p.covariant && p.default.is_some());

    let p =
        only_param("<?nvs class A { public function constructor(public covariant Dog $pet) {} }");
    assert!(p.covariant);
    assert_eq!(p.modifiers, vec![Modifier::Public]);

    let p = only_param("<?nvs class A { public function feed(Dog $a): void {} }");
    assert!(!p.covariant);
}

#[test]
fn covariant_parses_before_a_propertys_type() {
    let src = "<?nvs class A { public covariant Dog $pet; }";
    let p = only_property(src);
    assert!(p.covariant);
    assert_eq!(named(&p.ty, src), "Dog");
    assert_eq!(p.modifiers, vec![Modifier::Public]);

    let p = only_property("<?nvs class A { public covariant Dog|Cat $pet { get => $this->pet; } }");
    assert!(p.covariant && p.hooks.is_some());

    let p = only_property("<?nvs class A { public Dog $pet; }");
    assert!(!p.covariant);
}

#[test]
fn covariant_stays_an_identifier_everywhere_else() {
    // A method, a variable and a call keep the word as a plain name.
    let members = members(
        "<?nvs class A { public function covariant(int $covariant): int { return $this->covariant($covariant); } }",
    );
    let [member] = &members[..] else {
        panic!("expected one member: {members:?}");
    };
    let ClassMemberKind::Method(m) = &member.kind else {
        panic!("expected a method: {member:?}");
    };
    assert!(!m.params[0].covariant);

    // Followed by the parameter's name, the word is the type itself.
    let src = "<?nvs class A { public function feed(covariant $a): void {} }";
    let p = only_param(src);
    assert!(!p.covariant);
    assert_eq!(named(p.ty.as_ref().unwrap(), src), "covariant");
}

#[test]
fn covariant_in_front_of_a_modifier_is_reported_and_still_counts() {
    for src in [
        "<?nvs class A { covariant public Dog $pet; }",
        "<?nvs class A { public function feed(covariant inout Dog $a): void {} }",
    ] {
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", src.to_string());
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        let bad: Vec<_> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_BAD_MODIFIER))
            .collect();
        assert_eq!(bad.len(), 1, "{src}: {diags:?}");
        assert_eq!(diags.iter().count(), 1, "{src}: {diags:?}");
        let StmtKind::ClassDecl(decl) = &stmts[0].kind else {
            panic!("expected a class");
        };
        let covariant = match &decl.members[0].kind {
            ClassMemberKind::Property(p) => p.covariant,
            ClassMemberKind::Method(m) => m.params[0].covariant,
            other => panic!("unexpected member {other:?}"),
        };
        assert!(covariant, "{src}");
    }
}
