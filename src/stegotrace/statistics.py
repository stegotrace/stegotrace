from __future__ import annotations

import math
import zlib
from dataclasses import dataclass

import numpy as np
from scipy.stats import chi2

from .models import Finding

MAX_STATISTICAL_VALUES = 3_000_000


def _bounded_pixels(pixels: np.ndarray) -> np.ndarray:
    """Return a deterministic, source-wide sample to bound CPU and memory."""
    if pixels.size <= MAX_STATISTICAL_VALUES:
        return pixels
    stride = math.ceil(math.sqrt(pixels.size / MAX_STATISTICAL_VALUES))
    return pixels[::stride, ::stride]


@dataclass(slots=True)
class StatisticalResult:
    findings: list[Finding]
    score: float


def counterfactual_reembedding(pixels: np.ndarray, payload: float = 0.10, repeats: int = 5) -> dict[str, float | int]:
    """Measure detector response after controlled subsequent LSB replacement."""
    pixels = _bounded_pixels(pixels)
    if pixels.ndim == 2:
        pixels = pixels[:, :, None]
    base = analyze_pixels(pixels).score
    flat_size = pixels.size
    count = max(1, int(flat_size * payload))
    scores = []
    for seed in range(repeats):
        rng = np.random.default_rng(seed)
        indices = rng.choice(flat_size, size=count, replace=False)
        candidate = pixels.copy().reshape(-1)
        candidate[indices] = (candidate[indices] & 0xFE) | rng.integers(0, 2, size=count, dtype=np.uint8)
        scores.append(analyze_pixels(candidate.reshape(pixels.shape)).score)
    delta = float(np.mean(scores) - base)
    return {
        "payload_fraction": payload,
        "repeats": repeats,
        "base_score": round(base, 6),
        "reembedded_mean_score": round(float(np.mean(scores)), 6),
        "response_delta": round(delta, 6),
        "response_std": round(float(np.std(scores)), 6),
    }


def local_evidence_map(pixels: np.ndarray, tile_size: int = 128, limit: int = 12) -> dict:
    """Locate tiles with the strongest low-order statistical evidence."""
    if pixels.ndim == 2:
        pixels = pixels[:, :, None]
    height, width = pixels.shape[:2]
    tiles = []
    for y in range(0, height, tile_size):
        for x in range(0, width, tile_size):
            tile = pixels[y : min(y + tile_size, height), x : min(x + tile_size, width)]
            if tile.size < 1024:
                continue
            channel_scores = []
            for channel in range(tile.shape[2]):
                _, p_value = _chi_square_lsb(tile[:, :, channel])
                bit_entropy = shannon_entropy((tile[:, :, channel] & 1) * 255)
                channel_scores.append(100 * (0.6 * p_value + 0.4 * min(1, bit_entropy)))
            tiles.append(
                {
                    "x": x,
                    "y": y,
                    "width": tile.shape[1],
                    "height": tile.shape[0],
                    "score": round(max(channel_scores), 3),
                }
            )
    tiles.sort(key=lambda item: item["score"], reverse=True)
    all_scores = [item["score"] for item in tiles]
    return {
        "tile_size": tile_size,
        "tiles_analyzed": len(tiles),
        "top_tiles": tiles[:limit],
        "score_dispersion": round(float(np.std(all_scores)), 6) if all_scores else 0,
    }


def shannon_entropy(values: np.ndarray) -> float:
    counts = np.bincount(values.astype(np.uint8).ravel(), minlength=256)
    probabilities = counts[counts > 0] / counts.sum()
    return float(-(probabilities * np.log2(probabilities)).sum())


def _chi_square_lsb(values: np.ndarray) -> tuple[float, float]:
    counts = np.bincount(values.astype(np.uint8).ravel(), minlength=256).astype(float)
    pairs = counts.reshape(128, 2)
    expected = pairs.mean(axis=1)
    valid = expected > 0
    statistic = float((((pairs[valid] - expected[valid, None]) ** 2) / expected[valid, None]).sum())
    p_value = float(chi2.sf(statistic, int(valid.sum()))) if valid.any() else 0.0
    return statistic, p_value


def _runs_test(bits: np.ndarray) -> tuple[float, int]:
    bits = bits.astype(np.uint8).ravel()
    n = len(bits)
    n1 = int(bits.sum())
    n0 = n - n1
    runs = int(np.count_nonzero(bits[1:] != bits[:-1]) + 1)
    if not n0 or not n1 or n < 2:
        return 0.0, runs
    expected = 1 + (2 * n0 * n1) / n
    variance = (2 * n0 * n1 * (2 * n0 * n1 - n)) / (n * n * (n - 1))
    return float((runs - expected) / math.sqrt(max(variance, 1e-12))), runs


def _flip(values: np.ndarray, positive: bool) -> np.ndarray:
    values = values.copy()
    even = values % 2 == 0
    if positive:
        values[even] = np.minimum(values[even] + 1, 255)
        values[~even] = np.maximum(values[~even] - 1, 0)
    else:
        values[even] = np.maximum(values[even] - 1, 0)
        values[~even] = np.minimum(values[~even] + 1, 255)
    return values


