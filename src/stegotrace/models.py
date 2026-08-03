from __future__ import annotations

from dataclasses import asdict, dataclass, field
from typing import Any, Literal

Severity = Literal["info", "low", "medium", "high"]


@dataclass(slots=True)
class Finding:
    id: str
    category: str
    title: str
    severity: Severity
    method: str
    value: Any
    interpretation: str
    confidence: int


@dataclass(slots=True)
class Artifact:
    id: str
    kind: str
    suggested_name: str
    size: int
    sha256: str
    description: str
    extractor: Literal["slice", "lsb"]
    parameters: dict[str, Any]
    mime: str = "application/octet-stream"


@dataclass(slots=True)
class ScientificResult:
    available: bool
    provider: str
    methods: list[str] = field(default_factory=list)
    predictions: dict[str, float] = field(default_factory=dict)
    limitation: str | None = None


@dataclass(slots=True)
class AnalysisReport:
    schema_version: str
    engine_version: str
    filename: str
    media_type: str
    size: int
    sha256: str
    verdict: str
    score: int
    score_kind: str
    findings: list[Finding]
    artifacts: list[Artifact]
    scientific: ScientificResult
    methods: list[str]
    limitations: list[str]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)
