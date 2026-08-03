# Registro de releases

## Distribución CLI · revisión del 3 de agosto de 2026

Estado: publicado y verificado.

- Instalación gratuita desde `stegotrace.guillermozubikarai.dev`, sin consultas ni descargas desde GitHub.
- Arquitecturas: detección del hardware físico; arm64 para Apple Silicon y x86_64 para Mac Intel,
  incluso cuando Terminal se ejecuta traducido. No requiere Rosetta, Rust ni Python.
- Mac limpio: instalación en `/usr/local/bin`, ya presente en el `PATH` estándar de macOS; no
  requiere Homebrew ni Xcode y ejecuta `doctor` antes de declarar éxito.
- Integridad: binarios versionados con SHA-256 individual y manifiesto público; ambos flujos de
  selección y sus checksums están cubiertos por `make web-check`.
- Favicon: SVG, ICO, PNG y Apple Touch comparten el mismo símbolo y URLs versionadas.
- Vercel: producción en `stegotrace.guillermozubikarai.dev`, servida por Cloudflare.
- Verificado: instalación limpia arm64 desde el comando público con `doctor.ok=true`, selección
  simulada Intel y Apple Silicon traducido, artefactos remotos y favicons con hash local coincidente.

## Web · revisión del 3 de agosto de 2026

Estado: publicado y verificado.

- Interfaz: copy forense específico, jerarquía tipográfica simplificada y muestra LSB descargable.
- Móvil: eliminado el cuadrado decorativo ambiguo del aviso sobre archivos temporales.
- Resultados: el índice se identifica expresamente como heurístico y la extracción como descarga sin apertura.
- Vercel: deployment `dpl_5dEkmTssh24esYQ9eM5czga1sz7u`, servido por el dominio público detrás de Cloudflare.
- Verificado: `make web-check`, escaneo `kill-ai-slop` sin señales, escritorio y móvil en Chrome,
  análisis público de la muestra y extracción remota con coincidencia SHA-256.

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
