PYTHON := $(shell command -v python3 || command -v python || echo "none")
PIP3 := $(shell command -v pip3 || command -v pip || echo "none")
PG_VERSION := 16

.PHONY: run
run:
	cargo pgrx run

.PHONY: clean
clean:
	cargo clean
	
.PHONY: init
init:
	cargo install cargo-pgrx pg-trunk
	cargo pgrx init --pg$(PG_VERSION) download

.PHONY: lint
lint:
	cargo fmt --check
	cargo clippy

.PHONY: test
test:
	@if [ "$(PYTHON)" = "none" ]; then echo >&2 "Error: python3 is required."; exit 1; fi
	@if [ "$(PIP)" = "none" ]; then echo >&2 "Error: pip3 is required."; exit 1; fi
	$(PIP3) install aiosmtpd
	trap 'kill `cat /tmp/smtpd.pid`' EXIT; \
	$(PYTHON) -m aiosmtpd -n & echo $$! > /tmp/smtpd.pid; \
	cargo pgrx test pg$(PG_VERSION)
