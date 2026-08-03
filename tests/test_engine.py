from __future__ import annotations

import io
from pathlib import Path

import numpy as np
from PIL import Image

from stegotrace.engine import analyze_file, extract_artifact


def _png_bytes(pixels: np.ndarray) -> bytes:
    buffer = io.BytesIO()
    Image.fromarray(pixels, "RGB").save(buffer, format="PNG")
    return buffer.getvalue()


def _embed_lsb(payload: bytes) -> bytes:
    bits = np.unpackbits(np.frombuffer(payload, dtype=np.uint8), bitorder="big")
    pixels = np.full((96, 96, 3), 128, dtype=np.uint8)
    flat = pixels.reshape(-1)
    flat[: len(bits)] = (flat[: len(bits)] & 0xFE) | bits
    return _png_bytes(pixels)


def test_trailing_payload_is_extracted_and_verified(tmp_path: Path) -> None:
    carrier = tmp_path / "carrier.png"
    payload = b"PK\x03\x04" + b"evidence" * 5
    carrier.write_bytes(_png_bytes(np.zeros((16, 16, 3), dtype=np.uint8)) + payload)

    report = analyze_file(carrier)
    artifact = next(item for item in report.artifacts if item.kind == "zip")
    metadata, recovered = extract_artifact(carrier, artifact.id)

    assert recovered == payload
    assert metadata.sha256 == report.artifacts[0].sha256
    assert report.score >= 75


def test_signature_anchored_lsb_stream_is_recovered(tmp_path: Path) -> None:
    payload = b"PK\x03\x04" + b"hidden-data"
    carrier = tmp_path / "lsb.png"
    carrier.write_bytes(_embed_lsb(payload))

    report = analyze_file(carrier)
    artifact = next(item for item in report.artifacts if item.kind == "lsb-zip")
    _, recovered = extract_artifact(carrier, artifact.id)

    assert recovered.startswith(payload)

