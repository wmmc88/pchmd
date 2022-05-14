fn main() {
    std::process::Command::new("capnp")
        .args([
            "--version",
    ])
    .output()
    .expect("`capnp` binary not found! Try looking for and installing the `capnproto` package in your package manager(ex. sudo apt install capnproto).");
    
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/pchmd.capnp")
        .run().expect("`capnpc` failed to compile rust bindings for schemas!");

}


