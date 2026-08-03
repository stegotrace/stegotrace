.PHONY: sync check web-check install-local dev-api dev-web

sync:
	uv sync --all-extras

check:
	uv run ruff check src tests
	uv run pytest --cov=stegotrace --cov-report=term-missing
	cargo test --manifest-path cli/Cargo.toml
	cargo clippy --manifest-path cli/Cargo.toml --all-targets -- -D warnings

web-check:
	cd web && npm ci && npm run lint && npm run build

install-local:
	cargo install --path cli --force

dev-api:
	uv run uvicorn stegotrace.api:app --reload --port 8000

dev-web:
	cd web && npm run dev
