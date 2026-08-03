from __future__ import annotations

import heapq
from dataclasses import dataclass
from pathlib import Path

import jpeglib
import numpy as np
from scipy.fft import dctn, idctn

from .models import Finding


@dataclass(slots=True)
class CompatibilityResult:
    finding: Finding
    score: float


def _jpeg_round(values: np.ndarray) -> np.ndarray:
    return np.where(values >= 0, np.floor(values + 0.5), np.ceil(values - 0.5)).astype(np.int16)


def _forward(block: np.ndarray, quantization: np.ndarray) -> np.ndarray:
    coefficients = dctn(block.astype(float) - 128, axes=(-2, -1), norm="ortho") / quantization
    return _jpeg_round(coefficients)


def _backward(target: np.ndarray, quantization: np.ndarray) -> np.ndarray:
    pixels = idctn(target * quantization, axes=(-2, -1), norm="ortho") + 128
    return _jpeg_round(pixels).clip(0, 255)


def _error(candidate: np.ndarray, target: np.ndarray, quantization: np.ndarray) -> int:
    return int(np.abs(_forward(candidate, quantization) - target).sum())


def _find_antecedent(target: np.ndarray, quantization: np.ndarray, max_iterations: int) -> tuple[str, int]:
    start = _backward(target, quantization)
    start_error = _error(start, target, quantization)
    if start_error == 0:
        return "compatible", 0
    queue: list[tuple[int, int, bytes, np.ndarray]] = [(start_error, 0, start.tobytes(), start)]
    visited = {start.tobytes()}
    serial = 0
    for iteration in range(1, max_iterations + 1):
        if not queue:
            return "incompatible", iteration - 1
        _, _, _, current = heapq.heappop(queue)
        flat = current.reshape(-1)
        for index in range(64):
            for delta in (-1, 1):
                value = int(flat[index]) + delta
                if not 0 <= value <= 255:
                    continue
                child = current.copy()
                child.reshape(-1)[index] = value
                key = child.tobytes()
                if key in visited:
                    continue
                visited.add(key)
                error = _error(child, target, quantization)
                if error == 0:
                    return "compatible", iteration
                serial += 1
                heapq.heappush(queue, (error, serial, key, child))
        if len(queue) > 4096:
            queue = heapq.nsmallest(2048, queue)
            heapq.heapify(queue)
    return "timeout", max_iterations


def analyze_jpeg_compatibility(
    path: str | Path,
    *,
    max_blocks: int = 32,
    max_iterations: int = 32,
) -> CompatibilityResult:
    """Run a bounded blind antecedent search from Levecque, Butora & Bas (TIFS 2024).

    The paper's calibrated LRT needs pipeline-specific likelihood tables. This online variant
    reports the raw compatible/timeout evidence and only treats queue exhaustion as proof under
    the tested mathematical pipeline.
    """
    image = jpeglib.read_dct(str(path))
    quantization = np.asarray(image.qt[0], dtype=np.int16)
    quality_100 = bool(np.all(quantization == 1))
    if not quality_100:
        return CompatibilityResult(
            Finding(
                "frontier.jpeg-compatibility-not-applicable",
                "frontier",
                "Compatibilidad JPEG de baja carga",
                "info",
                "bounded-jpeg-antecedent-search",
                {
                    "quantization_min": int(quantization.min()),
                    "quantization_max": int(quantization.max()),
                    "qf100_equivalent": False,
                },
                "La propiedad publicada es discriminante en QF100; este JPEG usa otra cuantización y no se clasifica.",
                100,
            ),
            0,
        )
    blocks = np.asarray(image.Y, dtype=np.int16).reshape(-1, 8, 8)
    reconstructed = idctn(blocks * quantization, axes=(-2, -1), norm="ortho") + 128
    rounding_variance = np.var(reconstructed - _jpeg_round(reconstructed), axis=(1, 2))
    selected_indices = np.argsort(rounding_variance)[::-1][: min(max_blocks, len(blocks))]
    statuses = {"compatible": 0, "timeout": 0, "incompatible": 0}
    iterations: list[int] = []
    for index in selected_indices:
        status, count = _find_antecedent(blocks[index], quantization, max_iterations)
        statuses[status] += 1
        iterations.append(count)
    selected = max(len(selected_indices), 1)
    timeout_rate = statuses["timeout"] / selected
    proven = statuses["incompatible"]
    severity = "high" if proven else "medium" if timeout_rate >= 0.25 else "info"
    confidence = 96 if proven else min(68, round(35 + 60 * timeout_rate))
    interpretation = (
        "Se agotó el espacio de búsqueda para al menos un bloque: incompatible bajo la DCT matemática probada."
        if proven
        else "Los timeouts son una variable para el LRT publicado, no una prueba individual; "
        "faltan tablas de verosimilitud de la tubería origen."
    )
    return CompatibilityResult(
        Finding(
            "frontier.jpeg-compatibility",
            "frontier",
            "Búsqueda de antecedentes JPEG QF100",
            severity,
            "bounded-jpeg-antecedent-search",
            {
                "selected_blocks": len(selected_indices),
                "total_luma_blocks": len(blocks),
                "selection": "highest-rounding-error-variance",
                "max_iterations": max_iterations,
                "statuses": statuses,
                "timeout_rate": round(timeout_rate, 6),
                "mean_iterations": round(float(np.mean(iterations)), 3) if iterations else 0,
            },
            interpretation,
            confidence,
        ),
        100.0 if proven else min(55.0, timeout_rate * 100),
    )
