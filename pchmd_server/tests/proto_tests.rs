#[test]
fn match_capnproto_and_cargo_version() {
    assert_eq!(
        pchmd_server::pchmd_capnp::MAJOR_VERSION.to_string(),
        env!("CARGO_PKG_VERSION_MAJOR")
    );
    assert_eq!(
        pchmd_server::pchmd_capnp::MINOR_VERSION.to_string(),
        env!("CARGO_PKG_VERSION_MINOR")
    );
    assert_eq!(
        pchmd_server::pchmd_capnp::PATCH_VERSION.to_string(),
        env!("CARGO_PKG_VERSION_PATCH")
    );
}
