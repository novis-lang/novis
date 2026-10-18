`public`, `private` or `protected` before the enum is the error `E0133`. The cases leave the class in
every value it returns, so a visibility could only hide the name.

A subclass does not inherit the enum. `Refund::Status`, where `Refund` extends `Order`, is the error
`E0309`, and its help names `Order::Status`. Inside `Refund`, the name `Status` alone is the error
`E0303`. The same is true in a class that implements an interface with an enum. Write
`Order::Status` or `Job::Result` there.

`self::Status` and `static::Status` are the error `E0135`. Inside the class that declares the enum,
write `Status`.

`use Shop\Order::Status;` is the error `E0212`. Write `use Shop\Order;` and then `Order::Status`.

An enum, a class constant and a `type` alias in one class must have different names. Two of them
with the same name are the error `E0304`. Two enums with the same name in one class are `E0304` too.

An enum body does not declare an enum (`E0220`). An `if` block does not declare one either
(`E0233`). A class body declares an enum or a `type` alias, but not a class or an interface
(`E0233`).

An enum in a class may have the same name as an enum outside the classes. Inside the class, the
name means the class's own enum.

`Order::Status::class` gives the full name of the enum. In the namespace `Shop`, that is
`Shop\Order::Status`. The enum `Status` in the class `Shop\Order` and a class `Status` in the
namespace `Shop\Order` are two different types. `Shop\Order::Status` names the enum, and
`Shop\Order\Status` names the class.

Two classes may each declare an enum with the same name. They are two different enums.
`A::Kind::X == B::Kind::X` is the error `E0466`.
