Returns the one value of an attribute as a `Core\Uuid`, such as an entry's `objectGUID`.

Every entry in Active Directory has an `objectGUID`. It does not change when the entry is renamed
or moved, so it is a good key for an entry in your own database. The directory sends it as 16
bytes, in an order that is different from the usual UUID text. `uuid` reads those bytes and returns
the `Core\Uuid` that Windows shows for the same entry.

If the entry has no value for the attribute, the result is `null`. For an attribute with several
values, `uuid` throws a `LogicError`. It also throws a `LogicError` when the value is not 16 bytes,
and the message names the attribute.
