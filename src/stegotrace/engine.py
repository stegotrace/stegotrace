from __future__ import annotations

import hashlib
import io
import json
import wave
import zipfile
import zlib
from pathlib import Path

import numpy as np
from PIL import Image, UnidentifiedImageError

from . import __version__, statistics
from .jpeg_compatibility import analyze_jpeg_compatibility
from .models import AnalysisReport, Artifact, Finding
from .scientific import analyze_with_aletheia
from .structure import SIGNATURES, _canonical_end, analyze_structure

Image.MAX_IMAGE_PIXELS = 40_000_000
MAX_DECODED_VALUES = 120_000_000
MAX_LSB_DECOMPRESSED_BYTES = 16 * 1024 * 1024
MAX_LSB_SIGNATURE_CANDIDATES = 64
LSB_SIGNATURES = tuple(item for item in SIGNATURES if item[1] in {"zip", "pdf", "png", "jpeg", "gzip"})


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


def _zsteg_stream(pixels: np.ndarray, *, bit_depth: int, channels: list[int], reverse_rows: bool = False) -> bytes:
    if pixels.ndim == 2:
        pixels = pixels[:, :, None]
    selected = pixels[::-1] if reverse_rows else pixels
    selected = selected[:, :, channels]
    shifts = np.arange(bit_depth - 1, -1, -1, dtype=np.uint8)
    bits = ((selected[:, :, :, None] >> shifts) & 1).reshape(-1)
    return np.packbits(bits, bitorder="big").tobytes()


def _artifact_id(recipe: dict) -> str:
    return hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest()[:16]


def _protocol_artifacts(pixels: np.ndarray, filename: str) -> tuple[list[Finding], list[Artifact]]:
    if pixels.ndim == 2 or pixels.shape[2] < 3:
        return [], []
    findings: list[Finding] = []
    artifacts: list[Artifact] = []

    openstego = _zsteg_stream(pixels, bit_depth=1, channels=[0, 1, 2])
    start = openstego.find(b"OPENSTEGO")
    if start >= 0 and start + 18 <= len(openstego):
        version = openstego[start + 9]
        data_size = int.from_bytes(openstego[start + 10 : start + 14], "little")
        channel_bits, name_size, compressed, encrypted = openstego[start + 14 : start + 18]
        name_end = start + 18 + name_size
        end = name_end + data_size
        name = openstego[start + 18 : name_end]
        if (
            version in {1, 2}
            and 1 <= channel_bits <= 8
            and data_size > 0
            and end <= len(openstego)
            and name
            and all(32 <= byte <= 126 for byte in name)
        ):
            decoded_name = name.decode("ascii")
            recipe = {
                "plane": 0,
                "channels": [0, 1, 2],
                "bit_order": "big",
                "start": start,
                "end": end,
            }
            artifact_id = _artifact_id(recipe)
            payload = openstego[start:end]
            findings.append(
                Finding(
                    f"extraction.openstego-{artifact_id}",
                    "extraction",
                    "Contenedor OpenStego validado",
                    "high",
                    "openstego-v1-header",
                    {
                        "version": version,
                        "data_bytes": data_size,
                        "channel_bits": channel_bits,
                        "filename": decoded_name,
                        "compressed": bool(compressed),
                        "encrypted": bool(encrypted),
                    },
                    "La cabecera, longitudes y nombre interno forman un contenedor OpenStego coherente.",
                    97,
                )
            )
            artifacts.append(
                Artifact(
                    id=artifact_id,
                    kind="openstego",
                    suggested_name=f"{Path(filename).stem}-openstego.bin",
                    size=len(payload),
                    sha256=hashlib.sha256(payload).hexdigest(),
                    description=f"Contenedor OpenStego; nombre interno: {decoded_name}.",
                    extractor="lsb",
                    parameters=recipe,
                    mime="application/octet-stream",
                )
            )

    wbstego = _zsteg_stream(pixels, bit_depth=1, channels=[2, 1, 0], reverse_rows=True)
    declared = int.from_bytes(wbstego[:3], "little") if len(wbstego) >= 6 else 0
    if 4 <= declared <= len(wbstego) - 3:
        extension = wbstego[3:6]
        message = wbstego[6 : 3 + declared]
        if (
            extension.isalnum()
            and message
            and all(byte in {9, 10, 13} or 32 <= byte <= 126 for byte in message)
            and len(message.rstrip(b"\r\n\t ")) >= 8
        ):
            recipe = {
                "bit_depth": 1,
                "channels": [2, 1, 0],
                "reverse_rows": True,
                "start": 6,
                "end": 3 + declared,
            }
            artifact_id = _artifact_id(recipe)
            decoded_extension = extension.decode("ascii").lower()
            findings.append(
                Finding(
                    f"extraction.wbstego-{artifact_id}",
                    "extraction",
                    "Carga wbStego sin cifrar validada",
                    "high",
                    "wbstego-plain-header",
                    {"declared_bytes": declared, "extension": decoded_extension},
                    "El tamaño declarado, la extensión y el contenido forman una carga wbStego coherente.",
                    96,
                )
            )
            artifacts.append(
                Artifact(
                    id=artifact_id,
                    kind="wbstego-text",
                    suggested_name=f"{Path(filename).stem}-wbstego.{decoded_extension}",
                    size=len(message),
                    sha256=hashlib.sha256(message).hexdigest(),
                    description="Texto sin cifrar recuperado de una carga wbStego.",
                    extractor="lsb-zsteg",
                    parameters=recipe,
                    mime="text/plain",
                )
            )

    for bit_depth in range(2, 5):
        stream = _zsteg_stream(pixels, bit_depth=bit_depth, channels=[0, 1, 2])
        end = stream.find(b"\x00")
        if end < 12 or stream[end : end + 4] != b"\x00" * 4:
            continue
        text = stream[:end]
        if not all(byte in {9, 10, 13} or 32 <= byte <= 126 for byte in text):
            continue
        recipe = {
            "bit_depth": bit_depth,
            "channels": [0, 1, 2],
            "reverse_rows": False,
            "start": 0,
            "end": end,
        }
        artifact_id = _artifact_id(recipe)
        findings.append(
            Finding(
                f"extraction.multibit-text-{artifact_id}",
                "extraction",
                f"Texto en {bit_depth} bits bajos por canal",
                "high",
                "multibit-lsb-text",
                {"bit_depth": bit_depth, "bytes": len(text), "channels": "RGB"},
                "Una secuencia ASCII terminada en nulos ocupa varios bits bajos de cada canal.",
                95,
            )
        )
        artifacts.append(
            Artifact(
                id=artifact_id,
                kind="lsb-text",
                suggested_name=f"{Path(filename).stem}-{bit_depth}bit-lsb.txt",
                size=len(text),
                sha256=hashlib.sha256(text).hexdigest(),
                description=f"Texto recuperado de los {bit_depth} bits bajos RGB.",
                extractor="lsb-zsteg",
                parameters=recipe,
                mime="text/plain",
            )
        )
        break
    return findings, artifacts


