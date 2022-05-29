use pchmd_server::*;

#[test]
fn match_capnproto_and_cargo_version() {
    // ensure that the version hard-coded in pchmd.capnp stays synced with server crate version

    assert_eq!(
        pchmd_capnp::MAJOR_VERSION.to_string(),
        env!("CARGO_PKG_VERSION_MAJOR")
    );
    assert_eq!(
        pchmd_capnp::MINOR_VERSION.to_string(),
        env!("CARGO_PKG_VERSION_MINOR")
    );
    assert_eq!(
        pchmd_capnp::PATCH_VERSION.to_string(),
        env!("CARGO_PKG_VERSION_PATCH")
    );
}
