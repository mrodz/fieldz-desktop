fn main() {
    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .include("protos")
        .inputs(["protos/algo_input.proto"])
        .out_dir("src/protos")
        .run_from_script();
}
