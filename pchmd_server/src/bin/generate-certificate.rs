use rcgen;

fn main() {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let pem = cert.serialize_pem().unwrap();
    let pem_private_key= cert.serialize_private_key_pem();

    println! ("cert pem:\n{pem}");
    println! ("private key pem:\n{pem_private_key}");
}
