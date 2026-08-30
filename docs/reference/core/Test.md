---
summary: the typed assertion roster a `#[Test]` method calls — what PHPUnit's `assert*` family becomes when testing is part of the language
keywords: PHPUnit, assert(), assertion, unit test, #[Test], #[Core\Test], Core\Test\Failure, nvs test, expectException, assertSame, assertEquals, ledger, fixed clock
---

`Core\Test` is the assertion surface: every member is `static`, takes the subject **first**
(`assertEquals($actual, $expected)` — the reverse of PHPUnit's order), and is generic, so comparing an
`int` with a `string` is a compile error rather than a failed test. A failed assertion throws
`Core\Test\Failure`, an ordinary `Throwable` a `catch` may take — but the runner keeps its own ledger of
what held and what failed, and a `catch` does not erase an entry; `expectFailure` is the one member that
discharges one. A test is a `public`, non-`static` method returning `void` and marked `#[Core\Test]` (or
`#[Test]` after `use Core\Test;`), run by `nvs test`; a test that asserts nothing fails. Marking, fixtures,
data rows and the runner's report belong to [testing](#lang-testing).

```nvs test
<?nvs
final class Cart {
    private array<int> $prices = [];

    public function add(int $price): void {
        if ($price < 0) {
            throw new LogicError("a price is never negative");
        }
        $this->prices = Core\Arr::append($this->prices, $price);
    }

    public function total(): int {
        return Core\Arr::sum($this->prices) as int;
    }

    public function lines(): array<int> {
        return $this->prices;
    }
}

final class CartTest {
    #[Core\Test]
    public function sumsWhatWasAdded(): void {
        var $cart = new Cart();
        $cart->add(3);
        $cart->add(4);
        Core\Test::assertSame($cart->total(), 7);
        Core\Test::assertCount($cart->lines(), 2, {message: "two lines were added"});
    }

    #[Core\Test]
    public function refusesANegativePrice(): void {
        var $cart = new Cart();
        Core\Test::assertThrows(fn (): void => { $cart->add(-1); }, LogicError::class);
        Core\Test::assertTrue($cart->total() == 0);
    }
}
```
```output
  CartTest
    ✓ sumsWhatWasAdded
    ✓ refusesANegativePrice
  0 failed, 2 passed
```
