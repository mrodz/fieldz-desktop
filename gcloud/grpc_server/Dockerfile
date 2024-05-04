FROM rust:1.77-slim-buster AS builder
RUN apt-get update && apt install -y pkg-config libssl-dev ca-certificates build-essential cmake clang llvm-dev libclang-dev git protobuf-compiler gcc-multilib && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY ./backend ./backend
COPY ./gcloud/grpc_server ./gcloud/grpc_server
WORKDIR /usr/src/app/gcloud/grpc_server
RUN cargo build --release --features gcp --bin scheduler

FROM debian:buster-slim
COPY --from=builder /usr/src/app/gcloud/grpc_server/target/release /usr/local/bin

EXPOSE 50051

CMD [ "scheduler" ]