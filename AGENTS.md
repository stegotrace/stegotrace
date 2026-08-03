# Instrucciones para agentes

## Prioridad documental

- `DEVELOPERS.md` es la referencia técnica del monorepo; `README.md` es la entrada pública.
- `docs/RESEARCH.md` registra el alcance científico y sus límites.
- Actualiza `docs/RELEASE_LOG.md` cuando cambie un despliegue o artefacto publicado.

## Superficies

- `src/stegotrace/`: motor y API Python para Railway.
- `cli/`: CLI macOS nativa en Rust.
- `web/`: cliente React/Vite desplegado en Vercel.
- `tests/`: contratos observables del motor y API.
- Los archivos bajo `assets/design/` son especificaciones visuales, no recursos de runtime.

## Reglas operativas

- No subas archivos analizados a terceros ni los conserves tras responder.
- No descomprimas, abras ni ejecutes automáticamente artefactos recuperados.
- No presentes una puntuación heurística como probabilidad calibrada.
- Si cambia un detector, añade o ajusta una prueba con cover y stego sintéticos.
- Verifica con `make check`; para cambios web añade `make web-check`.
- No edites `uv.lock` ni `web/package-lock.json` a mano: regénéralos con sus gestores.
- No imprimas secretos ni contenido recuperado potencialmente sensible en logs.
