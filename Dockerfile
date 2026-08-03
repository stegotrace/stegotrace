FROM python:3.12-slim AS builder
WORKDIR /app
RUN pip install --no-cache-dir uv==0.8.3
COPY pyproject.toml README.md LICENSE ./
COPY src ./src
RUN uv venv /opt/venv && uv pip install --python /opt/venv/bin/python '.[api]'

FROM python:3.12-slim
ENV PATH="/opt/venv/bin:$PATH" PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1 PORT=8000
RUN useradd --create-home --uid 10001 stegotrace
COPY --from=builder /opt/venv /opt/venv
USER stegotrace
EXPOSE 8000
CMD ["sh", "-c", "uvicorn stegotrace.api:app --host 0.0.0.0 --port ${PORT}"]

