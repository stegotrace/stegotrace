# Registro de releases

## 0.2.0

Estado: publicado y verificado el 3 de agosto de 2026.

- CLI: `stegotrace models install` y `models status` gestionan Aletheia, Python, TensorFlow y `uv`.
- Modelos: perfil core ALASKA2 fijado por commit y SHA-256 para LSBM, HILL, J-UNIWARD y Steghide.
- Ciencia: la respuesta neuronal conserva procedencia y no domina el score sin calibración de fuente.
- Web: instrucciones copiables para instalar la CLI, los modelos opcionales y ejecutar un análisis.
- Vercel: deployment `dpl_F8GQhZyVgQei3DxpbWWknEBumoAh`, dominio público detrás de Cloudflare.
- Railway: deployment `5594525a-e313-4936-b0af-304157b1f904`, `SUCCESS` en `europe-west4-drams3a`.
- Verificado: tests/lint/auditorías, builds arm64/x86_64, Lighthouse 100, CORS y extracción remota
  con coincidencia SHA-256. GitHub Release `v0.2.0` contiene ambos binarios y checksums.

## 0.1.1

Estado: publicado el 3 de agosto de 2026.

- Superficies: CLI Rust nativa, motor/API Python en Railway y cliente Vite en Vercel.
- Web: `https://stegotrace.guillermozubikarai.dev` (Vercel detrás de Cloudflare).
- API: `https://stegotrace-api.guillermozubikarai.dev` (Railway EU West detrás de Cloudflare).
- CLI: GitHub Release para macOS arm64/x86_64 con checksum e instalador de una línea.
- Verificado: tests, lint, build web, health remoto, CORS, análisis y extracción con SHA-256.
- Rollback: restaurar los deployments previos; no existen datos persistentes.
