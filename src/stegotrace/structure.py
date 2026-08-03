from __future__ import annotations

import hashlib
import mimetypes
import struct
from dataclasses import dataclass, field
from pathlib import Path

from .models import Artifact, Finding

SIGNATURES = (
    (b"PK\x03\x04", "zip", "application/zip"),
    (b"%PDF-", "pdf", "application/pdf"),
    (b"\x89PNG\r\n\x1a\n", "png", "image/png"),
    (b"\xff\xd8\xff", "jpeg", "image/jpeg"),
    (b"7z\xbc\xaf'\x1c", "7z", "application/x-7z-compressed"),
    (b"Rar!\x1a\x07", "rar", "application/vnd.rar"),
    (b"\x1f\x8b\x08", "gzip", "application/gzip"),
    (b"MZ", "pe", "application/vnd.microsoft.portable-executable"),
    (b"\x7fELF", "elf", "application/x-elf"),
)


@dataclass(slots=True)
class StructureResult:
    media_type: str
    format_name: str
    canonical_end: int | None
    findings: list[Finding] = field(default_factory=list)
    artifacts: list[Artifact] = field(default_factory=list)
    png_chunks: list[str] = field(default_factory=list)


def identify(data: bytes, filename: str) -> tuple[str, str]:
    if data.startswith(b"\x89PNG\r\n\x1a\n"):
        return "image/png", "png"
    if data.startswith(b"\xff\xd8\xff"):
        return "image/jpeg", "jpeg"
    if data.startswith((b"GIF87a", b"GIF89a")):
        return "image/gif", "gif"
    if data.startswith(b"RIFF") and data[8:12] == b"WAVE":
        return "audio/wav", "wav"
    if data.startswith(b"%PDF-"):
        return "application/pdf", "pdf"
    if data.startswith(b"PK\x03\x04"):
        return "application/zip", "zip"
    guessed = mimetypes.guess_type(filename)[0]
    return guessed or "application/octet-stream", "unknown"


def _artifact(
    data: bytes,
    *,
    kind: str,
    start: int,
    end: int,
    filename: str,
    mime: str,
    description: str,
) -> Artifact:
    payload = data[start:end]
    artifact_id = hashlib.sha256(f"slice:{start}:{end}:{kind}".encode()).hexdigest()[:16]
    return Artifact(
        id=artifact_id,
        kind=kind,
        suggested_name=f"{Path(filename).stem}-recovered.{kind}",
        size=len(payload),
        sha256=hashlib.sha256(payload).hexdigest(),
        description=description,
        extractor="slice",
        parameters={"start": start, "end": end},
        mime=mime,
    )


def _png_end(data: bytes) -> tuple[int | None, list[str], list[tuple[str, int]]]:
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        return None, [], []
    offset = 8
    chunks: list[str] = []
    text_chunks: list[tuple[str, int]] = []
    while offset + 12 <= len(data):
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        chunk_type = data[offset + 4 : offset + 8].decode("latin-1")
        chunk_end = offset + 12 + length
        if chunk_end > len(data):
            return None, chunks, text_chunks
        chunks.append(chunk_type)
        if chunk_type in {"tEXt", "zTXt", "iTXt", "eXIf"}:
            text_chunks.append((chunk_type, length))
        offset = chunk_end
        if chunk_type == "IEND":
            return offset, chunks, text_chunks
    return None, chunks, text_chunks


def _canonical_end(data: bytes, format_name: str) -> tuple[int | None, list[str], list]:
    if format_name == "png":
        return _png_end(data)
    if format_name == "jpeg":
        marker = data.find(b"\xff\xd9", 2)
        return (marker + 2 if marker >= 0 else None), [], []
    if format_name == "gif":
        marker = data.rfind(b"\x3b")
        return (marker + 1 if marker >= 0 else None), [], []
    if format_name == "wav" and len(data) >= 8:
        declared = 8 + struct.unpack("<I", data[4:8])[0]
        return min(declared, len(data)), [], []
    if format_name == "pdf":
        marker = data.rfind(b"%%EOF")
        if marker < 0:
            return None, [], []
        end = marker + 5
        while end < len(data) and data[end] in b"\r\n \t":
            end += 1
        return end, [], []
    return None, [], []


def analyze_structure(data: bytes, filename: str) -> StructureResult:
    media_type, format_name = identify(data, filename)
    canonical_end, chunks, text_chunks = _canonical_end(data, format_name)
    result = StructureResult(media_type, format_name, canonical_end, png_chunks=chunks)

    if canonical_end is not None and canonical_end < len(data):
        trailing = data[canonical_end:]
        if trailing.strip(b"\x00\r\n \t"):
            result.findings.append(
                Finding(
                    "structure.trailing-data",
                    "structure",
                    "Datos después del final canónico",
                    "high",
                    "container-boundary",
                    {"offset": canonical_end, "bytes": len(trailing)},
                    "El contenedor termina antes que el archivo; hay bytes adicionales recuperables.",
                    96,
                )
            )
            signature = next(
                ((kind, mime) for sig, kind, mime in SIGNATURES if trailing.startswith(sig)),
                ("bin", "application/octet-stream"),
            )
            kind, mime = signature
            result.artifacts.append(
                _artifact(
                    data,
                    kind=kind,
                    start=canonical_end,
                    end=len(data),
                    filename=filename,
                    mime=mime,
                    description="Bytes anexos detectados tras el final canónico del contenedor.",
                )
            )

    for chunk_type, length in text_chunks:
        severity = "medium" if length > 4096 else "low"
        result.findings.append(
            Finding(
                f"structure.png-{chunk_type.lower()}",
                "structure",
                f"Chunk PNG {chunk_type}",
                severity,
                "png-chunk-walk",
                {"chunk": chunk_type, "bytes": length},
                "Un chunk de metadatos puede contener texto o datos; su presencia no prueba ocultación.",
                72 if severity == "medium" else 45,
            )
        )

    seen_offsets = {artifact.parameters["start"] for artifact in result.artifacts}
    search_start = max(16, canonical_end or 16)
    for signature, kind, mime in SIGNATURES:
        offset = data.find(signature, search_start)
        if offset < 0 or offset in seen_offsets:
            continue
        seen_offsets.add(offset)
        result.findings.append(
            Finding(
                f"structure.embedded-{kind}-{offset}",
                "structure",
                f"Firma {kind.upper()} embebida",
                "high",
                "signature-carving",
                {"offset": offset, "signature": signature.hex()},
                "Se encontró una cabecera conocida fuera de la cabecera principal del archivo.",
                90,
            )
        )
        result.artifacts.append(
            _artifact(
                data,
                kind=kind,
                start=offset,
                end=len(data),
                filename=filename,
                mime=mime,
                description=f"Flujo {kind.upper()} tallado desde una firma embebida.",
            )
        )
    return result
