from __future__ import annotations

import io

import numpy as np
from fastapi.testclient import TestClient
from PIL import Image

from stegotrace.api import app


def test_health_and_upload_lifecycle() -> None:
    buffer = io.BytesIO()
    Image.fromarray(np.zeros((16, 16, 3), dtype=np.uint8), "RGB").save(buffer, format="PNG")
    client = TestClient(app)

    assert client.get("/health").json()["ok"] is True
    response = client.post("/v1/analyze", files={"file": ("clean.png", buffer.getvalue(), "image/png")})

    assert response.status_code == 200
    assert response.json()["filename"] == "clean.png"
    assert response.json()["media_type"] == "image/png"
