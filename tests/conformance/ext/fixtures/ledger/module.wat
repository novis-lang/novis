;; The fixture extension `Shop\Ledger` as a core module. Its exports follow the canonical ABI's
;; names, and the packer componentizes it against `api.wit`.
;;
;; Every export returns `result<T, error>`, which flattens to more than one value, so each one
;; writes its result to the return area at `$ret` and returns that address. A result's case is the
;; byte at `$ret`; the payload follows at its alignment, 4 for a 32-bit payload and 8 for a 64-bit
;; one.
(module
  (import "nvs:ext/types@1.0.0" "[method]value.kind" (func $kind (param i32) (result i32)))
  (import "nvs:ext/types@1.0.0" "[resource-drop]value" (func $drop (param i32)))

  (memory (export "memory") 1)
  ;; `fetch`'s answer.
  (data (i32.const 64) "remote")

  ;; The return area: 32 bytes at 16, 8-aligned.
  (global $ret i32 (i32.const 16))
  ;; The next free byte. Nothing is freed: an instance lives for one request.
  (global $bump (mut i32) (i32.const 1024))

  (func (export "cabi_realloc")
    (param $old i32) (param $old_size i32) (param $align i32) (param $size i32) (result i32)
    (local $ptr i32) (local $end i32)
    (local.set $ptr
      (i32.and
        (i32.sub (i32.add (global.get $bump) (local.get $align)) (i32.const 1))
        (i32.sub (i32.const 0) (local.get $align))))
    (local.set $end (i32.add (local.get $ptr) (local.get $size)))
    (block $done
      (loop $grow
        (br_if $done
          (i32.le_u (local.get $end) (i32.shl (memory.size) (i32.const 16))))
        (if (i32.eq (memory.grow (i32.const 1)) (i32.const -1))
          (then unreachable))
        (br $grow)))
    (global.set $bump (local.get $end))
    (if (local.get $old)
      (then (memory.copy (local.get $ptr) (local.get $old) (local.get $old_size))))
    (local.get $ptr))

  ;; The `ok` case, its payload still to be written.
  (func $ok (result i32)
    (i32.store8 (global.get $ret) (i32.const 0))
    (global.get $ret))

  ;; The `ok` case of a pointer-and-length payload: a string or a list.
  (func $ok_pair (param $ptr i32) (param $len i32) (result i32)
    (i32.store offset=4 (global.get $ret) (local.get $ptr))
    (i32.store offset=8 (global.get $ret) (local.get $len))
    (call $ok))

  (func (export "shop:ledger/api#echo-bool") (param $flag i32) (result i32)
    (i32.store8 offset=4 (global.get $ret) (local.get $flag))
    (call $ok))

  (func (export "shop:ledger/api#echo-int") (param $number i64) (result i32)
    (i64.store offset=8 (global.get $ret) (local.get $number))
    (call $ok))

  (func (export "shop:ledger/api#echo-uint") (param $count i64) (result i32)
    (i64.store offset=8 (global.get $ret) (local.get $count))
    (call $ok))

  (func (export "shop:ledger/api#echo-float") (param $amount f64) (result i32)
    (f64.store offset=8 (global.get $ret) (local.get $amount))
    (call $ok))

  (func (export "shop:ledger/api#echo-string") (param $ptr i32) (param $len i32) (result i32)
    (call $ok_pair (local.get $ptr) (local.get $len)))

  (func (export "shop:ledger/api#echo-bytes") (param $ptr i32) (param $len i32) (result i32)
    (call $ok_pair (local.get $ptr) (local.get $len)))

  (func (export "shop:ledger/api#echo-list") (param $ptr i32) (param $len i32) (result i32)
    (call $ok_pair (local.get $ptr) (local.get $len)))

  (func (export "shop:ledger/api#echo-map") (param $ptr i32) (param $len i32) (result i32)
    (call $ok_pair (local.get $ptr) (local.get $len)))

  ;; `item` is `name` at 0 and `note`, an `option<string>`, at 8: its case, then the string at 12.
  (func (export "shop:ledger/api#echo-item")
    (param $name i32) (param $name_len i32)
    (param $has_note i32) (param $note i32) (param $note_len i32)
    (result i32)
    (i32.store offset=4 (global.get $ret) (local.get $name))
    (i32.store offset=8 (global.get $ret) (local.get $name_len))
    (i32.store8 offset=12 (global.get $ret) (local.get $has_note))
    (i32.store offset=16 (global.get $ret) (local.get $note))
    (i32.store offset=20 (global.get $ret) (local.get $note_len))
    (call $ok))

  ;; `option<s64>` is its case at 0 and the number at 8, so at 8 and 16 of the result.
  (func (export "shop:ledger/api#echo-optional")
    (param $some i32) (param $number i64) (result i32)
    (i32.store8 offset=8 (global.get $ret) (local.get $some))
    (i64.store offset=16 (global.get $ret) (local.get $number))
    (call $ok))

  ;; `area` is its case at 0 and the case's fields, each an `f64`, from 8: so at 8, 16 and 24 of
  ;; the result. A `circle` has only the first field.
  (func (export "shop:ledger/api#echo-area")
    (param $case i32) (param $first f64) (param $second f64) (result i32)
    (i32.store8 offset=8 (global.get $ret) (local.get $case))
    (f64.store offset=16 (global.get $ret) (local.get $first))
    (f64.store offset=24 (global.get $ret) (local.get $second))
    (call $ok))

  ;; `instant` is `seconds` at 0 and `nanos` at 8, so at 8 and 16 of the result.
  (func (export "shop:ledger/api#echo-instant") (param $seconds i64) (param $nanos i64) (result i32)
    (i64.store offset=8 (global.get $ret) (local.get $seconds))
    (i64.store offset=16 (global.get $ret) (local.get $nanos))
    (call $ok))

  ;; `unit` is its case's index, one byte, stored where a `bool` is.
  (func (export "shop:ledger/api#echo-unit") (param $case i32) (result i32)
    (i32.store8 offset=4 (global.get $ret) (local.get $case))
    (call $ok))

  ;; `uri` is its text alone, so it is returned as a string is.
  (func (export "shop:ledger/api#echo-uri") (param $ptr i32) (param $len i32) (result i32)
    (call $ok_pair (local.get $ptr) (local.get $len)))

  ;; A borrowed handle is dropped before the export returns, as the canonical ABI requires.
  (func (export "shop:ledger/api#kind-of") (param $handle i32) (result i32)
    (i64.store offset=8 (global.get $ret)
      (i64.extend_i32_u (call $kind (local.get $handle))))
    (call $drop (local.get $handle))
    (call $ok))

  (func (export "shop:ledger/api#write-line") (param $ptr i32) (param $len i32) (result i32)
    (call $ok))

  (func (export "shop:ledger/api#fetch") (result i32)
    (call $ok_pair (i32.const 64) (i32.const 6)))

  ;; The `err` case: `error`'s own case at 4 and its string at 8.
  (func (export "shop:ledger/api#fail")
    (param $kind i64) (param $ptr i32) (param $len i32) (result i32)
    (i32.store8 (global.get $ret) (i32.const 1))
    (i32.store8 offset=4 (global.get $ret) (i32.wrap_i64 (local.get $kind)))
    (i32.store offset=8 (global.get $ret) (local.get $ptr))
    (i32.store offset=12 (global.get $ret) (local.get $len))
    (global.get $ret))

  (func (export "shop:ledger/api#crash") (result i32)
    unreachable))
