// The code of the extension. Novis calls `hello` as `Example\Greeting::hello`.

// This macro reads the WIT files in `wit/` and writes the Rust types for them.
// `generate_all` also writes the types of the `nvs:ext` world and of WASI.
wit_bindgen::generate!({
    path: "wit",
    world: "greeting",
    generate_all,
});

use exports::example::greeting::api::{Error, Guest};

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
}

export!(Greeting);
