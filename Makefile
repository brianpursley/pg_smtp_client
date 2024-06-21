PGRX_POSTGRES ?= pg16

build:
	trunk build

init:
	cargo install --locked cargo-pgrx 
	cargo pgrx init

test: build
	@command -v python3 >/dev/null 2>&1 || { echo >&2 "python3 is required but it's not installed.  Aborting."; exit 1; }
	@trap 'kill `cat /tmp/smtpd.pid`' EXIT; \
	python3 -W ignore::DeprecationWarning -m smtpd -n -c DebuggingServer 127.0.0.1:2525 & echo $$! > /tmp/smtpd.pid; \
	cargo pgrx test $(PGRX_POSTGRES)

fmt:
	cargo fmt

run:
	cargo pgrx run