def _validated_payload_end(stream: bytes, offset: int, kind: str) -> int | None:
    payload = stream[offset:]
    if kind in {"png", "jpeg"}:
        end, _, _ = _canonical_end(payload, kind)
        if not end:
            return None
        try:
            with Image.open(io.BytesIO(payload[:end])) as image:
                if image.format.lower() != kind:
                    return None
                image.verify()
        except (UnidentifiedImageError, OSError, SyntaxError):
            return None
        return offset + end
    if kind == "pdf":
        end, _, _ = _canonical_end(payload, kind)
        if end and b"startxref" in payload[:end] and b"/Catalog" in payload[:end]:
            return offset + end
        return None
    if kind == "zip":
        marker = payload.find(b"PK\x05\x06", 4)
        while marker >= 0 and marker + 22 <= len(payload):
            end = marker + 22 + int.from_bytes(payload[marker + 20 : marker + 22], "little")
            if end <= len(payload):
                candidate = payload[:end]
                try:
                    with zipfile.ZipFile(io.BytesIO(candidate)) as archive:
                        if archive.infolist():
                            return offset + end
                except (OSError, ValueError, zipfile.BadZipFile):
                    pass
            marker = payload.find(b"PK\x05\x06", marker + 4)
        return None
    if kind == "gzip":
        try:
            decoder = zlib.decompressobj(wbits=31)
            decoder.decompress(payload, MAX_LSB_DECOMPRESSED_BYTES)
            if decoder.eof:
                return offset + len(payload) - len(decoder.unused_data)
        except zlib.error:
            pass
    return None


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
                for signature, kind, mime in LSB_SIGNATURES:
                    offset = stream.find(signature)
                    attempts = 0
                    while offset >= 0 and attempts < MAX_LSB_SIGNATURE_CANDIDATES:
                        attempts += 1
                        end = _validated_payload_end(stream, offset, kind)
                        if end is not None:
                            break
                        offset = stream.find(signature, offset + 1)
                    key = (plane, tuple(channels), bit_order, offset)
                    if offset < 0 or key in seen:
                        continue
                    seen.add(key)
                    payload = stream[offset:end]
                    recipe = {
                        "plane": plane,
                        "channels": channels,
                        "bit_order": bit_order,
                        "start": offset,
                        "end": end,
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
            protocol_findings, protocol_artifacts = _protocol_artifacts(pixels, display_name)
            findings.extend(protocol_findings)
            artifacts.extend(protocol_artifacts)
            methods.update(
                {
                    "westfeld-chi-square",
                    "regular-singular-analysis",
                    "lsb-entropy-runs",
                    "bit-plane-complexity",
                    "subsequent-embedding-calibration",
                    "tiled-low-order-steganalysis",
                    "openstego-v1-header",
                    "wbstego-plain-header",
                    "multibit-lsb-text",
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
    if structure.format_name in {"png", "jpeg"} and not scientific.available and score < 50:
        verdict = "Análisis no concluyente sin perfil científico"
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
    limitations = [
        "La puntuación no es una probabilidad calibrada.",
        "Un negativo no demuestra ausencia y un positivo no identifica por sí solo el algoritmo.",
        "La extracción genérica no puede descifrar cargas protegidas por clave.",
    ]
    if structure.format_name in {"png", "jpeg"} and not scientific.available:
        limitations.append(
            "No se ejecutaron detectores neuronales específicos de fuente; "
            "el resultado PNG/JPEG no puede considerarse negativo."
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
        limitations=limitations,
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
    elif artifact.extractor == "lsb":
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
    else:
        decoded = _read_image(data)
        if decoded is None:
            raise ValueError("No se puede reconstruir el flujo LSB")
        pixels, _ = decoded
        stream = _zsteg_stream(
            pixels,
            bit_depth=artifact.parameters["bit_depth"],
            channels=artifact.parameters["channels"],
            reverse_rows=artifact.parameters["reverse_rows"],
        )
        payload = stream[artifact.parameters["start"] : artifact.parameters["end"]]
    if hashlib.sha256(payload).hexdigest() != artifact.sha256:
        raise ValueError("La verificación SHA-256 del artefacto ha fallado")
    return artifact, payload
