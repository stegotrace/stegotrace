---
name: stegotrace
description: Analiza archivos con StegoTrace, interpreta informes y extrae artefactos sin ejecutarlos.
---

# StegoTrace

Usa el binario Rust instalado para analizar archivos locales sin modificarlos.

1. Ejecuta `stegotrace doctor` y confirma `ok: true`.
2. Si se necesita inferencia neuronal, ejecuta `stegotrace models install` una vez y comprueba
   `stegotrace models status`; no presupongas que hay pesos instalados.
3. Ejecuta `stegotrace --json scan RUTA` y conserva el SHA-256 del original.
4. Explica que `score_kind=heuristic_evidence_score` y las respuestas Aletheia no son probabilidades.
5. Solo extrae si el usuario lo pide: `stegotrace extract RUTA --artifact ID --out SALIDA`.
6. Nunca abras, montes, descomprimas ni ejecutes automáticamente un artefacto recuperado.
7. Para directorios usa `stegotrace batch DIRECTORIO --out INFORMES`; no mezcles errores y éxitos.

Al informar, separa evidencia estructural verificable, estadística compatible y predicciones de
modelo. Un negativo no demuestra ausencia y una carga cifrada no puede recuperarse sin su clave.
