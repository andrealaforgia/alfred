FROM rust:latest

WORKDIR /alfred

RUN apt-get update && \
    apt-get install -y --no-install-recommends python3 python3-pip python3-venv procps && \
    rm -rf /var/lib/apt/lists/*

RUN python3 -m venv /opt/venv && \
    /opt/venv/bin/pip install --no-cache-dir pexpect pytest

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release --bin alfred
RUN cp target/release/alfred /usr/local/bin/alfred

COPY plugins/ /alfred/plugins/
COPY tests/e2e/test_alfred.py /alfred/tests/e2e/test_alfred.py

ENV PATH="/opt/venv/bin:$PATH"
ENV TERM=xterm-256color

CMD ["python3", "-m", "pytest", "-v", "/alfred/tests/e2e/test_alfred.py"]
