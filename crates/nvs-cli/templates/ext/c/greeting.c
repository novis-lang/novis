// The code of the extension. Novis calls `hello` as `Example\Greeting::hello`.
// `greeting.h` is written by `wit-bindgen c`. README.md has the command.

#include <stdlib.h>
#include <string.h>

#include "greeting.h"

// `hello` returns a greeting for `name`. An empty name returns an error,
// and the Novis program gets a `LogicError`.
bool exports_example_greeting_api_hello(greeting_string_t *name,
                                        greeting_string_t *ret,
                                        exports_example_greeting_api_error_t *err) {
    // This function owns `name`, so it frees `name` before it returns.
    if (name->len == 0) {
        greeting_string_free(name);
        err->tag = NVS_EXT_TYPES_ERROR_INVALID;
        greeting_string_dup(&err->val.invalid, "the name is empty");
        return false;
    }
    // "Hello, " is 7 bytes, and "!" is 1 more.
    size_t len = name->len + 8;
    uint8_t *text = malloc(len);
    memcpy(text, "Hello, ", 7);
    memcpy(text + 7, name->ptr, name->len);
    text[len - 1] = '!';
    greeting_string_free(name);
    ret->ptr = text;
    ret->len = len;
    return true;
}

// `read_all` copies a `bytes` parameter into `out`. Novis keeps the bytes,
// and `read` copies all of them in one call. Free `out` with `greeting_list_u8_free`.
static void read_all(nvs_ext_types_borrow_bytes_t data, greeting_list_u8_t *out) {
    nvs_ext_types_method_bytes_read(data, out);
}

// `bytes_from` makes the `bytes` that a function returns. It adds the bytes
// in parts of 1 MiB. Novis counts each part against the memory limit of the request.
static nvs_ext_types_own_bytes_t bytes_from(uint8_t *ptr, size_t len) {
    const size_t part_size = 1024 * 1024;
    nvs_ext_types_own_bytes_t bytes = nvs_ext_types_constructor_bytes();
    nvs_ext_types_borrow_bytes_t to = nvs_ext_types_borrow_bytes(bytes);
    for (size_t at = 0; at < len; at += part_size) {
        greeting_list_u8_t part = {ptr + at, len - at < part_size ? len - at : part_size};
        nvs_ext_types_method_bytes_append(to, &part);
    }
    return bytes;
}

// `reverse` returns the bytes of `data` in reverse order.
bool exports_example_greeting_api_reverse(exports_example_greeting_api_borrow_bytes_t data,
                                          exports_example_greeting_api_own_bytes_t *ret,
                                          exports_example_greeting_api_error_t *err) {
    (void)err;
    greeting_list_u8_t bytes;
    read_all(data, &bytes);
    // This function must drop `data` before it returns.
    nvs_ext_types_bytes_drop_borrow(data);
    for (size_t i = 0, j = bytes.len; i + 1 < j; i++, j--) {
        uint8_t byte = bytes.ptr[i];
        bytes.ptr[i] = bytes.ptr[j - 1];
        bytes.ptr[j - 1] = byte;
    }
    *ret = bytes_from(bytes.ptr, bytes.len);
    greeting_list_u8_free(&bytes);
    return true;
}
