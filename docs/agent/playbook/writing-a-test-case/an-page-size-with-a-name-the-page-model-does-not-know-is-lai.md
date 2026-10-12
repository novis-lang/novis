- **An `@page` `size` with a name the page model does not know is laid out as A4, with no
  error.** `paginate::named` holds CSS's list (A3 to A5, B4, B5, letter, legal, ledger), so
  `size: A6` gives an A4 page and a test that counts pages fails as if the layout were wrong.
  Give a small page as two lengths, `size: 105mm 148mm`, as `extensions/pdf/tests/tables.rs` does.
