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
