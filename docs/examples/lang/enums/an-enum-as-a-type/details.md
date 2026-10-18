A variable of the type `Suit` accepts only the cases of `Suit`. A number, a case of another enum and
an `array<Suit>` element of another type are the error `E0401`. A `foreach` variable declared with
another enum is `E0401` too. A method that declares `Suit` and returns a number is the error `E0403`.

`var $s = Suit::Hearts;` gives `$s` the type `Suit`. Assigning `1` to it later is `E0401`.

A property of an enum type has no default value. Give it one, such as `public Suit $suit =
Suit::Hearts;`, or assign it in the constructor. Without either, the class does not compile
(`E0409`). A parameter may have a case as its default value: `Suit $suit = Suit::Hearts`.

`?Suit` accepts a case or `null`.

`Core\Json::encode` writes an `array<Suit>` as an array of numbers: `[0,1]`.
