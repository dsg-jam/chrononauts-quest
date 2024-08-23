FROM rust:1-alpine3.20 AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.lock Cargo.toml ./
COPY api api
COPY src src

RUN cargo install --path .

FROM alpine:3.20

COPY --from=builder /usr/local/cargo/bin/backend /backend

CMD [ "/backend" ]
