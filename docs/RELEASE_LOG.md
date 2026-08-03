# Registro de releases

## Reubicación de GitHub · 3 de agosto de 2026

- Repositorio público: `https://github.com/stegotrace/stegotrace`.
- Propietario: organización `stegotrace` con plan GitHub Free (`0 USD`).
- Conservados: historial, releases, Actions, secretos, webhooks y redirecciones de GitHub.
- Actualizados: remoto Git, metadatos de la CLI y enlaces públicos de la web.
- Web: navegación y guía enlazan explícitamente el código, la investigación y la release en la
  organización neutral.
- Vercel: deployment `dpl_Gjk8TyH4j73zRtGP5p96dpRNGicM`, servido por Cloudflare.

## 0.3.0

Estado: publicado y verificado el 3 de agosto de 2026.

- Corpus real: 11/11 muestras esteganográficas detectadas con el perfil CLI completo y 2/2
  controles sin artefactos recuperables; matriz, SHA-256 y límites en `REAL_WORLD_EVALUATION.md`.
- Detección: nuevos perfiles LSBR, SteganoGAN, OutGuess y nsF5; extracción acotada de OpenStego
  v1, wbStego sin cifrar y texto RGB de 2–4 bits; validación reforzada de contenedores tallados.
- Decisión: PNG/JPEG sin modelos pasa a “no concluyente” en vez de comunicar ausencia; los modelos
  mantienen sus respuestas específicas separadas del índice heurístico.
- Web: español e inglés humanos, guía CLI completa, selector manual y redirección inicial por idioma
  con Cloudflare Worker `d270e049-fb5d-407e-9a2e-77b57eba1266`.
- Distribución: instalador gratuito desde el dominio propio y binarios nativos macOS arm64/x86_64,
  sin Homebrew, Xcode, Rosetta, Rust, Python ni descargas desde GitHub.
- Vercel: deployment `dpl_HqNPLQpDNgCfQFPeqf96sDAcmita`, servido por Cloudflare.
- Railway: deployment `72c0942c-83e8-4235-b8ba-a3084d1e34d0` en `europe-west4-drams3a`.
- Verificado: tests/lint/Clippy, builds de ambas arquitecturas, checksums, benchmark reproducible,
  rutas bilingües, proxy Cloudflare, instalador público, muestra pública y API de producción.

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
