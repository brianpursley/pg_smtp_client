PYTHON ?= $(shell which python3 || which python)
PIP ?= $(shell which pip3 || which pip)
PG_VERSION ?= 16

.PHONY: run
run:
	@cargo pgrx run

.PHONY: clean
clean:
	@cargo clean
	
.PHONY: init
init:
	@cargo install --locked cargo-pgrx
	@cargo pgrx init --pg$(PG_VERSION) download

.PHONY: lint
lint:
	@cargo fmt --check
	@cargo clippy

.PHONY: test
test:
	@if [ -z "$(PYTHON)" ]; then echo "python3 or python not found"; exit 1; fi
	@if [ -z "$(PIP)" ]; then echo "pip3 or pip not found"; exit 1; fi
	@$(PIP) install aiosmtpd
	@trap 'kill `cat /tmp/smtpd.pid`' EXIT; \
	$(PYTHON) -m aiosmtpd -n & echo $$! > /tmp/smtpd.pid; \
	cargo pgrx test pg$(PG_VERSION)
