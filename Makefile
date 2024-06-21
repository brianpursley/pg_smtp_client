PYTHON := $(shell command -v python3 || command -v python || echo "none")
PIP3 := $(shell command -v pip3 || command -v pip || echo "none")

.PHONY: build
build:
	@command -v trunk >/dev/null 2>&1 || { echo >&2 "Error: trunk is required (cargo install pg-trunk)."; exit 1; }
	trunk build

.PHONY: init
init:
	cargo install --locked cargo-pgrx 
	cargo pgrx init --pg16 download

.PHONY: test
test:
	@if [ "$(PYTHON)" = "none" ]; then echo >&2 "Error: python3 is required."; exit 1; fi
	@if [ "$(PIP)" = "none" ]; then echo >&2 "Error: pip3 is required."; exit 1; fi
	$(PIP3) install aiosmtpd
	trap 'kill `cat /tmp/smtpd.pid`' EXIT; \
	$(PYTHON) -m aiosmtpd -n & echo $$! > /tmp/smtpd.pid; \
	cargo pgrx test pg16

.PHONY: run
run:
	cargo pgrx run

.PHONY: clean
clean:
	rm -rf .trunk
