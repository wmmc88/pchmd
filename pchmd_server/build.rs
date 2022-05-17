use std::env;
use std::path::PathBuf;

fn main() {
    // TODO: put all rerun-if statements at beginning.
    println!("cargo:rerun-if-changed=build.rs");

    // TODO: put all pre-requisite checks before other build steps
    // TODO: convert pre-req checks to use pkg-config/cmake etc
    // TODO: commonize/split up os-specific build steps

    generate_capnproto_files();
    generate_libsensors_bindings();
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

fn generate_libsensors_bindings() {
    // TODO: check if libsensors is available (aka whether this system can poll libsensors)
    // TODO: check if libsensors-dev is available

    println!("cargo:rustc-link-lib=sensors");
    println!("cargo:rerun-if-changed=src/libsensors-wrapper.h");

    let bindings = bindgen::Builder::default()
        .header("src/libsensors-wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate rust bindings for libsensors");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("libsensors-bindings.rs"))
        .expect("Couldn't write bindings for libsensors!");
}

fn generate_librehardwaremonitor_bindings() {}
