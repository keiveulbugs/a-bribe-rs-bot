fn main() {
    if cfg!(target_arch = "arm") {
        println!("cargo:rustc-link-lib=atomic");
    }
}
