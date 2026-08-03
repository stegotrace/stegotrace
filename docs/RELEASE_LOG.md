# Registro de releases

## 0.2.0

Estado: preparado para publicar el 3 de agosto de 2026.

- CLI: `stegotrace models install` y `models status` gestionan Aletheia, Python, TensorFlow y `uv`.
- Modelos: perfil core ALASKA2 fijado por commit y SHA-256 para LSBM, HILL, J-UNIWARD y Steghide.
- Ciencia: la respuesta neuronal conserva procedencia y no domina el score sin calibración de fuente.
- Web: instrucciones copiables para instalar la CLI, los modelos opcionales y ejecutar un análisis.
- Verificación requerida antes de cerrar: tests, release macOS arm64/x86_64 y flujos remotos.

## 0.1.1

Estado: publicado el 3 de agosto de 2026.

- Superficies: CLI Rust nativa, motor/API Python en Railway y cliente Vite en Vercel.
- Web: `https://stegotrace.guillermozubikarai.dev` (Vercel detrás de Cloudflare).
- API: `https://stegotrace-api.guillermozubikarai.dev` (Railway EU West detrás de Cloudflare).
- CLI: GitHub Release para macOS arm64/x86_64 con checksum e instalador de una línea.
- Verificado: tests, lint, build web, health remoto, CORS, análisis y extracción con SHA-256.
- Rollback: restaurar los deployments previos; no existen datos persistentes.
