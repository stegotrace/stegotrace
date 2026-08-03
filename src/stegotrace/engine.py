from __future__ import annotations

import hashlib
import io
import json
import wave
from pathlib import Path

import numpy as np
from PIL import Image, UnidentifiedImageError

from . import __version__, statistics
from .jpeg_compatibility import analyze_jpeg_compatibility
from .models import AnalysisReport, Artifact, Finding
from .scientific import analyze_with_aletheia
from .structure import SIGNATURES, analyze_structure

Image.MAX_IMAGE_PIXELS = 40_000_000
MAX_DECODED_VALUES = 120_000_000


def _read_image(data: bytes) -> tuple[np.ndarray, dict] | None:
    try:
        with Image.open(io.BytesIO(data)) as image:
            if image.width * image.height * min(len(image.getbands()), 3) > MAX_DECODED_VALUES:
                raise ValueError("La imagen excede el límite de píxeles decodificados")
            metadata = {
                "format": image.format,
                "mode": image.mode,
                "width": image.width,
                "height": image.height,
                "metadata_fields": sorted(str(key) for key in image.info),
                "exif_fields": len(image.getexif()),
            }
            converted = image.convert("RGB" if image.mode not in {"L", "RGB"} else image.mode)
            return np.asarray(converted, dtype=np.uint8), metadata
    except (UnidentifiedImageError, OSError):
        return None


def _lsb_stream(pixels: np.ndarray, *, plane: int, channels: list[int], bit_order: str) -> bytes:
    if pixels.ndim == 2:
        pixels = pixels[:, :, None]
    selected = pixels[:, :, channels]
    bits = ((selected >> plane) & 1).reshape(-1)
    return np.packbits(bits, bitorder=bit_order).tobytes()


def _lsb_artifacts(pixels: np.ndarray, filename: str) -> tuple[list[Finding], list[Artifact]]:
    if pixels.ndim == 2:
        pixels = pixels[:, :, None]
    available = pixels.shape[2]
    channel_sets = [list(range(min(3, available)))] + [[index] for index in range(min(3, available))]
    findings: list[Finding] = []
    artifacts: list[Artifact] = []
    seen: set[tuple[int, tuple[int, ...], str, int]] = set()
    for plane in range(2):
        for channels in channel_sets:
            for bit_order in ("big", "little"):
                stream = _lsb_stream(pixels, plane=plane, channels=channels, bit_order=bit_order)
                for signature, kind, mime in SIGNATURES:
                    offset = stream.find(signature)
                    key = (plane, tuple(channels), bit_order, offset)
                    if offset < 0 or key in seen:
                        continue
                    seen.add(key)
                    payload = stream[offset:]
                    recipe = {
                        "plane": plane,
                        "channels": channels,
                        "bit_order": bit_order,
                        "start": offset,
                        "end": len(stream),
                    }
                    artifact_id = hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest()[:16]
                    channel_label = "".join("RGB"[channel] for channel in channels) if available > 1 else "Y"
                    findings.append(
                        Finding(
                            f"extraction.lsb-{artifact_id}",
                            "extraction",
                            f"Firma {kind.upper()} en flujo LSB",
                            "high",
                            "lsb-signature-carving",
                            {**recipe, "channel_label": channel_label, "signature": signature.hex()},
                            "La reconstrucción de bits contiene una firma de archivo conocida.",
                            94,
                        )
                    )
                    artifacts.append(
                        Artifact(
                            id=artifact_id,
                            kind=f"lsb-{kind}",
                            suggested_name=f"{Path(filename).stem}-lsb.{kind}",
                            size=len(payload),
                            sha256=hashlib.sha256(payload).hexdigest(),
                            description=f"Flujo {kind.upper()} reconstruido desde el plano {plane} ({channel_label}).",
                            extractor="lsb",
                            parameters=recipe,
                            mime=mime,
                        )
                    )
    return findings, artifacts


def _wav_samples(data: bytes) -> tuple[np.ndarray, dict] | None:
    try:
        with wave.open(io.BytesIO(data), "rb") as wav:
            width = wav.getsampwidth()
            frames = wav.getnframes()
            raw = wav.readframes(frames)
            dtype = {1: np.uint8, 2: "<i2", 4: "<i4"}.get(width)
            if dtype is None:
                return None
            return np.frombuffer(raw, dtype=dtype), {
                "channels": wav.getnchannels(),
                "sample_width": width,
                "sample_rate": wav.getframerate(),
                "frames": frames,
            }
    except (wave.Error, EOFError):
        return None


