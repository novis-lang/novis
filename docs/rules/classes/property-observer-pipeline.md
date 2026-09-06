Every read or write of a property on a class implementing `PropertyObserver` runs two steps in one
fixed order. First the value is produced or committed exactly as it would be without the interface in
the picture — by the property's own hook if it declares one, by plain field storage otherwise. Then
the settled value is passed to `onPropertyGet` or `onPropertySet`. A caller receives what the first
step produced; a slot holds what the first step committed, which for a transforming `set` hook is not
the caller's own argument.

This is a pipeline and not a fallback: a hooked property is not exempt, and a hookless one still
reaches the observer. An observer that throws still fails the access, propagated like any other call
(`rule:errors/propagation`).

Three boundaries follow from writing it about a receiver. An access inside the property's own hooks
is the backing slot and carries no observer, or one write would be reported twice. A `static`
property has no receiving instance and reaches nothing. An observer that touches a property of its own
class recurses, exactly as any method calling itself does — there is no re-entry guard, because a
guard would be a second rule about which write is the real one.
