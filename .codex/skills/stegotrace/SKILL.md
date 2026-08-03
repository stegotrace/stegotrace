---
name: stegotrace
description: Analiza archivos con StegoTrace, interpreta informes y extrae artefactos sin ejecutarlos.
---

# StegoTrace

Usa el binario Rust instalado para analizar archivos locales sin modificarlos.

1. Ejecuta `stegotrace doctor` y confirma `ok: True`.
2. Ejecuta `stegotrace --json scan RUTA` y conserva el SHA-256 del original.
3. Explica que `score_kind=heuristic_evidence_score` no es una probabilidad.
4. Solo extrae si el usuario lo pide: `stegotrace extract RUTA --artifact ID --out SALIDA`.
5. Nunca abras, montes, descomprimas ni ejecutes automáticamente un artefacto recuperado.
6. Para directorios usa `stegotrace batch DIRECTORIO --out INFORMES`; no mezcles errores y éxitos.

Al informar, separa evidencia estructural verificable, estadística compatible y predicciones de
modelo. Un negativo no demuestra ausencia y una carga cifrada no puede recuperarse sin su clave.
