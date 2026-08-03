# Base científica y alcance

StegoTrace combina evidencia complementaria porque ningún detector universal es fiable para todos
los algoritmos, cargas, fuentes y formatos. La salida es una puntuación orientativa, no una
probabilidad calibrada ni una prueba de contenido ilícito.

## Métodos implementados

- Inspección estructural: límites canónicos de PNG, JPEG, GIF, PDF y RIFF/WAV; firmas embebidas,
  segmentos anexos, chunks de texto y metadatos.
- Esteganálisis clásico: ataque χ² sobre pares de valores, estadística RS, entropía y autocorrelación
  del LSB, complejidad por plano y análisis de LSB de muestras PCM.
- Calibración operativa: cada hallazgo conserva método, valor observado, interpretación y nivel de
  confianza. La fusión penaliza indicadores estadísticos aislados.
- Frontera sin pesos: búsqueda acotada de antecedentes JPEG QF100 basada en incompatibilidad de
  bloques, calibración contrafactual mediante re-embebido posterior y mapas locales de evidencia.
  El modo online informa timeouts sin convertirlos en incompatibilidades; el LRT completo requiere
  probabilidades específicas de la tubería de compresión.
- Modelo científico: adaptador de Aletheia con timeout y fallo seguro. Sus redes entrenadas (incluidos
  EfficientNet/SRNet y DCI en su distribución) deben interpretarse en el contexto de su dominio de
  entrenamiento y del *cover-source mismatch*.
- Extracción: datos anexos y flujos LSB con firma reconocible. Los artefactos se copian como bytes;
  nunca se montan, descomprimen ni ejecutan.

## Frontera incorporada y límites

- SRNet introdujo una red residual específica para señales esteganográficas débiles, no un ResNet
  de visión genérico: https://doi.org/10.1109/TIFS.2018.2871749
- UCNet preserva canales RGB/YCbCr y usa 62 filtros residuales para análisis universal espacial/JPEG:
  https://arxiv.org/abs/2111.12231
- DCI usa re-embebido y clasificadores A/B para detectar inconsistencias y tratar el desajuste de
  fuente; la variante actor-based requiere varias imágenes de la misma fuente:
  https://arxiv.org/abs/2501.04362
- Aletheia es el motor de referencia integrable, publicado y con modelos reproducibles:
  https://doi.org/10.21105/joss.05982 y https://github.com/daniellerch/aletheia
- En JPEG de carga extremadamente baja, los tests de bloques incompatibles pueden superar redes
  profundas en el escenario estudiado; no están generalizados a cualquier codificador:
  https://arxiv.org/abs/2402.13660
- StegExpose documenta la fusión de χ², RS, sample pairs y primary sets para LSB, útil como baseline
  pero insuficiente contra esquemas adaptativos modernos: https://arxiv.org/abs/1410.6656

## Lo que no puede prometerse

Detectar no implica identificar el algoritmo. Identificar no implica recuperar el mensaje. Los
esquemas cifrados o con clave no permiten extracción genérica, y recomprimir/redimensionar puede
destruir la señal. Un resultado negativo tampoco demuestra ausencia: una carga pequeña, una fuente
distinta o un método desconocido pueden quedar bajo el umbral.
