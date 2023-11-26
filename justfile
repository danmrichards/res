default:
    just -l

release:
    cargo build -r

debug:
    cargo build

run:
    cargo run -- -r

fmt:
    cargo fmt

lint:
    cargo clippy --all-targets --all-features

test:
    RUST_BACKTRACE=1 cargo test --all-features