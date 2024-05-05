FROM rust:1.77-slim-buster AS builder
RUN apt-get update && apt-get -y install protobuf-compiler

WORKDIR /usr/src/app
COPY ./backend ./backend
COPY ./gcloud/grpc_server ./gcloud/grpc_server
WORKDIR /usr/src/app/gcloud/grpc_server
RUN cargo build --release --features gcp --bin scheduler

FROM debian:buster-slim
COPY --from=builder /usr/src/app/gcloud/grpc_server/target/release/scheduler /usr/local/bin

EXPOSE 50051

CMD [ "scheduler" ]