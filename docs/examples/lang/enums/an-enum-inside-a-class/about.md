Declares an enum in the body of the class or interface that uses it.

Write the enum in the body, next to the properties and methods. Inside the body, the enum is
`Status` and a case is `Status::Paid`. Everywhere else, the enum is `Order::Status` and a case is
`Order::Status::Paid`. A case is a whole number, the same as in an enum outside a class.

The enum takes no `public` or `private`. A subclass does not inherit it, and a class that
implements an interface does not inherit the interface's enum. Write `Status` inside the owner and
`Order::Status` everywhere else. `self::Status` and `use Shop\Order::Status;` do not compile. An
enum that several classes use is declared outside the classes.

**The examples below** show an order that keeps its status, a command line program with its own exit
codes, and an interface whose enum every class that implements it returns. Then three programs that
do not compile: an enum with `private`, a subclass that names its parent's enum, and a `use` of an
enum.
