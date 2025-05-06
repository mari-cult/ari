fn main() {
    let protos = &[
        "third_party/googleapis/google/ai/generativelanguage/v1beta/content.proto",
        "third_party/googleapis/google/ai/generativelanguage/v1beta/generative_service.proto",
        "third_party/googleapis/google/ai/generativelanguage/v1alpha/content.proto",
        "third_party/googleapis/google/ai/generativelanguage/v1alpha/generative_service.proto",
    ];

    let includes = &[
        "third_party/googleapis/",
        "third_party/googleapis/google/ai/generativelanguage/v1beta/",
        "third_party/googleapis/google/ai/generativelanguage/v1alpha/",
    ];

    tonic_build::configure()
        .build_server(false)
        .include_file("googleapis.rs")
        .compile_protos(protos, includes)
        .expect("compile protos, and generate code")
}
