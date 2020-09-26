FROM rust:latest

WORKDIR /usr/src/thissy
COPY . .

RUN cargo install --path .

CMD ["listentothissy"]
