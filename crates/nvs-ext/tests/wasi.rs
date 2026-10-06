//! `rule:packaging/a-guest-has-no-ambient-authority`'s link half: the WASI interfaces a guest's
//! linker defines are exactly `nvs_ext::wasi::LINKED`, proved by instantiating one probe component
//! per WASI interface the world's WIT holds.

use std::collections::BTreeSet;

use nvs_ext::call::Host;
use nvs_ext::wasi::LINKED;
use wasmtime::component::Component;
use wit_component::{ComponentEncoder, StringEncoding, dummy_module, embed_component_metadata};
use wit_parser::{ManglingAndAbi, Resolve};

/// Every WASI interface under `wit/nvs-ext/`, as `wasi:<package>/<interface>@<version>`.
fn wasi_interfaces(resolve: &Resolve) -> Vec<String> {
    let mut out = Vec::new();
    for (_, package) in &resolve.packages {
        if package.name.namespace != "wasi" {
            continue;
        }
        for name in package.interfaces.keys() {
            out.push(format!(
                "wasi:{}/{name}@{}",
                package.name.name,
                package
                    .name
                    .version
                    .as_ref()
                    .expect("a WASI package is versioned")
            ));
        }
    }
    out
}

/// A component importing `interface` alone, and whatever its types are written in.
fn probe(resolve: &mut Resolve, index: usize, interface: &str) -> Vec<u8> {
    let package = resolve
        .push_str(
            format!("probe{index}.wit"),
            &format!("package probe:p{index};\nworld w {{\n    import {interface};\n}}\n"),
        )
        .expect("the probe world parses");
    let world = resolve
        .select_world(&[package], Some("w"))
        .expect("the probe world exists");
    let mut module = dummy_module(resolve, world, ManglingAndAbi::Standard32);
    embed_component_metadata(&mut module, resolve, world, StringEncoding::UTF8)
        .expect("the probe's metadata embeds");
    ComponentEncoder::default()
        .module(&module)
        .expect("the probe module reads")
        .validate(true)
        .encode()
        .expect("the probe componentizes")
}

/// The name of `interface` without its version.
fn unversioned(interface: &str) -> &str {
    interface
        .split_once('@')
        .map_or(interface, |(name, _)| name)
}

#[test]
fn a_guest_linker_defines_only_the_worlds_wasi_interfaces() {
    let mut resolve = Resolve::default();
    resolve
        .push_dir(nvs_repo::path("wit/nvs-ext"))
        .expect("the world's WIT parses");
    let interfaces = wasi_interfaces(&resolve);
    let probes: Vec<(String, Vec<u8>)> = interfaces
        .iter()
        .enumerate()
        .map(|(index, interface)| (interface.clone(), probe(&mut resolve, index, interface)))
        .collect();
    assert!(
        probes.len() > LINKED.len(),
        "the WIT holds the WASI interfaces the host does not link too"
    );
    let mut linked = BTreeSet::new();
    Host::new(1, |linker| {
        for (interface, bytes) in &probes {
            let component = Component::new(linker.engine(), bytes)?;
            if linker.instantiate_pre(&component).is_ok() {
                linked.insert(unversioned(interface).to_owned());
            }
        }
        Ok(())
    })
    .expect("the host starts");
    let expected: BTreeSet<String> = LINKED.iter().map(|&name| name.to_owned()).collect();
    assert_eq!(linked, expected);
}
