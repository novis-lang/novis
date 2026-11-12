;; The attack fixture `Geo\Notes` as a core module. Its export follows the canonical ABI's names,
;; and the packer componentizes it against `api.wit`. The extension exists for the Novis source it
;; carries, so its one method only returns a number.
;;
;; `version` returns `result<s64, error>`, which flattens to more than one value, so it writes the
;; `ok` case to the return area at `$ret` and returns that address.
(module
  (memory (export "memory") 1)

  ;; The return area: 16 bytes at 16, 8-aligned. The case is at 0 and the `s64` at 8.
  (global $ret i32 (i32.const 16))

  ;; Nothing the interface passes needs memory from the guest, so nothing is ever allocated.
  (func (export "cabi_realloc")
    (param $old i32) (param $old_size i32) (param $align i32) (param $size i32) (result i32)
    unreachable)

  ;; Returns `1`.
  (func (export "geo:notes/api#version") (result i32)
    (i32.store8 (global.get $ret) (i32.const 0))
    (i64.store (i32.add (global.get $ret) (i32.const 8)) (i64.const 1))
    (global.get $ret)))
