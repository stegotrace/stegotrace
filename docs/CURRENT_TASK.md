# Tareas activas de este hilo

## 1. Probar StegoTrace con muestras reales

- [x] Localizar fuentes primarias con muestras etiquetadas.
- [x] Descargar un corpus fijado por commit y registrar sus SHA-256.
- [x] Ejecutar el núcleo nativo y el perfil Aletheia sobre técnicas espaciales, JPEG/DCT y datos anexados.
- [x] Contrastar también la API pública cuando el formato y el límite de tamaño lo permitan.
- [x] Documentar aciertos, falsos negativos, controles y límites reproducibles.

Terminado cuando cada muestra tenga procedencia, técnica conocida, resultado CLI/API y una conclusión
honesta sobre detección o extracción.

## 2. Publicar una guía CLI completa

- [x] Añadir en la portada un botón que lleve a una subpágina pública de documentación.
- [x] Cubrir instalación, diagnóstico, métodos, modelos, análisis, lotes, extracción, benchmark,
  salida JSON, automatización, errores y seguridad.
- [x] Incluir todos los comandos y opciones reales, con ejemplos copiables por caso de uso.
- [x] Verificar navegación, accesibilidad y composición en escritorio y móvil.

Terminado cuando la guía pública permita usar toda la CLI sin consultar el repositorio.

## 3. Publicar la web en español e inglés

- [x] Crear contenido humano completo para español e inglés; no usar traducción automática.
- [x] Usar rutas canónicas separadas por idioma, incluida la guía CLI.
- [x] Añadir selector manual de idioma y conservar la elección explícita.
- [x] Configurar Cloudflare para redirigir la entrada sin idioma según `Accept-Language`.
- [x] Evitar bucles, conservar archivos y API fuera de la redirección y permitir compartir URLs estables.
- [x] Añadir `lang`, URLs canónicas, `hreflang`, Open Graph y Twitter para cada ruta pública principal.
- [x] Verificar en producción ambos idiomas, el fallback y el paso real por Cloudflare.

Terminado cuando una visita nueva llegue al idioma compatible de su navegador y cualquier persona pueda
cambiarlo manualmente sin que Cloudflare deshaga su elección.

## 4. Mejorar el detector con los fallos demostrados por el corpus

- [x] Convertir cada falso positivo o falso negativo reproducible en una prueba de regresión.
- [x] Reforzar la validación de artefactos LSB para no presentar firmas aleatorias como archivos recuperables.
- [x] Evitar un veredicto de ausencia cuando un modelo verificado produzca una señal alta; conservarla como
  evidencia específica de fuente, no como probabilidad calibrada.
- [x] Ampliar la cobertura con métodos científicos publicados que el perfil actual omite.
- [x] Diseñar métodos experimentales propios solo cuando exista una hipótesis comprobable, controles y una
  mejora reproducible; etiquetarlos como experimentales hasta contar con validación suficiente.
- [x] Repetir el corpus nativo, científico y web después de cada corrección estable.
- [x] Documentar qué técnicas siguen fuera de alcance y por qué.

Terminado cuando las mejoras reduzcan los errores observados sin degradar los controles y queden cubiertas
por pruebas automatizadas proporcionales.
