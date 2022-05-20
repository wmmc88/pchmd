fn main() {
    // TODO: put all rerun-if statements at beginning.
    println!("cargo:rerun-if-changed=build.rs");

    // TODO: make each datasource dep an optional cargo feature

    // TODO: put all pre-requisite checks before other build steps
    // TODO: convert pre-req checks to use pkg-config/cmake etc
    // TODO: commonize/split up os-specific build steps

    generate_capnproto_files();
    generate_librehardwaremonitor_bindings();
}

fn generate_capnproto_files() {
    println!("cargo:rerun-if-changed=schema/pchmd.capnp");

    std::process::Command::new("capnp")
        .args([
            "--version",
        ])
        .output()
        .expect("`capnp` binary not found! Try looking for and installing the `capnproto` package in your package manager(ex. sudo apt install capnproto).");

    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/pchmd.capnp")
        .run()
        .expect("`capnpc` failed to compile rust bindings for schemas!");
}

fn generate_librehardwaremonitor_bindings() {}
