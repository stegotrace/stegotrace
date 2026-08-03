# StegoTrace

StegoTrace es una herramienta local y web para **detectar indicios de esteganografía y extraer
cargas recuperables sin ejecutarlas**. La CLI nativa Rust y la API Python comparten el mismo contrato:
los informes JSON incluyen SHA-256, método, evidencia, limitaciones y procedencia.

> Una puntuación es evidencia heurística, no una probabilidad ni una prueba de ausencia/presencia.
> La extracción genérica no puede descifrar cargas protegidas por una clave desconocida.

Web: <https://stegotrace.guillermozubikarai.dev> · API: <https://stegotrace-api.guillermozubikarai.dev/docs>

## CLI

La CLI es un binario Rust nativo para macOS y no necesita Python ni Rust en ejecución. Instalación:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/kattulus1997/stegotrace/main/install.sh | sh
```

Después:

```bash
stegotrace doctor
stegotrace models install
stegotrace models status
stegotrace scan imagen.png
stegotrace --json scan imagen.png > informe.json
stegotrace batch muestras/ --out informes/
stegotrace extract imagen.png --artifact ID --out recuperado.bin
```

La extracción nunca sobrescribe, abre, descomprime ni ejecuta el resultado. `scan` no modifica el
original. `models install` añade de forma opcional cuatro detectores EfficientNet-B0 de Aletheia:
LSBM y HILL para imágenes espaciales, y J-UNIWARD y Steghide para JPEG. Descarga 205 MiB de pesos
desde el commit oficial fijado, comprueba cada SHA-256 y crea un entorno Python/TensorFlow aislado
en `~/Library/Application Support/StegoTrace` (aproximadamente 2,8 GiB en total). El comando instala
también una copia gestionada y verificada de `uv` si no existe en el Mac.

Las respuestas de red son específicas de ALASKA2, no probabilidades calibradas. Se conservan como
evidencia separada y no elevan por sí solas la puntuación global: antes haría falta una calibración
representativa de la fuente. El informe conserva la procedencia y advierte sobre *cover-source
mismatch*. Sin esos pesos, `scan` continúa con los métodos nativos y declara que no hubo inferencia;
no fabrica resultados.

Para desarrollar la CLI:

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
- Modelo opcional local: `stegotrace models install`. La variable `STEGOTRACE_ALETHEIA_BIN` se
  conserva solo como adaptador compatible para una instalación externa.

## Cobertura científica

Incluye inspección estructural y *carving*, χ² de pares, RS, runs/entropía del LSB, complejidad de
planos, análisis PCM, re-embebido contrafactual, mapas locales y una búsqueda acotada de antecedentes
JPEG QF100 basada en Levecque–Butora–Bas. Aletheia puede aportar modelos específicos; si no está
configurado, el informe lo declara y no inventa una predicción. La metodología, referencias y límites
están en [docs/RESEARCH.md](docs/RESEARCH.md).

La CLI Rust ejecuta estructura, firmas, χ², RS, entropía/runs, re-embebido, mapa local y extracción
LSB. La búsqueda de antecedentes JPEG QF100 requiere coeficientes DCT y se ejecuta actualmente en la
API; el adaptador Aletheia es opcional en ambas superficies.

## Formatos y seguridad

PNG, JPEG, GIF, WAV/PCM, PDF, ZIP y archivos desconocidos para estructura/firmas. Los análisis de
píxeles están limitados a 40 megapíxeles. La API acepta 25 MiB por defecto. Los archivos hostiles no
se interpretan fuera de parsers acotados y los artefactos se entregan como `application/octet-stream`
con `nosniff`.

## Licencia

Código propio bajo MIT. Los modelos y Aletheia no se redistribuyen: conservan sus licencias y deben
instalarse desde su publicación original. El puente de inferencia conserva el aviso MIT de Aletheia;
consulte [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
