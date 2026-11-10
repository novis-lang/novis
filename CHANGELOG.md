# Changelog

Every released version of Novis, newest first. Generated from the commit log by
`bun nv release` and prepended by the release workflow -- edit a section only to correct it,
never to add one by hand.

What a version number promises is
[`rule:packaging/the-versioned-surface-is-enumerated`](docs/rules/packaging.md#packaging-the-versioned-surface-is-enumerated):
it covers the language, the `Core` library, `nvs.toml`, the CLI, diagnostic identity, the
extension ABI and `serialize()` output -- and explicitly not the Rust APIs of the `nvs-*` crates.
Before 1.0 the breaking slot moves left: `0.MINOR` carries breaking changes.

A release is cut by [the release workflow](.github/workflows/release.yml), fired by hand from the
Actions tab; [docs/release.md](docs/release.md) is the procedure and what it needs configured.

## [0.0.3] - 2026-11-10

### Features

- **apps**: apps/chat shows WebSockets and apps/poll shows server-sent events, each with its website page ([`13159de2`](https://github.com/novis-lang/novis/commit/13159de2aca6234e650a6a17e420e3a07b2a664f))
- **apps**: chat deploys with nvs.toml and one production.toml, as poll does ([`c9797093`](https://github.com/novis-lang/novis/commit/c97970936dd12510217e10e3c3a3a6664aad44a4))
- **apps**: poll's security policy is set in nvs.toml, for every response ([`082b8053`](https://github.com/novis-lang/novis/commit/082b8053758492fbd4216bcd44c00ebc3574ea6e))
- **ci**: a real release needs a green full CI run of main's tip, not a dry run ([`9c43636c`](https://github.com/novis-lang/novis/commit/9c43636c5d7c4e821c33e0a09fa338b3f37fe694))
- **cli**: NOVIS_PARSE_ONLY makes nvs check its command line and exit 0, running nothing ([`8f8f4da5`](https://github.com/novis-lang/novis/commit/8f8f4da5412aeffdffe10c87fd21be2eb80fbd2f))
- **cli**: a request file's body after --BODY-- may hold any bytes ([`41677edd`](https://github.com/novis-lang/novis/commit/41677edd59ec58970e368a96c8c8623985aca67a))
- **config**: an owner in Administrators passes the ownership check on Windows ([`13c4cc74`](https://github.com/novis-lang/novis/commit/13c4cc743c30b36ad4d2db0fc81d305954c97ef3))
- **config**: nvs init sets up the data folder and the project file, and offers to restrict its folder ([`bfff166e`](https://github.com/novis-lang/novis/commit/bfff166e7f03f93ddfb05181869568d274c75189))
- **config**: the folder above a checked folder fails only when another account can replace it ([`d6752d1a`](https://github.com/novis-lang/novis/commit/d6752d1a6d12fb82679bb11e146aff83c1e44d65))
- **csv**: Core\Csv::rows decodes a charset one chunk at a time ([`3db03f8a`](https://github.com/novis-lang/novis/commit/3db03f8ad0f77cc2b88e4a3b07e248dd752a7fdf))
- **csv**: Csv::rows reads any byte source, and its cells follow the source's taint ([`3e227c34`](https://github.com/novis-lang/novis/commit/3e227c3409764e7a53eb0f9b722e2f6ef71c7263))
- **deep**: bun nv deep runs the build legs and the release's version step ([`74105c5c`](https://github.com/novis-lang/novis/commit/74105c5c0f305900db25b8f85d86f9dc2e2570ac))
- **diagnostics**: nvs check --count and the editor index count the file-system work they did ([`0ee67eeb`](https://github.com/novis-lang/novis/commit/0ee67eeb7d1fdbdcc8041a49c6e260917909b449))
- **ext**: Core\Pdf and Core\Image read every byte source ([`03b9889a`](https://github.com/novis-lang/novis/commit/03b9889ac12d564344b9b95a8f8c39abd4fcb1fb))
- **format**: a literal Core\Str::format writes each integer placeholder into the result ([`a55599c0`](https://github.com/novis-lang/novis/commit/a55599c0d52f5d7a9ba7ae2d8431b8df67965552))
- **fuzz**: each fuzz target's check is a function cargo test replays over its seeds ([`2398bf57`](https://github.com/novis-lang/novis/commit/2398bf57b95617d48e0abe3bc04391598543244b))
- **html**: sanitize keeps the five value-checked attributes Markdown writes ([`fc0a1813`](https://github.com/novis-lang/novis/commit/fc0a181318e46dd1120e97fcc96a173caa8f7d38))
- **ide**: a file with routes shows R in the Explorer, and its tooltip names each route ([`ff916f38`](https://github.com/novis-lang/novis/commit/ff916f38ed5712b1bea4673b7c545fbf78fcbe5a))
- **io**: Core\IO and Core\IO\File read bytes, and write and append take bytes ([`21640d81`](https://github.com/novis-lang/novis/commit/21640d812e4f43e488c3b4b8baab6f16705a5749))
- **io**: Core\IO::stdinStream gives standard input as tainted bytes chunks ([`b67c68ad`](https://github.com/novis-lang/novis/commit/b67c68adb34f3dcd0b0a581b01b4399f28b3ba06))
- **ir**: interpolated strings and html templates join int and escaped holes as typed parts ([`ec835148`](https://github.com/novis-lang/novis/commit/ec8351480a3648d5fce37818896496498af17b86))
- **json**: encodeEach's pieces carry encode's taint, and a response streams clean ones ([`745de7b7`](https://github.com/novis-lang/novis/commit/745de7b75b3b8a3590b47a65bf5cbe0bb2f3d109))
- **json**: every JSON string Novis writes escapes the characters that end HTML markup or a script line ([`93d8f777`](https://github.com/novis-lang/novis/commit/93d8f777cf1e8cd1e9a3a4327d53608713b34d3c))
- **kit**: a usage wall holds only the login that met it ([`82162fdd`](https://github.com/novis-lang/novis/commit/82162fdd4b3c1641ac57c25112ba9fda82b6641e))
- **lsp**: a Core alias has a stub, a hover card and a completion row ([`78e67e9b`](https://github.com/novis-lang/novis/commit/78e67e9b51ff44e1ea326891f4a6a7f4e089a0ed))
- **lsp**: bun nv lsp-start times the editor's start and prints its index's work ([`e8969ddb`](https://github.com/novis-lang/novis/commit/e8969ddba396ddea4567f74d22a4a8ff226f48f3))
- **lsp**: nvs.inlayHints.parameterNames turns the parameter-name hints off without a restart ([`64250f15`](https://github.com/novis-lang/novis/commit/64250f150f3bed9074eeae599527e68869611071))
- **lsp**: nvs/redactions answers a docComment range per /// line to a client that declared the kind ([`110f8e3e`](https://github.com/novis-lang/novis/commit/110f8e3e2401996f00b01f8275600cf439dd5e21))
- **lsp**: the index build at start shows as work-done progress ([`74e43801`](https://github.com/novis-lang/novis/commit/74e43801cee11d82209dac9d8fdae6fa75979908))
- **markdown**: Core\Markdown::fromHtml writes Markdown from HTML ([`37789209`](https://github.com/novis-lang/novis/commit/37789209facbc7cde4e746ceaa8104be00389695))
- **markdown**: Core\Markdown::toHtml renders CommonMark and GFM through the sanitizer's list ([`6463e25a`](https://github.com/novis-lang/novis/commit/6463e25ac4ace0b6ccf905d74acdd85b236eb28a))
- **markdown**: Core\Markdown::toHtmlEach walks a source one block group per advance ([`da2bcdd5`](https://github.com/novis-lang/novis/commit/da2bcdd5c359b5d0ca47ccd4f9b20ecb2cf978fe))
- **markdown**: Core\Markdown\Live renders text that arrives in pieces ([`e1dc9472`](https://github.com/novis-lang/novis/commit/e1dc94726cf76376aaeeb1fceedcbefc21c828d1))
- **markdown**: a document is rendered in segments, and each reads the whole document's definitions ([`08bbe0a7`](https://github.com/novis-lang/novis/commit/08bbe0a7e022877c9e847cd2fa6206365b40ee79))
- **markdown**: a document renders one block group at a time, and the groups join to the whole HTML ([`1999047f`](https://github.com/novis-lang/novis/commit/1999047f3c171d63ddf7315a33b7a70b9e404475))
- **markdown**: fromHtml holds the whole tree, and a parse fed in pieces gives the same Markdown ([`40a8df38`](https://github.com/novis-lang/novis/commit/40a8df38bbe18dda921df24a9a719c283fa4519b))
- **markdown**: toHtml's link, image, heading and code callbacks change what a document renders ([`23e26c5a`](https://github.com/novis-lang/novis/commit/23e26c5a1bee69bc37d7510c340078fa4830e812))
- **nv**: `bun nv ci --check` runs CI's `rustdoc links` step before it pushes ([`ac8243fd`](https://github.com/novis-lang/novis/commit/ac8243fd3c224d838ecdd0159b23c84ff7a8ed7e))
- **nv**: `bun nv deep` runs every CI job this machine can run, and `bun nv fuzz --all` goes on after a crash ([`a2773216`](https://github.com/novis-lang/novis/commit/a27732169d6d42cae778eb98178e122ea2e3fe4f))
- **nv**: bun nv walkthrough takes pictures of a website walkthrough ([`bb31414d`](https://github.com/novis-lang/novis/commit/bb31414d14ef707ee4ee4d1cccc54a521c75d764))
- **nv**: every WSL step runs in its own test distro, which bun nv wsl makes and checks ([`cf18d84b`](https://github.com/novis-lang/novis/commit/cf18d84bd984867d0410e50e71fab7afaa99c61a))
- **nv**: proxy-validate checks nginx, Caddyfile and Dockerfile files in their pinned images ([`1d84fbaa`](https://github.com/novis-lang/novis/commit/1d84fbaa0ee5273654a3de5d64ab466111a2c340))
- **nv**: the WSL copy's HEAD is the checkout's HEAD, and deep leaves the db services up ([`661a58af`](https://github.com/novis-lang/novis/commit/661a58af3c19f5b269d1ad6f12b5247367b9c242))
- **pdf**: an img and an inline svg are inline boxes in their line ([`3dacbb53`](https://github.com/novis-lang/novis/commit/3dacbb53154d0983eca87303880cb9bda4b6de60))
- **proofs**: a member is named in its class's introduction when the class has one ([`c711b8a2`](https://github.com/novis-lang/novis/commit/c711b8a2433bf4c4e3ec55a68b224ee0c45c5ff4))
- **proofs**: a member that reads through byte_source.rs has its perf staled by it ([`b72f4595`](https://github.com/novis-lang/novis/commit/b72f4595b2c9df78fa24ca6eb9846943a2fed26b))
- **proofs**: a time stop is judged by the sweeps its run measured, not by a fixed slack ([`f7028f34`](https://github.com/novis-lang/novis/commit/f7028f346f20028d78933ee246612805bf65f7dc))
- **queue**: a Core\Queue\Id has toString, parse and tryParse ([`d4122775`](https://github.com/novis-lang/novis/commit/d4122775fa8d85c4f7a0a70f4cecd3165e262c78))
- **queue**: a job row carries a random token, and status, cancel and delete match it ([`5f82a493`](https://github.com/novis-lang/novis/commit/5f82a493ec0d455130cf8ca5d3a6e368650baa90))
- **queue**: every hand-made queue table has the token column ([`7a0a2df3`](https://github.com/novis-lang/novis/commit/7a0a2df309bc88a9121cc7bda7205175a2407ca8))
- **registry**: the package page shows each version's readme rendered once from Markdown ([`59536c04`](https://github.com/novis-lang/novis/commit/59536c04ef71a8f6bd9a8f1ef24ec4288ff4dd43))
- **release**: publishing a release lists the VS Code extension on Open VSX too ([`d76a075a`](https://github.com/novis-lang/novis/commit/d76a075aa91be9a41f66657ad3a3f2232941deda))
- **request**: Core\Request\BodyStream::peek gives the start of the body and consumes nothing ([`18af9b19`](https://github.com/novis-lang/novis/commit/18af9b19abfdfc6abffc5d8114466222747912b4))
- **request**: an upload part and a body stream give their start without being consumed ([`1aaad843`](https://github.com/novis-lang/novis/commit/1aaad843fd4f20efeb6504913df3bda1cee42dfe))
- **runtime**: a body written from an owned buffer takes it whole when the body is empty ([`e5b086f8`](https://github.com/novis-lang/novis/commit/e5b086f8adc302732cc6c8a577fe06dc163b9269))
- **runtime**: nvs_str_concat_parts joins strings, integers and escaped html text in one allocation ([`d9dd8d24`](https://github.com/novis-lang/novis/commit/d9dd8d245ef281c62254eaf4076512e92777424e))
- **runtime**: the request body's hold is shared through an Rc handle ([`cc257a8d`](https://github.com/novis-lang/novis/commit/cc257a8d03eb7e6c41a323878b60bb7bf9e75095))
- **scaling**: a constant bench's sibling is judged by what one operation costs ([`384d11e0`](https://github.com/novis-lang/novis/commit/384d11e0920ad2f44e7980715676e251275a19d2))
- **service**: a command that needs admin rights asks for them itself ([`7b1af697`](https://github.com/novis-lang/novis/commit/7b1af6976a26a19e7c63a4c9b9517f6804ea4410))
- **site**: a walkthrough scene a page shows counts as showing its editor's snippets ([`c35bd20e`](https://github.com/novis-lang/novis/commit/c35bd20e557f85d4ec3275620b4115f80cfbd171))
- **site**: every page with examples shows the nvs.toml they run with, once ([`2163c6b6`](https://github.com/novis-lang/novis/commit/2163c6b64a4470e12b8c332f52219c524dab048e))
- **site**: site --check code names every page code block no check runs ([`eccf3367`](https://github.com/novis-lang/novis/commit/eccf3367c3b9bc39c49e77b6e57978749cdeb795))
- **site**: site --check code parses every nvs line of a shell block, and each other block says why it is not run ([`26e366fd`](https://github.com/novis-lang/novis/commit/26e366fd591398e3811f10ba2383052dd7f484f5))
- **site**: site --check code reads every toml block with nvs config check, and parses json and yaml ([`3492132f`](https://github.com/novis-lang/novis/commit/3492132ff4744cf0ff2596a0655a9ac7ea5a4969))
- **site**: site --check code runs the command each txt block shows the output of, and marks each other one ([`4330bbcb`](https://github.com/novis-lang/novis/commit/4330bbcb1b9dcaf83a00e63019fed30cdb385c3e))
- **site**: site --check code validates nginx, Caddy, Dockerfile and rust blocks, and apps checks each deploy/ folder ([`a102d68b`](https://github.com/novis-lang/novis/commit/a102d68b81abb668d4cec63d2aa8f11ad545fc7e))
- **spreadsheet**: Reader::open takes a Core\Bytes::Source, a request body included ([`6f2d9be3`](https://github.com/novis-lang/novis/commit/6f2d9be3961b5c944ca04f35f95ae817e64f0525))
- **spreadsheet**: the reader has one CSV path, through Core\Csv::rows ([`46cc37ec`](https://github.com/novis-lang/novis/commit/46cc37ec5ead652c4a2be07fd99cade1ce8a385e))
- **stdlib**: Core\Hash, Core\Compress and Core\Storage::put read from every byte source ([`01fca0df`](https://github.com/novis-lang/novis/commit/01fca0dfd9910fef69959453c88ca7df114f09be))
- **stdlib**: Core\Json registers decodeEach and encodeEach with their cards ([`990f2590`](https://github.com/novis-lang/novis/commit/990f2590e6cc967af5d33958bddb8f92f1e00b79))
- **stdlib**: Core\Json::decodeEach reads one element at a time ([`fa934963`](https://github.com/novis-lang/novis/commit/fa9349631948549a5ea962bcef01ae4ca36049fa))
- **stdlib**: Core\Json::encodeEach writes an array or JSON Lines in pieces ([`459cd5d6`](https://github.com/novis-lang/novis/commit/459cd5d67001647f4a2bcf10fab9b3ba16878c93))
- **stdlib**: Core\Xml::reader reads every byte source one window at a time ([`16b25a0f`](https://github.com/novis-lang/novis/commit/16b25a0f05e1001b9e3592b3beb45c0b67f99532))
- **stdlib**: Core\Zip reads an archive by ranges from every byte source ([`81f8ac2a`](https://github.com/novis-lang/novis/commit/81f8ac2abc0574bbbcc81e242045231dca7ea90d))
- **stdlib**: a byte source reads a part forward, and decodeEach takes the forward source ([`053846ec`](https://github.com/novis-lang/novis/commit/053846eca75bfe108466cb786d2e9cc9a3c84a62))
- **stdlib**: a typed decode builds an object of scalar fields at its closing brace ([`9c5168f9`](https://github.com/novis-lang/novis/commit/9c5168f920bf0958e29d3be27ad9a5b45032b03b))
- **stdlib**: a typed decode builds every object at its closing brace ([`bc26f223`](https://github.com/novis-lang/novis/commit/bc26f2235c1d015d0849e680a3b9b94ec98dbc71))
- **stdlib**: one byte source reads a file, chunks and bytes ([`24f65922`](https://github.com/novis-lang/novis/commit/24f659220b73230903f5e98f40798f20348ae2e8))
- **types**: Core declares type aliases, and Core\Bytes::Source and Chunks exist ([`e5707179`](https://github.com/novis-lang/novis/commit/e57071797598c9444168e43c5f9292a5ae8872bb))
- **types**: Core\Json::decodeEach<mixed> type-checks and walks plain values ([`a377dc17`](https://github.com/novis-lang/novis/commit/a377dc1703e04b0ff5b847c5e94b27cfb932258f))
- **types**: a foreach over a written array with a typed binding says how to type its items ([`83c1e91c`](https://github.com/novis-lang/novis/commit/83c1e91c1fa812cde2d67bad11180436cf86ba6f))
- **types**: a request part, a body stream and a walk of tainted chunks carry taint ([`b5cd1a5f`](https://github.com/novis-lang/novis/commit/b5cd1a5fcb4853e92792f2f851302d7f2cc4c945))
- **vscode**: each /// doc comment line has a faint background in the Novis themes ([`260aa38b`](https://github.com/novis-lang/novis/commit/260aa38bd2551ca8bed5327c1205c5806548b672))
- **vscode**: the Novis themes draw brackets plainly, PhpStorm's method and variable colours, and boxless hints ([`882a9058`](https://github.com/novis-lang/novis/commit/882a9058090fa070ccc2d7d975d09fef37473e3d))
- **watchdog**: each sweep records how late it woke, and a time stop writes it for the proofs runner ([`a2906803`](https://github.com/novis-lang/novis/commit/a2906803ee7092ee3db0e3aff9fde42b61f5527a))
- **website**: N opens a speaker-notes window for a walkthrough ([`bef87bb1`](https://github.com/novis-lang/novis/commit/bef87bb1d182cbac4a3fe5eedfc8216d4dc3bfa4))
- **website**: a class page with no members has an introduction, and every class page names what returns it ([`3f5bb5e4`](https://github.com/novis-lang/novis/commit/3f5bb5e4c5a92b9de56c4502e4a65691cf6540b7))
- **website**: a gallery shows every walkthrough part in each of its states ([`4d545825`](https://github.com/novis-lang/novis/commit/4d5458251fb1eaab77ccc63ce377ad64d03d4225))
- **website**: a walkthrough caption is a bubble beside what it is about, read while the stage slows and the rest blurs ([`db394721`](https://github.com/novis-lang/novis/commit/db3947217b9bc4896f439d81120d79f504c6c49c))
- **website**: an interactive walkthrough of Core\Task::afterResponse, built from reusable parts ([`31515d9e`](https://github.com/novis-lang/novis/commit/31515d9ef34d3d7dd09d63d52aeeb5bae6314f3e))
- **website**: code takes the VS Code themes' colours, and the dev server restarts when a file the config reads changes ([`9e67d6c1`](https://github.com/novis-lang/novis/commit/9e67d6c183125a32138c5b0abc75bad3ca6b8855))
- **website**: every walkthrough page links its feature's reference pages ([`eab91ed3`](https://github.com/novis-lang/novis/commit/eab91ed3e6a646e1c98a2fcef1ed78f1ae3f0f3d))
- **website**: on a phone the walkthrough camera frames one part at a time ([`2230ad68`](https://github.com/novis-lang/novis/commit/2230ad68f5b4de6ff895f6566f8b9fcdf05d0bda))
- **website**: the front page has a carousel of walkthroughs that load on play ([`6bf7fef9`](https://github.com/novis-lang/novis/commit/6bf7fef96f99e4850f6fcacc76390bd4b40e231f))
- **website**: the phone header shows as many links as fit ([`facf6bc4`](https://github.com/novis-lang/novis/commit/facf6bc45a3a6c2054ac86fc4f125dc7416c54b2))
- **website**: the walkthrough "Page now, slow parts later" shows later and slotted ([`04d9cea3`](https://github.com/novis-lang/novis/commit/04d9cea3ec6efe91c7e01c3cfd84b5a1bc6d13a5))
- **website**: walkthroughs are a Guides group, and the header says Guides, Examples & Walkthroughs ([`18c61ae2`](https://github.com/novis-lang/novis/commit/18c61ae216c0bab4991a3108ad21164b468ba04f))

### Fixes

- **byte-sources**: the gate's reds from the byte source are fixed at their causes ([`9625b979`](https://github.com/novis-lang/novis/commit/9625b9799df81886f07c1d75fac6b2521460d741))
- **byte-sources**: the goal-end gates are green but the website's questions ([`4c38fde0`](https://github.com/novis-lang/novis/commit/4c38fde044fe67e1cc1bc3ae74b88e8f4ee27a52))
- **ci**: nv ci passes over a Release run cancelled behind another on its commit ([`cc76e4ac`](https://github.com/novis-lang/novis/commit/cc76e4ac66767dcf060ec4d96efb1bf11fb334f7))
- **ci**: nv ci watches every workflow that starts runs, so no red run goes unseen ([`e7934827`](https://github.com/novis-lang/novis/commit/e79348275f4de32c02e4d26bb72497c6265345a8))
- **cli**: nvs init checks only the folder it writes into, as nvs serve does ([`4366e437`](https://github.com/novis-lang/novis/commit/4366e4370da84577e5f54017aacf453d3d1d9c2d))
- **docker**: the smoke test copies its app into a volume on Windows, where a bind mount is world-writable ([`3e8b643d`](https://github.com/novis-lang/novis/commit/3e8b643d082c10779fec106ce57a25a053e79bdd))
- **errors**: a message a program sees names no rule id, tag number or slot ([`c0562a1d`](https://github.com/novis-lang/novis/commit/c0562a1d5cbbc26f037498c46409b1ec85e34c84))
- **examples**: text from a body stream and from a PDF is declared tainted ([`bc74e264`](https://github.com/novis-lang/novis/commit/bc74e2642240edae6a193ba7781f79ec8daf2246))
- **host**: a refused connect on macOS reports the refusal, not BrokenPipe ([`081f5974`](https://github.com/novis-lang/novis/commit/081f597409902e748db1005d141b38697999f2b3))
- **host**: a suspendable parse's resume and unwind are bracketed by a sanitizer fiber switch ([`634f091c`](https://github.com/novis-lang/novis/commit/634f091cbe22f323e2384a7e5d0d075c862a2db4))
- **html**: a sanitized answer starts with no BOM, and an adoption-agency move is parsed again ([`eb4153d9`](https://github.com/novis-lang/novis/commit/eb4153d98be67608df7ebcdf5b6f95d031d44851))
- **html**: a sanitized document that fostered a node is a fixed point ([`62e63198`](https://github.com/novis-lang/novis/commit/62e63198ab65ef80d7c2fb1be0dce1bc66901e94))
- **html**: the sanitizer parses in no-quirks mode, so `<p>a<table></p>` rebuilds to a fixed point ([`21528eff`](https://github.com/novis-lang/novis/commit/21528effa0646c08183f5a1eb26121aa7fc43e28))
- **html**: the serializer writes a CR as &#13; ([`bbfa6626`](https://github.com/novis-lang/novis/commit/bbfa662688b1ae28de57f52539501892521f865c))
- **http**: a reply body's walk takes from the front without moving the rest ([`04640cfc`](https://github.com/novis-lang/novis/commit/04640cfc26e821f810cac4795ed36d807da1ecfe))
- **limits**: a CPU raise from before the handler's entry does not stop the handler ([`f7f40eb1`](https://github.com/novis-lang/novis/commit/f7f40eb1636397bbed162b9bcd333c7f525f81dc))
- **limits**: a wall-time limit handler's slice is counted from its entry ([`cf009474`](https://github.com/novis-lang/novis/commit/cf009474e6639a8862a7bcfbd60675a62cc825c5))
- **lsp**: a comment offers nothing, and a doc line offers its tags and their targets ([`d947df7f`](https://github.com/novis-lang/novis/commit/d947df7fa7ab45a2012db67f7682b964132ecac5))
- **lsp**: a request with a later edit to its document queued is answered ContentModified ([`0f393976`](https://github.com/novis-lang/novis/commit/0f3939764d5a31f6d6abfa0278ce47f03c071ea8))
- **lsp**: a start answers requests before the workspace is indexed ([`8bfd5e22`](https://github.com/novis-lang/novis/commit/8bfd5e22b3f5b8b918b364b1ba39fc69546049ce))
- **markdown**: `~` and `~~` keep one lower bound each, so a `~~` with no opener leaves a `~` opener matchable ([`9edd84b4`](https://github.com/novis-lang/novis/commit/9edd84b466d8899ca980720b14976bf45f4ad32e))
- **markdown**: a CDATA section in a paragraph ends at the first `]]>` ([`a30beca3`](https://github.com/novis-lang/novis/commit/a30beca3b6b514797f8d20adc20df556ee1f0148))
- **markdown**: a CR an entity decodes to is written as LF ([`7f30cd83`](https://github.com/novis-lang/novis/commit/7f30cd837cca339aa57fad13e69168b74437ea19))
- **markdown**: a Live frees its state with the object, and a callback re-entering it throws ([`dbe3ab8b`](https://github.com/novis-lang/novis/commit/dbe3ab8b0d75aa60bde9cd4f1b24199075089171))
- **markdown**: a `\` in a link title is an escape, and a list item is no delimiter row ([`9378a955`](https://github.com/novis-lang/novis/commit/9378a95596a03e6dd44a4248148a9b6a916dbce6))
- **markdown**: a `\` that ends a header row is the cell's text ([`ce78e532`](https://github.com/novis-lang/novis/commit/ce78e532326cec2b63135901a85f67a9a383b8ec))
- **markdown**: a blank line that ends an HTML block loosens its list ([`ec06db20`](https://github.com/novis-lang/novis/commit/ec06db20ea0f1171ca965cb5e46bb30dd070b958))
- **markdown**: a closing tag may end on the next line ([`2cdcab4b`](https://github.com/novis-lang/novis/commit/2cdcab4be23ae02e74f4e31766536ff7271ec99a))
- **markdown**: a code block's language is read after its info string is decoded ([`a1f0c764`](https://github.com/novis-lang/novis/commit/a1f0c7646b057b6c9dae699fb22a44c228cb0d75))
- **markdown**: a delimiter row after spaces may be a list item ([`2d32fd04`](https://github.com/novis-lang/novis/commit/2d32fd049d5e46abea97af261e14c033a2da8237))
- **markdown**: a group after one the rebuild leaves empty starts the page ([`58a8254d`](https://github.com/novis-lang/novis/commit/58a8254d5e14c80e0b20c6afd7f0baf894983071))
- **markdown**: a header row indented four columns or more still starts a table ([`15d06a47`](https://github.com/novis-lang/novis/commit/15d06a474865a8bed60da6672decdc88f230fd7a))
- **markdown**: a header row that would be an empty list item starts a table ([`7c0b220b`](https://github.com/novis-lang/novis/commit/7c0b220b2f3386369e7f652f57c3312898377158))
- **markdown**: a lazy `===` line is part of a reference definition ([`734ccb35`](https://github.com/novis-lang/novis/commit/734ccb35a98b1a9df6d51ded4650f46d7d8bd433))
- **markdown**: a lazy line before a delimiter row is a header row ([`dffbfb86`](https://github.com/novis-lang/novis/commit/dffbfb869a53e6592b5737390f0359e3200d0043))
- **markdown**: a line indented four columns ends a table ([`ba21ac41`](https://github.com/novis-lang/novis/commit/ba21ac41a7cc187e4a04eb5ca42268b24249c7b1))
- **markdown**: a line that starts an HTML block of type 7 ends a table ([`0b3ca4b9`](https://github.com/novis-lang/novis/commit/0b3ca4b9962914408cc2894636b806b6e7997eef))
- **markdown**: a link label after an escaped `[` is text ([`1f62353b`](https://github.com/novis-lang/novis/commit/1f62353b8c6566293bd1011bf48b85742f244d09))
- **markdown**: a link title needs a space after its destination ([`ed3bd7f7`](https://github.com/novis-lang/novis/commit/ed3bd7f7b4697a7bb6bf7dfa4bd5fc8e4c624ff6))
- **markdown**: a list ends where a reference definition after it starts ([`740e0abe`](https://github.com/novis-lang/novis/commit/740e0abea5fdc58eb39f2a0d2f87b0b9286bc96d))
- **markdown**: a loose task item with nothing after its box has no paragraph ([`5429af7d`](https://github.com/novis-lang/novis/commit/5429af7d11b11c534d1c894628cdcab08ae91eec))
- **markdown**: a loose task item's box is written in front of its first paragraph ([`820d49fb`](https://github.com/novis-lang/novis/commit/820d49fbd47f11a84238add9b0125ab877114bec))
- **markdown**: a paragraph a table interrupts defines no reference ([`97224a30`](https://github.com/novis-lang/novis/commit/97224a30cbbfa8efc0d504bd5284058d16d882f6))
- **markdown**: a paragraph whose delimiter row made no table never becomes one ([`354bfb85`](https://github.com/novis-lang/novis/commit/354bfb85c9e8a014bb0c1340a50c7b94b17615cb))
- **markdown**: a parse of uncut text asks the memory limit first ([`e576e370`](https://github.com/novis-lang/novis/commit/e576e37007c366b4909e26f01a3c1a294b455549))
- **markdown**: a run of raw tags that never close costs linear time ([`4bec7d46`](https://github.com/novis-lang/novis/commit/4bec7d46032369a74142425fc2b0cd9d86fa3bec))
- **markdown**: a second blank line ends an empty list item ([`ddee5853`](https://github.com/novis-lang/novis/commit/ddee5853a4e42b46467778425b25075c2236a777))
- **markdown**: a segment is cut only at a line start ([`b8aa1191`](https://github.com/novis-lang/novis/commit/b8aa1191c56c99b0341cdfe5f2551451545717b9))
- **markdown**: a segment is cut where the text after it reads alone as in the whole ([`a5b2b137`](https://github.com/novis-lang/novis/commit/a5b2b137dbab12d9484c982f44acc73e92efe7e4))
- **markdown**: a setext underline under a lone task marker does not panic ([`05fc53b2`](https://github.com/novis-lang/novis/commit/05fc53b25f87b271cec3298558760e9598c85b2a))
- **markdown**: a tab between a task marker's brackets is text ([`d963408a`](https://github.com/novis-lang/novis/commit/d963408a37a0a2c987716cfcab5671f2d6c78f59))
- **markdown**: a tab ends an unquoted attribute value in raw HTML ([`23abbde8`](https://github.com/novis-lang/novis/commit/23abbde8b04d33b19e512cdbf69a8242c7c1709e))
- **markdown**: a tab-indented `>` does not continue a block quote ([`137fc24a`](https://github.com/novis-lang/novis/commit/137fc24a9571c897601e527f89ad5e21306e95e5))
- **markdown**: a task marker after four columns of tabs is code ([`c57f5d1c`](https://github.com/novis-lang/novis/commit/c57f5d1c3ca2a44d52b0810e1c67bccfd00cdbc6))
- **markdown**: an HTML block's end tag is matched in any case ([`6804edf7`](https://github.com/novis-lang/novis/commit/6804edf74b3e5c99a6d07d41b8d0a43bb22b0141))
- **markdown**: an angle autolink inside a link's text is written as text ([`b4f9ff87`](https://github.com/novis-lang/novis/commit/b4f9ff878b1a34ff26aef83b4205237e36c74d9e))
- **markdown**: an autolink's entities resolve, as cmark resolves them ([`b3f755d4`](https://github.com/novis-lang/novis/commit/b3f755d4710a488a1f6945794d59cd7c22254338))
- **markdown**: an escape inside `www.` or a scheme stops an autolink ([`2192f1d4`](https://github.com/novis-lang/novis/commit/2192f1d47e44bd276e286a293e57733ee8454039))
- **markdown**: an indented blank line keeps an empty list item open ([`a6585eda`](https://github.com/novis-lang/novis/commit/a6585edada31c27011556a80874e7c30f691d928))
- **markdown**: an inline link's title is the longest cmark reads ([`13740596`](https://github.com/novis-lang/novis/commit/13740596623a4206e2dbfb43c8d7137bb0455eb8))
- **markdown**: fromHtml drops a lone last break and cuts more emails ([`d76fd0a5`](https://github.com/novis-lang/novis/commit/d76fd0a5f96e5844debba9c040eb904cd533a5b8))
- **markdown**: fromHtml escapes a `!` right before a link ([`2d0fee0e`](https://github.com/novis-lang/novis/commit/2d0fee0ec86a65f74bfc475894159a601353dd51))
- **markdown**: fromHtml escapes a colon at the start of a line ([`9ba5124c`](https://github.com/novis-lang/novis/commit/9ba5124ccb437653af1a57bd4e229b3bd41863ee))
- **markdown**: fromHtml keeps a form feed as text ([`b4262c7d`](https://github.com/novis-lang/novis/commit/b4262c7d059c26e7db86aede202db508182a5b0e))
- **markdown**: fromHtml keeps text after a closed nested list tight ([`76fbb1fe`](https://github.com/novis-lang/novis/commit/76fbb1fe409a99b312784a01d47fc18c33e593c1))
- **markdown**: fromHtml writes a URL percent-encoded as the renderer writes it ([`9d7def29`](https://github.com/novis-lang/novis/commit/9d7def29b23ca008211268f36be693c2653035f2))
- **markdown**: fromHtml writes a code span of one space as ` ` ([`8a4cb7a2`](https://github.com/novis-lang/novis/commit/8a4cb7a280acc4bf1faf142cbc36a12ea87896a7))
- **markdown**: fromHtml writes a leading U+FEFF as an entity ([`9429a35c`](https://github.com/novis-lang/novis/commit/9429a35c0a4cb8a92ea059c8638ad5e0d1661885))
- **markdown**: fromHtml writes every line break a browser shows ([`a85a174a`](https://github.com/novis-lang/novis/commit/a85a174a93dee372a3328a8c2fe11a0ee47c4c55))
- **markdown**: fromHtml writes nested emphasis that parses back to the same nesting ([`9f43defc`](https://github.com/novis-lang/novis/commit/9f43defc6029f3482bb01d6d9adb795ed6461ac5))
- **markdown**: fromHtml writes nested lists on one line without a thematic break ([`9f233a7e`](https://github.com/novis-lang/novis/commit/9f233a7e9a30d6649e976677d80f8ff89964227e))
- **markdown**: fromHtml writes nothing for a list with no items ([`fb0a8053`](https://github.com/novis-lang/novis/commit/fb0a8053c72b9705dd4c2e27f780386366bafed0))
- **markdown**: fromHtml's Markdown reads back as the HTML it was written from ([`b4535b18`](https://github.com/novis-lang/novis/commit/b4535b18927118c65123d7a6a17a84ac9132ec93))
- **markdown**: fromHtml's www cut reads bytes, not a slice of the text ([`e6005b35`](https://github.com/novis-lang/novis/commit/e6005b35d37624caae8666a247bd259ed55370f0))
- **markdown**: inline raw HTML across lines drops each container marker ([`6aa0f087`](https://github.com/novis-lang/novis/commit/6aa0f0872845181e795d053c97ad526e4b08db03))
- **markdown**: raw HTML in an image's description is its source text in the `alt` ([`0a46b9b9`](https://github.com/novis-lang/novis/commit/0a46b9b91f6c0c3bf8c7c557217216606eafddfd))
- **markdown**: tables, task items and strikethrough read as GFM and cmark-gfm read them ([`912072ec`](https://github.com/novis-lang/novis/commit/912072ec0b0628007998bfdbcf92ecb466ee9389))
- **markdown**: text before the rebuild writes `"` as `&quot;` ([`6d7545e1`](https://github.com/novis-lang/novis/commit/6d7545e1b509c84c2ad1079b6c3f4921d0946882))
- **markdown**: the goal's gate is green ([`c56acc42`](https://github.com/novis-lang/novis/commit/c56acc428e595b998eeed2603a2a9ef7dfa8600f))
- **markdown**: toHtmlEach asks the memory limit before it reads each piece ([`73a87a8d`](https://github.com/novis-lang/novis/commit/73a87a8d8af27996905e8de041f8ab19ae5431e4))
- **markdown**: toHtmlEach keeps a double-escaped script open across group cuts ([`e31845c5`](https://github.com/novis-lang/novis/commit/e31845c58e1c35928fd13b432c6ae48cb86ac6bc))
- **metrics**: a gather never reads a core whose thread has ended ([`290a5237`](https://github.com/novis-lang/novis/commit/290a523784f9930ed30895d85d93fc40864837fd))
- **nv**: CONTRIBUTING.md lists nvs-fuzz-checks, and ci --check runs the layout check ([`3cec78f6`](https://github.com/novis-lang/novis/commit/3cec78f67dcf53815a30b4a7e2b29b9449604cc6))
- **nv**: `bun nv ci --watch --past` covers a fuzz leg that names several targets after a comma ([`aa90ad79`](https://github.com/novis-lang/novis/commit/aa90ad79103733ac2d068140a3d867ff43ea2cd4))
- **nv**: `bun nv deep`'s `fuzz-smoke` step is not killed while libFuzzer replays a grown corpus in silence ([`10a93a08`](https://github.com/novis-lang/novis/commit/10a93a0828604693f919ced8220ceace080f1595))
- **nv**: `bun nv deep`'s docker step gives the build a writable `target` inside the read-only source ([`a0d1aec6`](https://github.com/novis-lang/novis/commit/a0d1aec6ca6232fc7ffdd4d9fb5c9640f9d40c21))
- **nv**: links reads a source file's comments only, and skips vendor/ ([`3a409d8d`](https://github.com/novis-lang/novis/commit/3a409d8d0ead0cd7cc0bf74d28892b8da2c81698))
- **nv**: scaling --budget --write stores a new bench's share while another is over ([`65d01e61`](https://github.com/novis-lang/novis/commit/65d01e61e381718ea78f1eefb2cec342de8e4240))
- **pdf**: a fixed-height box taller than the room left continues on the next pages ([`8a45e985`](https://github.com/novis-lang/novis/commit/8a45e985dce9e2fe56440792eecaa4bda61d897b))
- **pdf**: a flex band taller than a page splits between its items' lines ([`3713e55c`](https://github.com/novis-lang/novis/commit/3713e55c87066a118061d906405688d3416975a7))
- **pdf**: a heading kept with a block taller than a page moves with that block's first lines ([`e833c6a6`](https://github.com/novis-lang/novis/commit/e833c6a63e9d2424c76ca8bd6f2982a7b7758f97))
- **pdf**: a line that floats leave no room starts below them ([`0f5f9fbd`](https://github.com/novis-lang/novis/commit/0f5f9fbd9023b340866d8472a7a1cdbb8ac5838f))
- **pdf**: a right-to-left table and flex row run from the right ([`8af3bc7e`](https://github.com/novis-lang/novis/commit/8af3bc7ea007516c6af73ec167fd3a19c3eacbf8))
- **pdf**: a table copies only its own cells, so tables nested 2000 deep end at the depth error ([`72252177`](https://github.com/novis-lang/novis/commit/7225217765b3f3d4c42d916c0ec1a38a53bab5fd))
- **pdf**: a table's header and footer are left off when no row fits with them ([`20cc89a9`](https://github.com/novis-lang/novis/commit/20cc89a9da2ca09d4417898cd81056c1942a3b8f))
- **pdf**: collapsed table borders resolve one border per edge by CSS 2.1 ([`98ea4028`](https://github.com/novis-lang/novis/commit/98ea40282df4b1f1d081176e9623e60ed7c23e2e))
- **pdf**: non-finite values are dropped and reported, var() resolves each property once, size-0 text is skipped ([`ed73be4d`](https://github.com/novis-lang/novis/commit/ed73be4d7d75edf7d75b2817df4f8dc7fcbed297))
- **pdf**: overflow clips, visibility hides, margins go negative, and list types are read ([`a253a7f1`](https://github.com/novis-lang/novis/commit/a253a7f1e6f36e3551042aa5ed14309fca368835))
- **pdf**: page counters restart, page selectors and media features are read or reported ([`42fa083b`](https://github.com/novis-lang/novis/commit/42fa083b1db22c8e46e5534d1cd95d70f15157e8))
- **pdf**: page sides follow the document's direction, and a forced break keeps the next margin ([`e1eb5d41`](https://github.com/novis-lang/novis/commit/e1eb5d41aaa67da5a09ecb9ee9f2f424d967093f))
- …and 58 more, in the full changes below.

### Performance

- **bench**: Core\Test's members are measured again on the completion's file read ([`9ae59789`](https://github.com/novis-lang/novis/commit/9ae5978922e473e64a68b54cf12e2b1092120cb5))
- **bench**: the comparison is recorded on the tree with the faster request path ([`972da656`](https://github.com/novis-lang/novis/commit/972da656aa131e2a1e947c578935157eef177d37))
- **bench**: the comparison's cases are recorded on the tree with one read per request ([`4ccbe86a`](https://github.com/novis-lang/novis/commit/4ccbe86a79038f024ca5aa9b277a2de28a9cf3e1))
- **bench**: the literals bench fills an html template, and three string benches declare their allocations ([`05a7f44d`](https://github.com/novis-lang/novis/commit/05a7f44db95aeced8721d98a484ebb98267a24fa))
- **byte-source**: a file is read into a buffer of its size, at most one read chunk ([`e73a6fcb`](https://github.com/novis-lang/novis/commit/e73a6fcb8e3f07e30e1fc341d9ed2a636ee1c6fa))
- **byte-sources**: every figure the byte source made stale is measured again ([`6bf83c94`](https://github.com/novis-lang/novis/commit/6bf83c94cbfa04a1389ace9ba55d67f877ceb974))
- **byte-sources**: the figures joined to byte_source.rs are measured again ([`c3719ad9`](https://github.com/novis-lang/novis/commit/c3719ad959d7bb0682f0cfa840ae68a261d8ae80))
- **check**: a class file's lender search walks its own project and reads each file once ([`62a91776`](https://github.com/novis-lang/novis/commit/62a917765aa2f8f54d3563de35d7371e4aac36a3))
- **check**: an autoloaded file's path is resolved once per compile and once per lender question ([`c8d86440`](https://github.com/novis-lang/novis/commit/c8d86440089be70e61ff31f2ceaf1c45439cb1ec))
- **config**: an uncached run resolves each settings path and its entry once per form ([`28aba995`](https://github.com/novis-lang/novis/commit/28aba995c8ff2481e57f46ce8d75c10df7735ff6))
- **host**: a request that keeps its default CPU limit never wakes the watchdog ([`f964813a`](https://github.com/novis-lang/novis/commit/f964813a5233a4ab8b5dbab83f9b3e2bea663856))
- **host**: a short read on Unix parks the next read without a recv ([`64976035`](https://github.com/novis-lang/novis/commit/649760357ac8bef60404622b8a09088a84839cd5))
- **host**: an outbound connect asks the zero-length write before SO_ERROR ([`5b2037df`](https://github.com/novis-lang/novis/commit/5b2037dfac90d226b3e1ca80eb8028bcca75421c))
- **html**: Core\Html's six figures are measured on the BOM and adoption-agency fix ([`0dead72b`](https://github.com/novis-lang/novis/commit/0dead72b3a6a9f45cf1d923fa9aef3f68cbbdc7d))
- **ir**: a bare integer placeholder joins as an integer part ([`670506f2`](https://github.com/novis-lang/novis/commit/670506f22d4bd1f8bb9f831dc9b5742fd6de5784))
- **lsp**: a cold workspace index costs a sixth of what it did ([`92d208d2`](https://github.com/novis-lang/novis/commit/92d208d21ced717dcb217e9f4b6af3d1df2f24ea))
- **lsp**: an index entry is analysed apart from the merge, and resolves its path once ([`09e9b51d`](https://github.com/novis-lang/novis/commit/09e9b51d102a267f7b273baa23bfea1a5523e33d))
- **lsp**: the editor parses each settings file once until it changes ([`9c8e2e99`](https://github.com/novis-lang/novis/commit/9c8e2e993b97843bd3ad9c73ae52adb38dd36e5f))
- **lsp**: the index settles on worker threads, and files each result in path order ([`cd620e16`](https://github.com/novis-lang/novis/commit/cd620e164b01c4565ed63c4ff9d2577f8abd6829))
- **nv**: bun nv fuzz runs a target on half the cores in fork mode ([`31d1ab81`](https://github.com/novis-lang/novis/commit/31d1ab814882887a0a65aefba54da531e51d452c))
- **pdf**: a table's rows are paged with suffix sums, so a longer table costs work in proportion ([`e4634c50`](https://github.com/novis-lang/novis/commit/e4634c50385183dbd450a757c864ecc71996903a))
- **pdf**: an image used many times in a document is resolved and decoded once ([`f8c95dc2`](https://github.com/novis-lang/novis/commit/f8c95dc242214ee907d2c6e756071efd5161956c))
- **pdf**: margin boxes read named strings, counters and running elements from a page index ([`4c581b3e`](https://github.com/novis-lang/novis/commit/4c581b3edbe8c5125fc82f4cb6734c678d4cbb25))
- **pdf**: the cascade tests an element only against selectors its id, classes or tag can match ([`89081b3a`](https://github.com/novis-lang/novis/commit/89081b3aef94b4783ec30410c5a0b9f50ab2cd3e))
- **proofs**: every stale stdlib perf figure is measured again on this tree ([`c7517507`](https://github.com/novis-lang/novis/commit/c7517507c310c550f9a3ff23bada82f0dda5bff9))
- **proofs**: the IO, Xml, Compress, Request and Mime classes are measured again ([`2ffccd6a`](https://github.com/novis-lang/novis/commit/2ffccd6a1dfa781e1a0786b602ea1e75e78c3c27))
- **proofs**: the members stage 4 touched and the escape and format members carry fresh records ([`cde06a13`](https://github.com/novis-lang/novis/commit/cde06a13e812f51bc923c6c4632e3ae7e7714e3d))
- **proofs**: the stream, flags and reflect members carry records taken after rustfmt ([`a603b742`](https://github.com/novis-lang/novis/commit/a603b742465851b3345e144cd7054a67e683631e))
- **queue**: the queue and JSON members carry fresh perf records ([`b84382ec`](https://github.com/novis-lang/novis/commit/b84382ec2f83f35f0854343c8c997c9e67269a85))
- **render**: an escaped text is scanned once to measure and once to write ([`755bf4df`](https://github.com/novis-lang/novis/commit/755bf4df62418d4927eb473d4298b4a68d41912c))
- **runtime**: a byte writer's write that fits its buffer asks the budget nothing ([`7b04f310`](https://github.com/novis-lang/novis/commit/7b04f310528dcfb1b3b5b18d16483a0bc0467996))
- **runtime**: an integer placeholder writes its digits without Rust's formatter ([`f837844b`](https://github.com/novis-lang/novis/commit/f837844b20e977472c19eae2228ef9077963343b))
- **server**: Core\Response::json's body is the encoder's chunks and peaks at the document plus one chunk ([`f4dc58dd`](https://github.com/novis-lang/novis/commit/f4dc58dd760fce207667d525e6e87afebb548dee))
- **spreadsheet**: custom formats, long format codes and whole-column references cost linear work ([`4e38a9cc`](https://github.com/novis-lang/novis/commit/4e38a9cc181f1e05a1b915dc77537f634d161aec))
- **stdlib**: 56 Core\Json, Core\Request, Core\Response, Core\Script and Core\Test records are measured again ([`a29c3d4c`](https://github.com/novis-lang/novis/commit/a29c3d4c1e3492f6ec4ee92b2e2ad2a0a7052a13))
- **stdlib**: Core\Json's encoder allocates nothing per element and peaks at twice the document ([`486b65e4`](https://github.com/novis-lang/novis/commit/486b65e4c87dba5863c575b291fa2bccc1514871))
- **stdlib**: Core\Response::json hands the encoder's buffer to the body ([`7f05c071`](https://github.com/novis-lang/novis/commit/7f05c071e587beb1e16bd769d15b7cec80129f72))
- **stdlib**: Core\Str is measured again after find_counted ([`8df6864d`](https://github.com/novis-lang/novis/commit/8df6864d1fe64f880e60b1983df175e3fc17d559))
- **stdlib**: Core\Test's assertCount, assertEquals and assertNull ramp to a batch where their counts agree ([`ac16c795`](https://github.com/novis-lang/novis/commit/ac16c79512cb68d073b223d1b85e452bd54cc310))
- **stdlib**: Core\Test's perf records are measured on the test.rs that answers a sendFile response ([`bcd795c5`](https://github.com/novis-lang/novis/commit/bcd795c5fa68d450ba4b293f4d39133a621d08d2))
- **stdlib**: a Core instance finds its descriptor by the class's address ([`35745036`](https://github.com/novis-lang/novis/commit/3574503695d1a8f4e0015ee91cfbd33ec6926e90))
- **stdlib**: a request body with a declared length is read into one buffer of that length ([`9bcf85a1`](https://github.com/novis-lang/novis/commit/9bcf85a1ac8bd11d251f02aed0965a739031c7fa))
- **stdlib**: decodeEach and encodeEach each have a bench whose ladder holds memory flat to 256 MiB ([`81ca7312`](https://github.com/novis-lang/novis/commit/81ca73122d929df1543adc2a8c2096650d5bf74b))
- **stdlib**: every JSON member states its peak per document byte, and its size ladder is recorded flat ([`793ef0da`](https://github.com/novis-lang/novis/commit/793ef0daea3ce3fd1169c89f27ce9595624f7233))
- **stdlib**: the JSON members' size ladders grow the document in bytes from 1 KiB ([`59f53faa`](https://github.com/novis-lang/novis/commit/59f53faa1ca7af29c2e270237494f292e526f1a9))
- **stdlib**: the JSON reference ladder times the free apart and prints serde_json's own encode ([`e3edb8a4`](https://github.com/novis-lang/novis/commit/e3edb8a44a87c76ce0cd208326537f3b3cdddd13))
- **stdlib**: the typed JSON readers are measured again and declare their allocation counts ([`c60ce6a4`](https://github.com/novis-lang/novis/commit/c60ce6a4d6cdb4d2d248b03eb99a1672b7e5b202))
- **test**: a directory of programs reads each file once ([`6c13c339`](https://github.com/novis-lang/novis/commit/6c13c3390800d3caed715c2e70a93a70922eb771))
- **test**: a directory of programs resolves only the files another one requires ([`5e53c0ad`](https://github.com/novis-lang/novis/commit/5e53c0ada37a68c95f601d9927238dd42f304759))

### Formatter

- **fmt**: io, otlp and the limits tests are formatted by rustfmt ([`c0ff7183`](https://github.com/novis-lang/novis/commit/c0ff718328a47c6c081f6e73f4d0336e4d19a5cf))
- **fmt**: the goal's earlier slices are in rustfmt's layout ([`ebc4ac9f`](https://github.com/novis-lang/novis/commit/ebc4ac9fd1941fc5dd8137266bd63318159ef4fa))
- **fmt**: the parallel index's files are in rustfmt's layout ([`cc9bfb2b`](https://github.com/novis-lang/novis/commit/cc9bfb2bae2b01649b45a6fa6588114bc7bfc936))
- **fmt**: the typed-parts join and its lowering are formatted as rustfmt writes them ([`5c238d3c`](https://github.com/novis-lang/novis/commit/5c238d3c02f2259e7f31b885f09dad176d6661c8))
- **fmt**: the watchdog policy case is formatted by rustfmt ([`6560a9b5`](https://github.com/novis-lang/novis/commit/6560a9b5ea6170ac847574404401e2094860776f))

### Internal

463 further commits: docs (268), test (101), chore (72), build (12), refactor (6), wip (4).

[Full changes](https://github.com/novis-lang/novis/compare/v0.0.2...v0.0.3) — 772 commits.

## [0.0.2] - 2026-11-03

### Features

- **lsp**: a body-only edit of an open file re-queues no reader ([`84172184`](https://github.com/novis-lang/novis/commit/84172184433eeea9ba51f4fac6495dd294a4dda4))
- **lsp**: a file is parsed again only when its text changed ([`207a0cd4`](https://github.com/novis-lang/novis/commit/207a0cd4674b468c6ce2b814c176108e9db15a4a))
- **lsp**: the editor checks only the bodies whose inputs changed, against one session interner ([`4e0674e8`](https://github.com/novis-lang/novis/commit/4e0674e8cce327d17c3459ba314885c6e3f1f4b4))
- **lsp**: the symbol index is kept on disk, and a start analyses only its stale entries ([`59eb4b7f`](https://github.com/novis-lang/novis/commit/59eb4b7f4cd60f171137a0d99c0be431644039c2))
- **site**: the website counts page views with a cookieless Matomo on stats.novis-lang.org ([`da19c63e`](https://github.com/novis-lang/novis/commit/da19c63e1091ffe261a879a791b9905ee189af8a))
- **tools**: NVS_PROOF_TIMES writes each proof program's wall time, slowest first ([`f62d0639`](https://github.com/novis-lang/novis/commit/f62d063984e5a5c7e32209606169a6b65c2581d3))
- **types**: a check walks the bodies of a chosen set of files and folds every other file's cached output ([`297f9356`](https://github.com/novis-lang/novis/commit/297f935629565bc0a81299be015467c5845f5cf2))
- **vscode**: the nvs.path setting links the download and the release page ([`1e6737b7`](https://github.com/novis-lang/novis/commit/1e6737b7c6240011c979b7b0403061f194176421))

### Fixes

- **cli**: nvs run's stop cancels a queue worker still opening its connection ([`63a6861d`](https://github.com/novis-lang/novis/commit/63a6861d44bb8588bf152a90baf48dc67468f310))
- **cli**: nvs run's stop cancels a queue worker still opening its connection again ([`45e4ac4c`](https://github.com/novis-lang/novis/commit/45e4ac4c7c3db5e521e098171b3359f877455f2a))
- **cli**: nvs run's stop no longer cancels a queue worker still opening its connection ([`cb931f1e`](https://github.com/novis-lang/novis/commit/cb931f1efaded760478f1e2e75b98f081f5cabde))
- **goals**: incremental-index's named checks run the tests they name ([`fa8b07a3`](https://github.com/novis-lang/novis/commit/fa8b07a3b628d191997883063d477b132dc9e4d0))
- **host**: a cancelled task unwinds without a crash in Windows' optimized builds ([`6b0bc591`](https://github.com/novis-lang/novis/commit/6b0bc591ce0f793bf528233e8496b34fd34fffae))
- **host**: the slice clock wakes on time on macOS, so a group's deadline is read before the CPU stop ([`48879136`](https://github.com/novis-lang/novis/commit/488791366020cd1f43d05e9a4b11898cceb2674b))
- **lsp**: the editor types a program against the built-in components ([`35d3753f`](https://github.com/novis-lang/novis/commit/35d3753fe6f2e0c5235303120366468eac5096d2))
- **nv**: a release judge reads build.yml's legs at the run's commit ([`60bf5449`](https://github.com/novis-lang/novis/commit/60bf5449fbce929d6f196fe8ea6b5484ade31214))
- **nv**: embedded text runs no platform matrix ([`0d9eda6a`](https://github.com/novis-lang/novis/commit/0d9eda6a7ee33fa769c90fb421b8fabe82854f4d))
- **nv**: the code lane covers every input the binary is built from ([`b5a12569`](https://github.com/novis-lang/novis/commit/b5a12569b6185c7afe4e5a8d3a89bafcdaeb8f55))
- **release**: a release binary names the commit it was built from, without -dirty ([`4ce8a689`](https://github.com/novis-lang/novis/commit/4ce8a6893fc6951470870f022d7f63d757600a98))
- **release**: publish fetches the committed lock before --apply bumps the version ([`53c7b504`](https://github.com/novis-lang/novis/commit/53c7b504e968ddaaac27b108149747d5db811e21))
- **runtime**: a Serialize payload refused partway frees every object decode built ([`4b8503ca`](https://github.com/novis-lang/novis/commit/4b8503ca80d7fb8ee67c02f4768bf0dfebcb3ee8))
- **site**: the front page names the site Novis-Lang to search engines ([`1684a158`](https://github.com/novis-lang/novis/commit/1684a1580b6c9026d58ed5237ff4ee83fce9e5fa))
- **site**: the front page's search questions are stamped against its new head ([`383af5ab`](https://github.com/novis-lang/novis/commit/383af5abc45d4659f0e35cdd722c338960196ae8))
- **site**: the hero slogan says SQL injection does not compile ([`d5294d15`](https://github.com/novis-lang/novis/commit/d5294d15a7339ff03c827c29e37b5ecf680cf074))
- **site**: the hero slogan says SQL injection is blocked, so it never reads as NoSQL injection ([`72b8c974`](https://github.com/novis-lang/novis/commit/72b8c9749abe6095f90bb4765f11c93c23c87f59))
- **spreadsheet**: the tests read back through calamine 0.38, clear of RUSTSEC-2026-0436 ([`56f215bb`](https://github.com/novis-lang/novis/commit/56f215bb0a1fe129b0e47fdf4bf1d887726f63bc))
- **types**: a per-file table part inherits the provenance switch and never turns it on ([`a35771e7`](https://github.com/novis-lang/novis/commit/a35771e7ca124c25619eb2b5c88d7c29dded36d3))
- **website**: the sidebar opens with the current page's entry in view ([`2dfacf60`](https://github.com/novis-lang/novis/commit/2dfacf6099c8f4c6068b1be838b5ced38f1f31df))

### Performance

- **lsp**: an answer waits for the open document, never for the index ([`e5b77942`](https://github.com/novis-lang/novis/commit/e5b7794256c67ea741d1f4ffac795ed65fc8b757))

### Internal

55 further commits: docs (24), build (10), test (10), chore (8), wip (2), refactor (1).

[Full changes](https://github.com/novis-lang/novis/compare/v0.0.1...v0.0.2) — 84 commits.

## [0.0.1] - 2026-11-02

This is the first release of Novis.

Novis is the programming language for web applications and the command line. You can build a
business app, a website, an API or any other web service with it. The tools that a modern web
application needs are built in.

- **Input is tainted.** Text from a request must be checked or escaped before it reaches a
  database, a shell or a page.
- **Every request on its own.** Each request has its own memory. When one request crashes, the
  others keep running.
- **One program.** `nvs` runs your code, serves web requests, runs your tests and formats your files.
- **Compiled to machine code.** Each file is compiled the first time it is needed. You never run a
  build step.

### Pre-Alpha

Novis is Pre-Alpha. Any release can change the language, and Novis is not ready for production use.

### Install

Download the archive for your system from the files below. Each archive contains the `nvs` program.
Put `nvs` on your `PATH`, then run `nvs --version` to check it. `SHA256SUMS` lists the checksum of
every file.

You can also use the Docker image:

```sh
docker pull ghcr.io/novis-lang/novis:0.0.1
```

The editor extension for VS Code is `nvs-0.0.1.vsix`, in the files below. In VS Code, open the
Extensions view and choose **Install from VSIX...**.

### Read more

- [Install Novis](https://novis-lang.org/install/)
- [The roadmap](https://novis-lang.org/in-depth/roadmap/): the work that is planned next.
- [What changed](https://novis-lang.org/in-depth/what-changed/): what is new in Novis.
