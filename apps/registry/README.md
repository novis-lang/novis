# Registry

The package registry of Novis, written in Novis. An account is one or more passkeys and ten recovery
codes. It has no email and no password. The data is kept in SQLite.

Run these commands in this folder:

```sh
nvs serve public/index.nvs --config nvs.toml   # open http://localhost:8000/
nvs test . --config nvs.toml                   # runs the tests
```

`hosting.json` has the settings of one installation. `origin` is the address of the site, and
passkeys work only on it. `signUpWork` is the number of zero bits the proof-of-work of a sign-up
needs. Each extra bit doubles the work. The file in this folder is for one computer, and a
deployment writes its own.
