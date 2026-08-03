from __future__ import annotations

import io
import zipfile
from pathlib import Path

import numpy as np
from PIL import Image

from stegotrace.engine import analyze_file, extract_artifact
from stegotrace.models import ScientificResult


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


def _embed_zsteg(
    payload: bytes,
    *,
    bit_depth: int = 1,
    channels: tuple[int, ...] = (0, 1, 2),
    reverse_rows: bool = False,
) -> bytes:
    bits = np.unpackbits(np.frombuffer(payload, dtype=np.uint8), bitorder="big")
    pixels = np.full((96, 96, 3), 128, dtype=np.uint8)
    slots_per_pixel = bit_depth * len(channels)
    for index, bit in enumerate(bits):
        pixel_index, slot = divmod(index, slots_per_pixel)
        y, x = divmod(pixel_index, pixels.shape[1])
        if reverse_rows:
            y = pixels.shape[0] - 1 - y
        channel, depth_index = divmod(slot, bit_depth)
        plane = bit_depth - 1 - depth_index
        target = channels[channel]
        pixels[y, x, target] = (int(pixels[y, x, target]) & ~(1 << plane)) | (int(bit) << plane)
    return _png_bytes(pixels)


def _zip_bytes() -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("evidence.txt", "hidden-data")
    return buffer.getvalue()


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
    payload = _zip_bytes()
    carrier = tmp_path / "lsb.png"
    carrier.write_bytes(_embed_lsb(payload))

    report = analyze_file(carrier)
    artifact = next(item for item in report.artifacts if item.kind == "lsb-zip")
    _, recovered = extract_artifact(carrier, artifact.id)

    assert recovered == payload


def test_unvalidated_lsb_signatures_are_not_reported(tmp_path: Path) -> None:
    carrier = tmp_path / "false-signatures.png"
    carrier.write_bytes(_embed_lsb(b"MZ" + b"x" * 32 + b"\xff\xd8\xffnot-a-jpeg"))

    report = analyze_file(carrier)

    assert not [artifact for artifact in report.artifacts if artifact.kind.startswith("lsb-")]


def test_png_without_scientific_profile_abstains(monkeypatch, tmp_path: Path) -> None:
    monkeypatch.setattr(
        "stegotrace.engine.analyze_with_aletheia",
        lambda _path: ScientificResult(False, "Aletheia", limitation="not configured"),
    )
    carrier = tmp_path / "clean.png"
    carrier.write_bytes(_png_bytes(np.zeros((16, 16, 3), dtype=np.uint8)))

    report = analyze_file(carrier)

    assert report.verdict == "Análisis no concluyente sin perfil científico"
    assert any("no puede considerarse negativo" in limitation for limitation in report.limitations)


def test_openstego_header_is_detected_and_recovered(tmp_path: Path) -> None:
    envelope = b"OPENSTEGO" + bytes([1]) + (4).to_bytes(4, "little") + bytes([1, 8, 1, 1])
    envelope += b"flag.txt" + b"data"
    carrier = tmp_path / "openstego.png"
    carrier.write_bytes(_embed_zsteg(envelope))

    report = analyze_file(carrier)
    artifact = next(item for item in report.artifacts if item.kind == "openstego")
    _, recovered = extract_artifact(carrier, artifact.id)

    assert report.score >= 75
    assert recovered == envelope


def test_wbstego_plain_text_is_detected_and_recovered(tmp_path: Path) -> None:
    message = b"SuperSecretMessage\n"
    envelope = (len(message) + 3).to_bytes(3, "little") + b"txt" + message
    carrier = tmp_path / "wbstego.png"
    carrier.write_bytes(_embed_zsteg(envelope, channels=(2, 1, 0), reverse_rows=True))

    report = analyze_file(carrier)
    artifact = next(item for item in report.artifacts if item.kind == "wbstego-text")
    _, recovered = extract_artifact(carrier, artifact.id)

    assert report.score >= 75
    assert recovered == message


def test_multibit_lsb_text_is_detected_and_recovered(tmp_path: Path) -> None:
    message = b"SuperSecretMessage"
    carrier = tmp_path / "rgb3.png"
    carrier.write_bytes(_embed_zsteg(message + b"\x00" * 4, bit_depth=3))

    report = analyze_file(carrier)
    artifact = next(item for item in report.artifacts if item.kind == "lsb-text")
    _, recovered = extract_artifact(carrier, artifact.id)

    assert report.score >= 75
    assert recovered == message
