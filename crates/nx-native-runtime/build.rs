fn main() {
    println!("cargo:rerun-if-changed=c/runtime.c");
    println!("cargo:rerun-if-changed=c/runtime.h");

    cc::Build::new()
        .file("c/runtime.c")
        .include("c")
        .opt_level(3)
        .compile("nx_native_runtime");
}
