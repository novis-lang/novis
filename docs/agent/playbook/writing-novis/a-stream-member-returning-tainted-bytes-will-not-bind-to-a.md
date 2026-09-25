- **A stream member returning `tainted bytes` will not bind to a `bytes` local, and
  `Core\Hash::equals` refuses it as well.** `Core\Compress\Decompressor::finish` is declared
  `CoreTy::TaintedBytes` while the static `Core\Compress::decompress` answers plain `bytes`, so an
  example written from the static member's file fails `E0401` twice — once on the binding and once
  on the comparison it copied. Declare the local `tainted bytes` and compare two of them with `==`,
  which is not a sink; read the registry row's `return_ty` before reusing a sibling member's
  example. [until: gone crates/nvs-stdlib/src/compress.rs:Decompressor::finish]
