Adds up all the values of an array and divides the total by how many entries there are. That is the
mean.

An array with no entries gives you `null`. There is nothing to divide, so there is no mean.

A `decimal` anywhere in the array makes the result a `decimal`, which stays exact and is the type to
use for money. Every other array gives you a `float`. A `decimal` result carries as many digits as
the type holds, and the last digit is rounded.

The total is built the way `Core\Arr::sum` builds it. A total that grows past the range a whole
number covers throws an error, and a `float` and a `decimal` in the same array throw an error too.

The examples show the mean of a list of whole numbers, an average price that stays exact, and the
days a shop was busier than usual.
