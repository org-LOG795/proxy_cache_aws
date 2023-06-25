FROM rust:1-bullseye

COPY . .

RUN cargo build --release

CMD ["cargo", "run", "--release"]