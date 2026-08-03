.PHONY: sync check web-check install-local dev-api dev-web

sync:
	uv sync --all-extras

check:
	uv run ruff check src tests
	uv run pytest --cov=stegotrace --cov-report=term-missing
	cargo test --manifest-path cli/Cargo.toml
	cargo clippy --manifest-path cli/Cargo.toml --all-targets -- -D warnings

web-check:
	cmp -s install.sh web/public/install.sh
	sh -n install.sh
	cd web/public/cli/v0.2.0 && shasum -a 256 -c stegotrace-aarch64-apple-darwin.tar.gz.sha256
	cd web/public/cli/v0.2.0 && shasum -a 256 -c stegotrace-x86_64-apple-darwin.tar.gz.sha256
	cd web && npm ci && npm run lint && npm run build

install-local:
	cargo install --path cli --force

dev-api:
	uv run uvicorn stegotrace.api:app --reload --port 8000

dev-web:
	cd web && npm run dev