def analyze_file(path: str | Path, *, filename: str | None = None) -> AnalysisReport:
    source = Path(path)
    data = source.read_bytes()
    display_name = filename or source.name
    structure = analyze_structure(data, display_name)
    findings = list(structure.findings)
    artifacts = list(structure.artifacts)
    methods = {"container-boundary", "signature-carving"}
    statistical_score = 0.0

    if structure.format_name in {"png", "gif"}:
        decoded = _read_image(data)
        if decoded:
            pixels, metadata = decoded
            findings.append(
                Finding(
                    "metadata.image",
                    "structure",
                    "Propiedades de imagen",
                    "info",
                    "pillow-container-metadata",
                    metadata,
                    "Inventario de estructura y campos; los valores privados no se registran.",
                    100,
                )
            )
            statistical = statistics.analyze_pixels(pixels)
            findings.extend(statistical.findings)
            statistical_score = statistical.score
            counterfactual = statistics.counterfactual_reembedding(pixels)
            saturated = counterfactual["response_delta"] < 2
            findings.append(
                Finding(
                    "frontier.counterfactual-reembedding",
                    "frontier",
                    "Calibración contrafactual por re-embebido",
                    "medium" if saturated else "info",
                    "subsequent-embedding-calibration",
                    counterfactual,
                    "Una respuesta saturada tras re-embebido controlado es compatible con "
                    "modificación previa, pero depende de fuente y algoritmo.",
                    66 if saturated else 48,
                )
            )
            localization = statistics.local_evidence_map(pixels)
            findings.append(
                Finding(
                    "frontier.local-evidence-map",
                    "frontier",
                    "Mapa local de evidencia",
                    "info",
                    "tiled-low-order-steganalysis",
                    localization,
                    "Localiza regiones para revisión; no segmenta de forma concluyente los bits modificados.",
                    58,
                )
            )
            lsb_findings, lsb_artifacts = _lsb_artifacts(pixels, display_name)
            findings.extend(lsb_findings)
            artifacts.extend(lsb_artifacts)
            methods.update(
                {
                    "westfeld-chi-square",
                    "regular-singular-analysis",
                    "lsb-entropy-runs",
                    "bit-plane-complexity",
                    "subsequent-embedding-calibration",
                    "tiled-low-order-steganalysis",
                }
            )
    elif structure.format_name == "jpeg":
        decoded = _read_image(data)
        if decoded:
            _, metadata = decoded
            findings.append(
                Finding(
                    "metadata.image",
                    "structure",
                    "Propiedades JPEG",
                    "info",
                    "pillow-container-metadata",
                    metadata,
                    "El LSB de píxeles descomprimidos no se usa para inferir esteganografía DCT.",
                    100,
                )
            )
        compatibility = analyze_jpeg_compatibility(source)
        findings.append(compatibility.finding)
        statistical_score = max(statistical_score, compatibility.score)
        methods.add("bounded-jpeg-antecedent-search")
    elif structure.format_name == "wav":
        decoded_audio = _wav_samples(data)
        if decoded_audio:
            samples, metadata = decoded_audio
            findings.append(
                Finding(
                    "metadata.wav",
                    "structure",
                    "Propiedades PCM",
                    "info",
                    "riff-wave-parser",
                    metadata,
                    "Parámetros del flujo PCM usados para seleccionar el análisis de muestras.",
                    100,
                )
            )
            statistical = statistics.analyze_pcm(samples)
            findings.extend(statistical.findings)
            statistical_score = statistical.score
            methods.add("pcm-chi-square-entropy-runs")

    if structure.format_name in {"png", "jpeg"}:
        scientific = analyze_with_aletheia(source)
    else:
        from .models import ScientificResult

        scientific = ScientificResult(
            False, "Aletheia", limitation="La integración neuronal configurada solo admite imágenes PNG/JPEG."
        )
    if scientific.available:
        neural_score = max(scientific.predictions.values(), default=0) * 100
        findings.append(
            Finding(
                "scientific.aletheia",
                "model",
                "Predicción del modelo científico",
                "high" if neural_score >= 70 else "medium" if neural_score >= 50 else "info",
                "aletheia-auto",
                scientific.predictions,
                "Clasificación específica por esquema; puede degradarse por cover-source mismatch.",
                min(90, round(neural_score)),
            )
        )
        methods.add("aletheia-auto")
    else:
        neural_score = 0.0

    structural_score = max(
        (
            finding.confidence
            for finding in findings
            if finding.severity == "high" and finding.category in {"structure", "extraction"}
        ),
        default=0,
    )
    score = round(max(structural_score, min(72, statistical_score * 0.72), neural_score))
    verdict = (
        "Indicios fuertes compatibles con esteganografía"
        if score >= 75
        else "Indicios que requieren revisión"
        if score >= 50
        else "Indicios débiles o inespecíficos"
        if score >= 25
        else "Sin indicios relevantes en los métodos ejecutados"
    )
    if not findings:
        findings.append(
            Finding(
                "structure.no-specific-evidence",
                "structure",
                "Sin evidencia específica",
                "info",
                "format-and-signature-scan",
                None,
                "El archivo no coincide con un analizador especializado; se inspeccionaron límites y firmas.",
                100,
            )
        )
    return AnalysisReport(
        schema_version="1.0",
        engine_version=__version__,
        filename=display_name,
        media_type=structure.media_type,
        size=len(data),
        sha256=hashlib.sha256(data).hexdigest(),
        verdict=verdict,
        score=max(0, min(100, score)),
        score_kind="heuristic_evidence_score",
        findings=findings,
        artifacts=artifacts,
        scientific=scientific,
        methods=sorted(methods),
        limitations=[
            "La puntuación no es una probabilidad calibrada.",
            "Un negativo no demuestra ausencia y un positivo no identifica por sí solo el algoritmo.",
            "La extracción genérica no puede descifrar cargas protegidas por clave.",
        ],
    )


def extract_artifact(path: str | Path, artifact_id: str, *, filename: str | None = None) -> tuple[Artifact, bytes]:
    source = Path(path)
    report = analyze_file(source, filename=filename)
    artifact = next((item for item in report.artifacts if item.id == artifact_id), None)
    if artifact is None:
        raise KeyError(f"Artefacto no encontrado: {artifact_id}")
    data = source.read_bytes()
    if artifact.extractor == "slice":
        payload = data[artifact.parameters["start"] : artifact.parameters["end"]]
    else:
        decoded = _read_image(data)
        if decoded is None:
            raise ValueError("No se puede reconstruir el flujo LSB")
        pixels, _ = decoded
        stream = _lsb_stream(
            pixels,
            plane=artifact.parameters["plane"],
            channels=artifact.parameters["channels"],
            bit_order=artifact.parameters["bit_order"],
        )
        payload = stream[artifact.parameters["start"] : artifact.parameters["end"]]
    if hashlib.sha256(payload).hexdigest() != artifact.sha256:
        raise ValueError("La verificación SHA-256 del artefacto ha fallado")
    return artifact, payload
