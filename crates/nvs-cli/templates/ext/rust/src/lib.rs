// The code of the extension. Novis calls `hello` as `Example\Greeting::hello`.

// This macro reads the WIT files in `wit/` and writes the Rust types for them.
// `generate_all` also writes the types of the `nvs:ext` world and of WASI.
wit_bindgen::generate!({
    path: "wit",
    world: "greeting",
    generate_all,
});

use exports::example::greeting::api::{Error, Guest};
use nvs::ext::types::Bytes;

struct Greeting;

impl Guest for Greeting {
    // `hello` returns a greeting for `name`. An empty name returns an error,
    // and the Novis program gets a `LogicError`.
    fn hello(name: String) -> Result<String, Error> {
        if name.is_empty() {
            return Err(Error::Invalid("the name is empty".to_string()));
        }
        Ok(format!("Hello, {name}!"))
    }

    // `reverse` returns the bytes of `data` in reverse order.
    fn reverse(data: &Bytes) -> Result<Bytes, Error> {
        let mut bytes = read_all(data);
        bytes.reverse();
        Ok(bytes_from(&bytes))
    }
}

// `read_all` copies a `bytes` parameter into a `Vec<u8>`.
// Novis keeps the bytes, and `read` copies all of them in one call.
fn read_all(data: &Bytes) -> Vec<u8> {
    data.read()
}

// `bytes_from` makes the `bytes` that a function returns.
// It adds `data` in parts of 1 MiB. Novis counts each part against the
// memory limit of the request.
fn bytes_from(data: &[u8]) -> Bytes {
    let bytes = Bytes::new();
    for part in data.chunks(1024 * 1024) {
        bytes.append(part);
    }
    bytes
}

export!(Greeting);
