//! `rule:config/one-local-control-socket`'s endpoint with a real client on it, which is the only
//! place the accept half can be asked at all.
//!
//! What a connected client may *ask for* is `nvs-server`'s, and its tests drive that with no
//! endpoint at all. What is here is the transport underneath it: that a client reaches the
//! endpoint the tree named, that the stream an accept hands back carries bytes in both directions,
//! and that the next client is accepted once the stream before it is dropped — which is the shape
//! § 3's "operations serialize" rests on.
//!
//! Each case names an endpoint of its own, because the namespace is per platform and not per test:
//! a path under a directory of this case's own on Unix, and a pipe name carrying this process's id
//! on Windows, where there is no directory to be in. Each passes `bind` a guard that accepts, which
//! is what that seam is for — whether a directory is inside the trust boundary is
//! `rule:config/ownership-is-the-trust-boundary`'s question and `tests/trust.rs`'s subject, and a
//! system temporary directory is world-writable on most Unixes, so asking it here would assert
//! something about the host rather than about the transport.

use std::io::{Read, Write};
use std::path::PathBuf;

use nvs_config::control::{bind, connect};

/// The whole of `into` from a stream that answers `WouldBlock` rather than waiting, which is what
/// an accepted control stream does — see `nvs_config::control`'s module doc. In production the
/// waiting is the drive loop's, and here it is this.
fn filled(from: &mut impl Read, into: &mut [u8]) {
    let mut taken = 0;
    while taken < into.len() {
        match from.read(&mut into[taken..]) {
            Ok(0) => panic!("the client closed after {taken} of {} bytes", into.len()),
            Ok(read) => taken += read,
            Err(nothing) if nothing.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(failed) => panic!("the stream failed after {taken} bytes: {failed}"),
        }
    }
}

/// An endpoint name no other case and no other process is using.
fn endpoint(case: &str) -> PathBuf {
    let unique = std::process::id();
    #[cfg(windows)]
    {
        PathBuf::from(format!(r"\\.\pipe\nvs-control-{unique}-{case}"))
    }
    #[cfg(unix)]
    {
        let dir = std::env::temp_dir().join(format!("nvs-control-{unique}-{case}"));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).expect("a scratch directory of this case's own");
        dir.join("control.sock")
    }
}

/// Whatever a case left behind, which on Windows is nothing: the pipe went with its handle.
fn clean(name: &std::path::Path) {
    #[cfg(unix)]
    drop(std::fs::remove_dir_all(
        name.parent()
            .expect("the scratch directory the socket is in"),
    ));
    #[cfg(windows)]
    let _ = name;
}

/// A client asks and is answered over the one stream, which is the whole of what the accept owes
/// the operation loop above it.
#[test]
fn a_client_is_answered_on_the_stream_the_accept_hands_back() {
    let name = endpoint("answered");
    let endpoint = bind(&name, |_| Ok(())).unwrap_or_else(|why| panic!("{}", why.message()));

    let addressed = name.clone();
    let client = std::thread::spawn(move || {
        let mut connected = connect(&addressed).expect("a client reaches the endpoint it named");
        connected.write_all(b"ping").expect("the client's request");
        connected.flush().expect("the client's request, delivered");
        let mut answered = [0_u8; 4];
        connected
            .read_exact(&mut answered)
            .expect("the answer the server wrote");
        answered
    });

    let mut connected = endpoint.accept().expect("a client is on the endpoint");
    let mut asked = [0_u8; 4];
    filled(&mut connected, &mut asked);
    assert_eq!(&asked, b"ping", "the bytes arrive as they were sent");
    connected.write_all(b"pong").expect("the server's answer");
    connected.flush().expect("the server's answer, delivered");
    drop(connected);

    assert_eq!(
        &client.join().expect("the client thread"),
        b"pong",
        "the answer reaches the client before the stream is taken back",
    );
    clean(&name);
}

/// The next client is accepted once the stream before it is dropped — on Windows because the drop
/// is what gives the instance back, and on Unix because that is what the loop does. One endpoint
/// answers a run of clients, one at a time, which is `rule:config/one-local-control-socket`'s
/// serialization stated as a transport property.
#[test]
fn the_next_client_is_accepted_once_the_stream_before_it_is_dropped() {
    let name = endpoint("in-turn");
    let endpoint = bind(&name, |_| Ok(())).unwrap_or_else(|why| panic!("{}", why.message()));

    for round in *b"12" {
        let addressed = name.clone();
        let client = std::thread::spawn(move || {
            let mut connected =
                connect(&addressed).expect("a client reaches the endpoint it named");
            connected.write_all(&[round]).expect("the client's request");
            connected.flush().expect("the client's request, delivered");
            let mut answered = [0_u8; 1];
            connected
                .read_exact(&mut answered)
                .expect("the answer the server wrote");
            answered[0]
        });

        let mut connected = endpoint.accept().expect("a client is on the endpoint");
        let mut asked = [0_u8; 1];
        filled(&mut connected, &mut asked);
        assert_eq!(asked[0], round, "each round reads its own client's bytes");
        connected.write_all(&asked).expect("the server's answer");
        connected.flush().expect("the server's answer, delivered");
        drop(connected);

        assert_eq!(
            client.join().expect("the client thread"),
            round,
            "the client of this round is the one that was answered",
        );
    }

    clean(&name);
}
