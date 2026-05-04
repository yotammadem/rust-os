fn main() {
    if std::env::var("TARGET").as_deref() == Ok("x86_64-unknown-none") {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir");
        println!("cargo:rustc-link-arg=-T{manifest_dir}/../linker/kernel.ld");
    }
}
