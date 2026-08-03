from __future__ import annotations

import os
import re
import shutil
import subprocess
from pathlib import Path

from .models import ScientificResult

METHOD_RE = re.compile(
    r"(?P<method>Outguess|Steghide|nsF5|J-UNIWARD|LSBM|LSBR|HILL|UNIWARD)\s+\[?(?P<score>[01](?:\.\d+)?)\]?"
)


def analyze_with_aletheia(path: Path) -> ScientificResult:
    executable = os.getenv("STEGOTRACE_ALETHEIA_BIN", "").strip() or shutil.which("aletheia.py")
    if not executable:
        return ScientificResult(
            available=False,
            provider="Aletheia",
            limitation="Aletheia no está configurado; no se ejecutó inferencia neuronal.",
        )
    try:
        completed = subprocess.run(
            [executable, "auto", str(path)],
            check=False,
            capture_output=True,
            text=True,
            timeout=180,
            env={**os.environ, "TF_CPP_MIN_LOG_LEVEL": "3"},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return ScientificResult(False, "Aletheia", limitation=f"Inferencia no disponible: {type(error).__name__}.")
    if completed.returncode:
        return ScientificResult(
            False, "Aletheia", limitation="Aletheia terminó con error; salida omitida por seguridad."
        )
    predictions = {match.group("method"): float(match.group("score")) for match in METHOD_RE.finditer(completed.stdout)}
    return ScientificResult(
        available=bool(predictions),
        provider="Aletheia",
        methods=sorted(predictions),
        predictions=predictions,
        limitation=None if predictions else "La salida de Aletheia no contenía predicciones interpretables.",
    )
