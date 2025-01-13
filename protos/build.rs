extern crate prost_build;

fn main() {
    prost_build::compile_protos(
        &[
            "src/protos/file.proto",
            "src/protos/requests.proto",
            "src/protos/responses.proto",
        ],
        &["src/protos"],
    )
    .unwrap();
}
