`nvs info` prints information about the `nvs` binary and the computer it runs on.

The **Build** part has the version, the commit, the build profile and the target platform. The
**Host** part has the operating system, the architecture, the number of processor threads and the
path of the executable. The **Licensing** part has the license of `nvs`, which is MIT, and every
third-party component with its version and its license. `--licenses` adds the full text of every
license.

**Good to know:** this is the information that `php -i` prints for PHP. `nvs -i` does not exist,
and it ends with exit status `2`.
