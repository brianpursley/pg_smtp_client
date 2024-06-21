build:
	cargo fmt --check
	cargo clippy
	@command -v trunk >/dev/null 2>&1 || { echo >&2 "trunk is required but it's not installed (cargo install pg-trunk).  Aborting."; exit 1; }
	trunk build

init:
	cargo install --locked cargo-pgrx 
	cargo pgrx init --pg16 download

test:
	cargo build
	@command -v python >/dev/null 2>&1 || { echo >&2 "python is required but it's not installed.  Aborting."; exit 1; }
	@trap 'kill `cat /tmp/smtpd.pid`' EXIT; \
	python3 -W ignore::DeprecationWarning -m smtpd -n -c DebuggingServer 127.0.0.1:2525 & echo $$! > /tmp/smtpd.pid; \
	cargo pgrx test pg16

run:
	cargo pgrx run

clean:
	rm -rf .trunk