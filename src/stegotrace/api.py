from __future__ import annotations

import os
import tempfile
import time
from collections import defaultdict, deque
from pathlib import Path

from fastapi import FastAPI, File, Form, HTTPException, Request, UploadFile
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import Response

from . import __version__
from .engine import analyze_file, extract_artifact

MAX_FILE_BYTES = int(os.getenv("STEGOTRACE_MAX_FILE_BYTES", str(25 * 1024 * 1024)))
RATE_LIMIT = int(os.getenv("STEGOTRACE_RATE_LIMIT", "20"))
ALLOWED_ORIGINS = [
    item.strip() for item in os.getenv("STEGOTRACE_ALLOWED_ORIGINS", "http://localhost:5173").split(",") if item.strip()
]
CHUNK_SIZE = 1024 * 1024

app = FastAPI(title="StegoTrace API", version=__version__, docs_url="/docs", redoc_url=None)
app.add_middleware(
    CORSMiddleware,
    allow_origins=ALLOWED_ORIGINS,
    allow_credentials=False,
    allow_methods=["GET", "POST"],
    allow_headers=["Content-Type"],
    expose_headers=["Content-Disposition", "X-Content-SHA256"],
)

_requests: dict[str, deque[float]] = defaultdict(deque)


def _client_key(request: Request) -> str:
    return request.headers.get("cf-connecting-ip") or (request.client.host if request.client else "unknown")


def _check_rate_limit(request: Request) -> None:
    now = time.monotonic()
    bucket = _requests[_client_key(request)]
    while bucket and bucket[0] < now - 60:
        bucket.popleft()
    if len(bucket) >= RATE_LIMIT:
        raise HTTPException(429, "Límite temporal alcanzado; inténtalo de nuevo en un minuto")
    bucket.append(now)


async def _save_upload(upload: UploadFile) -> Path:
    suffix = Path(upload.filename or "upload.bin").suffix[:16]
    total = 0
    handle = tempfile.NamedTemporaryFile(prefix="stegotrace-", suffix=suffix, delete=False)
    path = Path(handle.name)
    try:
        with handle:
            while chunk := await upload.read(CHUNK_SIZE):
                total += len(chunk)
                if total > MAX_FILE_BYTES:
                    raise HTTPException(413, f"El archivo supera el máximo de {MAX_FILE_BYTES} bytes")
                handle.write(chunk)
        if not total:
            raise HTTPException(400, "El archivo está vacío")
        return path
    except Exception:
        path.unlink(missing_ok=True)
        raise
    finally:
        await upload.close()


@app.get("/health")
def health() -> dict:
    return {
        "ok": True,
        "service": "stegotrace-api",
        "version": __version__,
        "region": os.getenv("RAILWAY_REPLICA_REGION", "local"),
    }


@app.get("/v1/methods")
def methods() -> dict:
    return {
        "native": ["structure", "signatures", "chi-square", "RS", "entropy", "bit-planes", "PCM-LSB"],
        "scientific_provider": "Aletheia",
        "max_file_bytes": MAX_FILE_BYTES,
    }


@app.post("/v1/analyze")
async def analyze(request: Request, file: UploadFile = File(...)) -> dict:
    _check_rate_limit(request)
    filename = file.filename or "upload.bin"
    path = await _save_upload(file)
    try:
        return analyze_file(path, filename=filename).to_dict()
    except (ValueError, OSError) as error:
        raise HTTPException(422, str(error)) from error
    finally:
        path.unlink(missing_ok=True)


@app.post("/v1/extract")
async def extract(request: Request, artifact_id: str = Form(...), file: UploadFile = File(...)) -> Response:
    _check_rate_limit(request)
    filename = file.filename or "upload.bin"
    path = await _save_upload(file)
    try:
        artifact, payload = extract_artifact(path, artifact_id, filename=filename)
        return Response(
            payload,
            media_type="application/octet-stream",
            headers={
                "Content-Disposition": f'attachment; filename="{artifact.suggested_name}"',
                "X-Content-SHA256": artifact.sha256,
                "X-Content-Type-Options": "nosniff",
            },
        )
    except KeyError as error:
        raise HTTPException(404, str(error)) from error
    except (ValueError, OSError) as error:
        raise HTTPException(422, str(error)) from error
    finally:
        path.unlink(missing_ok=True)
