;; `Geo\Atlas`, the extension the proof programs of `extension-grants-what-a-component-may-reach`
;; call. A core module written against the canonical ABI; the packer makes it a component.
;;
;; Every export returns `result<T, error>`, written to the return area at `$ret`: the case byte at
;; `$ret`, and both payloads at 4, or at 8 when `T` is 64-bit. A refusal is the `runtime` case of `error`
;; carrying one of the messages below, so it throws a `RuntimeError` in the program.
(module
  (import "wasi:filesystem/preopens@0.2.12" "get-directories" (func $dirs (param i32)))
  (import "wasi:filesystem/types@0.2.12" "[method]descriptor.open-at"
    (func $open (param i32 i32 i32 i32 i32 i32 i32)))
  (import "wasi:filesystem/types@0.2.12" "[method]descriptor.read"
    (func $read (param i32 i64 i64 i32)))
  (import "wasi:filesystem/types@0.2.12" "[method]descriptor.write"
    (func $write (param i32 i32 i32 i64 i32)))
  (import "wasi:filesystem/types@0.2.12" "[resource-drop]descriptor" (func $drop (param i32)))
  (import "wasi:io/poll@0.2.12" "poll" (func $poll (param i32 i32 i32)))
  (import "wasi:http/types@0.2.12" "[constructor]fields" (func $fields (result i32)))
  (import "wasi:http/types@0.2.12" "[constructor]outgoing-request"
    (func $request (param i32) (result i32)))
  (import "wasi:http/types@0.2.12" "[method]outgoing-request.set-scheme"
    (func $scheme (param i32 i32 i32 i32 i32) (result i32)))
  (import "wasi:http/types@0.2.12" "[method]outgoing-request.set-authority"
    (func $authority (param i32 i32 i32 i32) (result i32)))
  (import "wasi:http/types@0.2.12" "[method]future-incoming-response.subscribe"
    (func $subscribe (param i32) (result i32)))
  (import "wasi:http/types@0.2.12" "[method]future-incoming-response.get"
    (func $get (param i32 i32)))
  (import "wasi:http/types@0.2.12" "[method]incoming-response.status"
    (func $status (param i32) (result i32)))
  (import "wasi:http/outgoing-handler@0.2.12" "handle"
    (func $handle (param i32 i32 i32 i32)))

  (memory (export "memory") 1)

  ;; The messages a refusal carries, each at its own address.
  (data (i32.const 256) "no folder is granted")             ;; 20 bytes
  (data (i32.const 288) "not permitted")                    ;; 13
  (data (i32.const 320) "the folder is read-only")          ;; 23
  (data (i32.const 352) "no such file")                     ;; 12
  (data (i32.const 384) "the file could not be opened")     ;; 28
  (data (i32.const 416) "the request was denied")           ;; 22
  (data (i32.const 448) "the request failed")               ;; 18

  ;; The return area: 24 bytes at 16, 8-aligned.
  (global $ret i32 (i32.const 16))
  ;; The return areas of the imports: `get-directories` at 512, `open-at` at 528, `read` at 544,
  ;; `write` at 576, `handle` at 600, `poll`'s list at 640 and its answer at 648, and the
  ;; response future's `get` at 704.
  ;; The next free byte. Nothing is freed: an instance lives for one request.
  (global $heap (mut i32) (i32.const 4096))

  (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) (local $at i32)
    (local.set $at
      (i32.and
        (i32.add (global.get $heap) (i32.sub (local.get 2) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get 2))))
    (global.set $heap (i32.add (local.get $at) (local.get 3)))
    (block $done
      (loop $grow
        (br_if $done
          (i32.le_u (global.get $heap) (i32.shl (memory.size) (i32.const 16))))
        (if (i32.eq (memory.grow (i32.const 1)) (i32.const -1)) (then unreachable))
        (br $grow)))
    (local.get $at))

  ;; The `runtime` case of `error`, carrying the message at `$ptr`, as the payload at `$at` bytes
  ;; into the return area: 4 when the `ok` payload is 32-bit or absent, 8 when it is 64-bit.
  (func $fail_at (param $ptr i32) (param $len i32) (param $at i32) (result i32)
    (local $payload i32)
    (local.set $payload (i32.add (global.get $ret) (local.get $at)))
    (i32.store8 (global.get $ret) (i32.const 1))
    (i32.store8 (local.get $payload) (i32.const 2))
    (i32.store offset=4 (local.get $payload) (local.get $ptr))
    (i32.store offset=8 (local.get $payload) (local.get $len))
    (global.get $ret))

  (func $fail (param $ptr i32) (param $len i32) (result i32)
    (call $fail_at (local.get $ptr) (local.get $len) (i32.const 4)))

  ;; The refusal for the filesystem's `error-code` case `$code`: `no-entry` is 20,
  ;; `not-permitted` 31 and `read-only` 33. -1 means there was no folder to try.
  (func $fs_fail (param $code i32) (result i32)
    (if (i32.eq (local.get $code) (i32.const -1))
      (then (return (call $fail (i32.const 256) (i32.const 20)))))
    (if (i32.eq (local.get $code) (i32.const 31))
      (then (return (call $fail (i32.const 288) (i32.const 13)))))
    (if (i32.eq (local.get $code) (i32.const 33))
      (then (return (call $fail (i32.const 320) (i32.const 23)))))
    (if (i32.eq (local.get $code) (i32.const 20))
      (then (return (call $fail (i32.const 352) (i32.const 12)))))
    (call $fail (i32.const 384) (i32.const 28)))

  ;; Opens `path` under each preopen in turn with `open-flags` `$how` and `descriptor-flags`
  ;; `$flags`, and returns the first descriptor opened. When none opens, it returns -1 and leaves
  ;; the last `error-code` at 1020, or -1 there when there was no preopen to try.
  (func $open_any (param $ptr i32) (param $len i32) (param $how i32) (param $flags i32)
    (result i32)
    (local $list i32) (local $n i32) (local $i i32)
    (call $dirs (i32.const 512))
    (local.set $list (i32.load (i32.const 512)))
    (local.set $n (i32.load (i32.const 516)))
    (i32.store (i32.const 1020) (i32.const -1))
    (block $tried
      (loop $next
        (br_if $tried (i32.ge_u (local.get $i) (local.get $n)))
        (call $open
          (i32.load (i32.add (local.get $list) (i32.mul (local.get $i) (i32.const 12))))
          (i32.const 1) (local.get $ptr) (local.get $len) (local.get $how) (local.get $flags)
          (i32.const 528))
        (if (i32.eqz (i32.load8_u (i32.const 528)))
          (then (return (i32.load (i32.const 532)))))
        (i32.store (i32.const 1020) (i32.load8_u (i32.const 532)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (i32.const -1))

  (func (export "geo:atlas/api#read") (param $ptr i32) (param $len i32) (result i32)
    (local $fd i32)
    (local.set $fd (call $open_any (local.get $ptr) (local.get $len) (i32.const 0) (i32.const 1)))
    (if (i32.eq (local.get $fd) (i32.const -1))
      (then (return (call $fs_fail (i32.load (i32.const 1020))))))
    (call $read (local.get $fd) (i64.const 65536) (i64.const 0) (i32.const 544))
    (call $drop (local.get $fd))
    (if (i32.load8_u (i32.const 544))
      (then (return (call $fs_fail (i32.load8_u (i32.const 548))))))
    (i32.store8 (global.get $ret) (i32.const 0))
    (i32.store offset=4 (global.get $ret) (i32.load (i32.const 548)))
    (i32.store offset=8 (global.get $ret) (i32.load (i32.const 552)))
    (global.get $ret))

  (func (export "geo:atlas/api#write")
    (param $ptr i32) (param $len i32) (param $text i32) (param $size i32) (result i32)
    (local $fd i32)
    (local.set $fd (call $open_any (local.get $ptr) (local.get $len) (i32.const 9) (i32.const 2)))
    (if (i32.eq (local.get $fd) (i32.const -1))
      (then (return (call $fs_fail (i32.load (i32.const 1020))))))
    (call $write (local.get $fd) (local.get $text) (local.get $size) (i64.const 0) (i32.const 576))
    (call $drop (local.get $fd))
    (if (i32.load8_u (i32.const 576))
      (then (return (call $fs_fail (i32.load8_u (i32.const 584))))))
    (i32.store8 (global.get $ret) (i32.const 0))
    (global.get $ret))

  ;; The refusal for the HTTP `error-code` case `$code`: `HTTP-request-denied` is 15.
  (func $http_fail (param $code i32) (result i32)
    (if (result i32) (i32.eq (local.get $code) (i32.const 15))
      (then (call $fail_at (i32.const 416) (i32.const 22) (i32.const 8)))
      (else (call $fail_at (i32.const 448) (i32.const 18) (i32.const 8)))))

  (func (export "geo:atlas/api#status") (param $ptr i32) (param $len i32) (result i32)
    (local $req i32) (local $future i32)
    (local.set $req (call $request (call $fields)))
    (if (call $scheme (local.get $req) (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0))
      (then (return (call $http_fail (i32.const -1)))))
    (if (call $authority (local.get $req) (i32.const 1) (local.get $ptr) (local.get $len))
      (then (return (call $http_fail (i32.const -1)))))
    (call $handle (local.get $req) (i32.const 0) (i32.const 0) (i32.const 600))
    (if (i32.load8_u (i32.const 600))
      (then (return (call $http_fail (i32.load8_u (i32.const 608))))))
    (local.set $future (i32.load (i32.const 608)))
    (i32.store (i32.const 640) (call $subscribe (local.get $future)))
    (call $poll (i32.const 640) (i32.const 1) (i32.const 648))
    (call $get (local.get $future) (i32.const 704))
    (if (i32.eqz (i32.load8_u (i32.const 704)))
      (then (return (call $http_fail (i32.const -1)))))
    (if (i32.load8_u (i32.const 712))
      (then (return (call $http_fail (i32.const -1)))))
    (if (i32.load8_u (i32.const 720))
      (then (return (call $http_fail (i32.load8_u (i32.const 728))))))
    (i32.store8 (global.get $ret) (i32.const 0))
    (i64.store offset=8 (global.get $ret)
      (i64.extend_i32_u (call $status (i32.load (i32.const 728)))))
    (global.get $ret)))