def _rs(values: np.ndarray) -> dict[str, float | int]:
    flat = values.astype(np.int16).ravel()
    usable = len(flat) - len(flat) % 4
    groups = flat[:usable].reshape(-1, 4)
    if not len(groups):
        return {
            "groups": 0,
            "regular_positive": 0,
            "singular_positive": 0,
            "regular_negative": 0,
            "singular_negative": 0,
            "symmetry": 0.0,
        }
    base = np.abs(np.diff(groups, axis=1)).sum(axis=1)
    pos = np.abs(np.diff(_flip(groups, True), axis=1)).sum(axis=1)
    neg = np.abs(np.diff(_flip(groups, False), axis=1)).sum(axis=1)
    rp, sp = int((pos > base).sum()), int((pos < base).sum())
    rn, sn = int((neg > base).sum()), int((neg < base).sum())
    symmetry = 1 - (abs(rp - rn) + abs(sp - sn)) / max(2 * len(groups), 1)
    return {
        "groups": len(groups),
        "regular_positive": rp,
        "singular_positive": sp,
        "regular_negative": rn,
        "singular_negative": sn,
        "symmetry": round(float(max(0, min(1, symmetry))), 6),
    }


def analyze_pixels(pixels: np.ndarray) -> StatisticalResult:
    pixels = _bounded_pixels(pixels)
    if pixels.ndim == 2:
        pixels = pixels[:, :, None]
    if pixels.shape[2] > 3:
        pixels = pixels[:, :, :3]
    findings: list[Finding] = []
    chi_scores: list[float] = []
    entropy_scores: list[float] = []
    rs_scores: list[float] = []
    channel_names = ["R", "G", "B"] if pixels.shape[2] == 3 else ["Y"]

    for index, name in enumerate(channel_names):
        values = pixels[:, :, index].astype(np.uint8)
        statistic, p_value = _chi_square_lsb(values)
        bits = values & 1
        bit_entropy = shannon_entropy(bits * 255)
        z_score, runs = _runs_test(bits)
        rs = _rs(values)
        chi_scores.append(p_value)
        entropy_score = min(1.0, bit_entropy)
        entropy_scores.append(entropy_score)
        rs_scores.append(float(rs["symmetry"]))
        findings.extend(
            [
                Finding(
                    f"statistics.chi-square-{name.lower()}",
                    "statistics",
                    f"χ² de pares de valores · {name}",
                    "medium" if p_value > 0.95 else "info",
                    "westfeld-chi-square",
                    {"chi_square": round(statistic, 4), "p_value": round(p_value, 8)},
                    "Pares casi equiprobables son compatibles con sustitución LSB; "
                    "también pueden aparecer de forma natural.",
                    min(82, round(35 + 47 * p_value)),
                ),
                Finding(
                    f"statistics.lsb-runs-{name.lower()}",
                    "statistics",
                    f"Aleatoriedad del LSB · {name}",
                    "low" if abs(z_score) < 1.96 and bit_entropy > 0.98 else "info",
                    "lsb-entropy-runs",
                    {"entropy": round(bit_entropy, 6), "runs": runs, "z_score": round(z_score, 4)},
                    "Un LSB próximo a ruido aleatorio es compatible con carga cifrada, pero no es específico.",
                    55 if abs(z_score) < 1.96 and bit_entropy > 0.98 else 30,
                ),
                Finding(
                    f"statistics.rs-{name.lower()}",
                    "statistics",
                    f"Análisis RS · {name}",
                    "medium" if rs["symmetry"] > 0.97 else "info",
                    "regular-singular-analysis",
                    rs,
                    "La simetría entre grupos regulares/singulares mide perturbaciones compatibles con LSB.",
                    min(78, round(30 + 48 * float(rs["symmetry"]))),
                ),
            ]
        )

    plane_complexity = {}
    for plane in range(4):
        plane_bytes = np.packbits(((pixels >> plane) & 1).ravel()).tobytes()
        ratio = len(zlib.compress(plane_bytes, level=9)) / max(len(plane_bytes), 1)
        plane_complexity[f"plane_{plane}"] = round(ratio, 6)
    findings.append(
        Finding(
            "statistics.bit-plane-complexity",
            "statistics",
            "Complejidad de planos bajos",
            "low" if plane_complexity["plane_0"] > 0.98 else "info",
            "bit-plane-deflate-ratio",
            plane_complexity,
            "Planos poco compresibles se parecen a ruido; la textura y el sensor producen el mismo efecto.",
            48 if plane_complexity["plane_0"] > 0.98 else 28,
        )
    )
    score = 100 * (0.45 * max(chi_scores) + 0.25 * np.mean(rs_scores) + 0.30 * np.mean(entropy_scores))
    return StatisticalResult(findings, float(score))


def analyze_pcm(samples: np.ndarray) -> StatisticalResult:
    values = samples.astype(np.int64).ravel()
    least_bytes = (values & 0xFF).astype(np.uint8)
    statistic, p_value = _chi_square_lsb(least_bytes)
    bits = least_bytes & 1
    entropy = shannon_entropy(bits * 255)
    z_score, runs = _runs_test(bits)
    score = 100 * (0.55 * p_value + 0.45 * min(1, entropy))
    findings = [
        Finding(
            "statistics.pcm-lsb",
            "statistics",
            "LSB de muestras PCM",
            "medium" if score > 80 else "low" if score > 65 else "info",
            "pcm-chi-square-entropy-runs",
            {
                "chi_square": round(statistic, 4),
                "p_value": round(p_value, 8),
                "entropy": round(entropy, 6),
                "runs": runs,
                "z_score": round(z_score, 4),
            },
            "Distribución compatible con sustitución del bit menos significativo en audio PCM.",
            min(80, round(score)),
        )
    ]
    return StatisticalResult(findings, score)
