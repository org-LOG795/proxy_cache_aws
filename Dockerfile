FROM rust:1-bullseye

COPY . .

RUN export IMAGE_VERSION=$(cat image_version)
RUN rm -f image_version

RUN cargo build --release

CMD ["cargo", "run", "--release"]