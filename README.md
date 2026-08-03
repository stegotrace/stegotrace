# StegoTrace

StegoTrace es una herramienta local y web para **detectar indicios de esteganografía y extraer
cargas recuperables sin ejecutarlas**. La CLI nativa Rust y la API Python comparten el mismo contrato:
los informes JSON incluyen SHA-256, método, evidencia, limitaciones y procedencia.

> Una puntuación es evidencia heurística, no una probabilidad ni una prueba de ausencia/presencia.
> La extracción genérica no puede descifrar cargas protegidas por una clave desconocida.

## CLI

La CLI es un binario Rust nativo para macOS y no necesita Python ni Rust en ejecución. Instalación:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/kattulus1997/stegotrace/main/install.sh | sh
```

Después:

```bash
stegotrace doctor
stegotrace scan imagen.png
stegotrace --json scan imagen.png > informe.json
stegotrace batch muestras/ --out informes/
stegotrace extract imagen.png --artifact ID --out recuperado.bin
```

La extracción nunca sobrescribe, abre, descomprime ni ejecuta el resultado. `scan` no modifica el
original. Para desarrollar la CLI:

```bash
cargo install --path cli
cargo test --manifest-path cli/Cargo.toml
cargo clippy --manifest-path cli/Cargo.toml --all-targets -- -D warnings
```

Para desarrollar la API y la web:

```bash
uv sync --all-extras
uv run pytest
make dev-api
make dev-web
```

## Web

El cliente Vite se sirve desde Vercel y envía cada archivo a la API de Railway EU West. La API usa
temporales efímeros, los elimina al finalizar, no tiene base de datos y limita tamaño y frecuencia.
El navegador conserva el archivo solo para permitir una extracción explícita en la misma sesión.

Variables:

- API: `STEGOTRACE_ALLOWED_ORIGINS`, `STEGOTRACE_MAX_FILE_BYTES`, `STEGOTRACE_RATE_LIMIT`.
- Web: `VITE_API_URL`.
- Modelo opcional: `STEGOTRACE_ALETHEIA_BIN` apunta al `aletheia.py` de una instalación verificada.

## Cobertura científica

Incluye inspección estructural y *carving*, χ² de pares, RS, runs/entropía del LSB, complejidad de
planos, análisis PCM, re-embebido contrafactual, mapas locales y una búsqueda acotada de antecedentes
JPEG QF100 basada en Levecque–Butora–Bas. Aletheia puede aportar modelos específicos; si no está
configurado, el informe lo declara y no inventa una predicción. La metodología, referencias y límites
están en [docs/RESEARCH.md](docs/RESEARCH.md).

## Formatos y seguridad

PNG, JPEG, GIF, WAV/PCM, PDF, ZIP y archivos desconocidos para estructura/firmas. Los análisis de
píxeles están limitados a 40 megapíxeles. La API acepta 25 MiB por defecto. Los archivos hostiles no
se interpretan fuera de parsers acotados y los artefactos se entregan como `application/octet-stream`
con `nosniff`.

## Licencia

Código propio bajo MIT. Los modelos y Aletheia no se redistribuyen: conservan sus licencias y deben
instalarse desde su publicación original.
