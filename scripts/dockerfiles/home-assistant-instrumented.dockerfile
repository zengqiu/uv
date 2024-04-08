FROM python:3.12-alpine
ENV LANG="C.UTF-8"
RUN apk add curl
ENV UV_EXTRA_INDEX_URL="https://wheels.home-assistant.io/musllinux-index/"
# docker run --rm -it -v .:/uv -v $(pwd)/../home-assistant:/home-assistant --entrypoint /bin/sh python:3.12-alpine
# export UV_EXTRA_INDEX_URL="https://wheels.home-assistant.io/musllinux-index/"
# /uv/target/x86_64-unknown-linux-musl/release/uv venv
# RUST_LOG=uv=info TRACING_DURATIONS_FILE=/uv/install1.jsonl /uv/target/x86_64-unknown-linux-musl/release/uv pip install --no-build -r /home-assistant/requirements.txt
# RUST_LOG=uv=info TRACING_DURATIONS_FILE=/uv/install2.jsonl /uv/target/x86_64-unknown-linux-musl/release/uv pip install --no-build -r /home-assistant/requirements_all.txt
# RUST_LOG=uv=info TRACING_DURATIONS_FILE=/uv/install3.jsonl /uv/target/x86_64-unknown-linux-musl/release/uv pip install -e /home-assistant
