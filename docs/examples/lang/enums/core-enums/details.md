A number where a member needs a `Core` enum is the error `E0401`, such as `{ order: 1 }` for
`Core\Arr::sort`. A case of another enum is `E0401` too. `Core\Arr::Order::Desc` passed to
`Core\Decimal::divRound` needs a `Core\RoundMode`, so it does not compile.

A case the enum does not have, such as `Core\Arr::Order::Random`, is the error `E0405`.

`7 as Core\Arr::Order` throws a `RuntimeError` when no case has the number `7`. The message names
every case: `` `7` is not one of `Core\Arr::Order::Asc`, `Core\Arr::Order::Desc` ``.
`7 as ?Core\Arr::Order` gives `null`.

A case of one `Core` enum compared with a case of another is the error `E0466`.

`use Core\Arr;` lets you write `Arr::Order::Desc`. `use Core\Arr::Order;` is the error `E0212`,
because an import names a class and not the enum inside it.

You may declare your own enum named `Order`. It is a different enum from `Core\Arr::Order`.

`Core\Json::encode` and `Core\Debug::dump` write a case of a `Core` enum as its number.
