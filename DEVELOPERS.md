# Guía técnica

## Arquitectura

StegoTrace es un monorepo sin base de datos. El cliente Vite envía un archivo al API FastAPI de
Railway. El API limita el tamaño, lo guarda en un temporal privado, ejecuta el motor Python y elimina
el temporal en un bloque `finally`. La CLI Rust reproduce el contrato y las operaciones locales sin
runtime Python. Los resultados son evidencia JSON; la extracción web requiere volver a enviar el
archivo y nunca abre el artefacto.

## Estructura

- `src/stegotrace/engine.py`: orquestación, fusión conservadora y extracción.
- `src/stegotrace/structure.py`: estructura de contenedores, firmas y datos anexos.
- `src/stegotrace/statistics.py`: χ², RS, entropía, planos de bits y audio PCM.
- `src/stegotrace/scientific.py`: adaptador aislado para Aletheia.
- `src/stegotrace/api.py`: límites, CORS, rate limiting y ciclo de vida temporal.
- `cli/src/main.rs`: CLI macOS nativa, contrato JSON, lotes, benchmark y extracción.
- `web/src/`: flujo de carga, resultados, informe y extracción.

## Desarrollo local

```bash
uv sync --all-extras
uv run uvicorn stegotrace.api:app --reload --port 8000
cargo run --manifest-path cli/Cargo.toml -- --json doctor
cd web && npm ci && VITE_API_URL=http://localhost:8000 npm run dev
```

## Comprobaciones

- Motor/API: `make check`
- Web: `make web-check`
- Smoke CLI instalado: `make install-local && (cd /tmp && stegotrace --json doctor)`
- Contenedor Railway: `docker build -t stegotrace-api .`

## Configuración

- `STEGOTRACE_ALLOWED_ORIGINS`: orígenes CORS exactos separados por coma.
- `STEGOTRACE_MAX_FILE_BYTES`: máximo aceptado; 25 MiB por defecto.
- `STEGOTRACE_RATE_LIMIT`: análisis por IP y minuto; 20 por defecto.
- `STEGOTRACE_ALETHEIA_BIN`: ruta opcional al ejecutable Aletheia.
- `VITE_API_URL`: URL pública del API compilada en el cliente.

## Despliegue

- Railway usa el `Dockerfile`, `railway.toml`, `/health` y una réplica en `eu-west` (Ámsterdam).
- Vercel compila `web/` como Vite. Los cambios de `VITE_API_URL` requieren nuevo build.
- Cloudflare debe mantener proxy activo en los CNAME web/API después de la validación de dominio.
- No hay persistencia, migraciones ni rollback de datos. El rollback es volver al deployment previo.

## Diagnóstico

- `scientific.available=false`: Aletheia no está instalado o su proceso no terminó correctamente;
  el informe conserva las pruebas estructurales/estadísticas y reduce la confianza.
- CORS en navegador: confirmar origen exacto en Railway y volver a desplegar.
- 413: el archivo excede el máximo antes o durante la lectura; no aumentar el límite sin revisar
  memoria, timeout y límites de Cloudflare/Railway.
