
platform_dirs::AppDirs::new(env!("CARGO_PKG_NAME"))

fn main() {
    if has_existing_ssl_certificate()    {
        if !should_overrite(){
            return
        }
    }

    let (pem, pem_private_key) = generate_certificate();
    save_certificate();
}

fn has_existing_ssl_certificate() -> Bool {
    true
}

fn generate_certificate() -> (String, String) {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let pem = cert.serialize_pem().unwrap();
    let pem_private_key= cert.serialize_private_key_pem();
    (pem, pem_private_key)
}

fn save_certificate() {
    // println! ("cert pem:\n{pem}"); .crt
    // println! ("private key pem:\n{pem_private_key}"); .key
}
