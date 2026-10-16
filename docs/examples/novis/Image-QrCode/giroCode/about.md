Makes the text of a GiroCode: a QR code on an invoice that a banking app reads to fill in a bank transfer.

`QrCode::giroCode` takes the details of one SEPA transfer in euros and returns them as text. Give the
text to `QrCode::render` or `QrCode::svg` to draw the QR code. Many banking apps in Europe read it.

The details are fields of one object. `name` is who receives the money, and `iban` is their IBAN. These
two are needed, and spaces in the IBAN are removed. `amount` is a `decimal` in euros. `bic` is the BIC of
the bank. `reference` is a creditor reference, such as `RF18539007547034`. `text` is a free text.

These throw a `LogicError`: a wrong IBAN, a field that is too long or has a line break, an amount with
more than 2 decimals, and both a `reference` and a `text`.

**Good to know:** the IBAN check finds most typing mistakes. It does not check that the account exists.

**The examples below** show the text of a payment, a GiroCode on an invoice, and the errors.
