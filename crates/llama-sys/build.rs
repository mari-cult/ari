use std::env;

macro_rules! source {
    () => {
        "../../subprojects/llama.cpp/"
    };
    ($path:literal) => {
        concat!(source!(), $path)
    };
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Backend {
    #[default]
    Cpu,
    Clblast,
    Cublas,
    Hipblas,
}

impl Backend {
    pub fn from_env() -> Self {
        let clblast = has_feature("clblast");
        let cublas = has_feature("cublas");
        let hipblas = has_feature("hipblas");

        match (clblast, cublas, hipblas) {
            (true, false, false) => Self::Clblast,
            (false, true, false) => Self::Cublas,
            (false, false, true) => Self::Hipblas,
            (false, false, false) => Self::Cpu,
            _ => panic!("Cannot use multiple backends at once"),
        }
    }
}

fn build_common() -> cc::Build {
    let mut build = cc::Build::new();

    build
        .define("_GNU_SOURCE", None)
        .define("_XOPEN_SOURCE", "600")
        .flag_if_supported("-Wno-unused")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-pthread")
        .include(source!());

    build
}

fn has_feature(feature: &str) -> bool {
    let feature = feature.to_uppercase();
    let key = format!("CARGO_FEATURE_{feature}");

    let value = env::var_os(&key);

    println!("{key} = {value:?}");

    value.is_some()
}

fn link(name: &str) {
    println!("cargo:rustc-link-lib={name}");
}

fn main() {
    println!("cargo:rerun-if-changed=src/bindings.cpp");

    let backend = Backend::from_env();
    let mut c = build_common();
    let mut cxx = build_common();

    c.file(source!("ggml-alloc.c"))
        .file(source!("ggml-backend.c"))
        .file(source!("ggml-quants.c"))
        .file(source!("ggml.c"));

    cxx.cpp(true)
        .file("src/bindings.cpp")
        .file(source!("common/common.cpp"))
        .file(source!("common/grammar-parser.cpp"))
        .file(source!("common/sampling.cpp"))
        .file(source!("examples/llava/clip.cpp"))
        .file(source!("llama.cpp"))
        .include(source!("common"))
        .include(source!("examples/llava"));

    match backend {
        Backend::Clblast => {
            c.define("GGML_USE_CLBLAST", "1");
            cxx.define("GGML_USE_CLBLAST", "1")
                .file(source!("ggml-opencl.cpp"));

            link("clblast");
            link("OpenCL");
        }
        Backend::Cublas => {
            link("cublas");
        }
        Backend::Hipblas => {
            link("hipblas");
        }
        _ => {}
    }

    c.compile("bindings-c");
    cxx.compile("bindings-cxx");
}
