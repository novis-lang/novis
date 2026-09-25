- **An `--ORACLE--` case cannot call `mb_*`: the `php` on the Windows `PATH` has no `mbstring`.**
  `Core\Str` counts grapheme clusters (`nvs_stdlib::granularity::DEFAULT` is `Unit::Grapheme`) where
  `substr` counts bytes and `mb_substr` code points, and the three coincide only inside ASCII with
  no carriage return. Put the ASCII agreement in one `--ORACLE--` file and the multibyte split in an
  `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--`. [until: gone crates/nvs-stdlib/src/granularity.rs:Unit::Grapheme]
