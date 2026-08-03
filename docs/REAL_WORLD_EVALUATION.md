# Evaluación reproducible con muestras reales

Fecha: 3 de agosto de 2026. Esta evaluación no entrena ni calibra con las muestras: fija dos
repositorios públicos por commit, comprueba cada SHA-256 y contrasta StegoTrace con las etiquetas y
el extractor de referencia. Los resultados solo describen este corpus.

## Procedencia

- [Aletheia](https://github.com/daniellerch/aletheia/tree/1baf974ea8fcf0b51802935d9acbe59903d06845/sample_images),
  commit `1baf974ea8fcf0b51802935d9acbe59903d06845`. Sus nombres y README identifican controles y
  algoritmos; se usaron también los pesos EfficientNet-B0 publicados en ese mismo commit.
- [zsteg](https://github.com/zed-0xff/zsteg/tree/b75b578ea13ed207561a46b8620b843c0a894422/samples),
  commit `b75b578ea13ed207561a46b8620b843c0a894422`. zsteg 0.2.14 actuó como oráculo independiente
  para el protocolo y los bytes recuperados.

## Matriz de resultados

`Nativo` es la puntuación heurística sin modelos. `Modelo` es la respuesta del detector Aletheia
homónimo cuando existe; no es una probabilidad calibrada. Un caso cuenta como detectado si tiene un
artefacto estructural/protocolario validado o un veredicto explícito de señal científica.

| Archivo | Etiqueta conocida | SHA-256 | Nativo | Modelo | Resultado final |
| --- | --- | --- | ---: | ---: | --- |
| `04686.png` | control espacial | `46a0f6f87325cb9589fabc457883e67e439105ec9439fdb28918bd8b372d59a3` | 32 | 0.000103 máx. | control, sin artefactos |
| `74006.jpg` | control JPEG | `bfa53e73a21e4525033099c52c5566835d582da2a68e935e973bb61e80be7abd` | 0 | 0.123639 máx. | control, sin artefactos |
| `37831_lsbm.png` | LSB matching | `028a603fe7d3f8a9a25a18481694a86728e3ee1025506fcae010eff912a4d93d` | 32 | 0.999993 | señal específica; revisión |
| `74051_hill.png` | HILL | `09026fdc4e1ab490b49b265906f60840a5c120181cafa241fa04883544314277` | 32 | 0.931171 | señal específica; revisión |
| `27693_steganogan.png` | SteganoGAN | `8fe9868bc4afbaef2ef0e5c880855c8e5a0a63cd0170f6a9c7d14465f50c5cbf` | 32 | 0.999961 | señal específica; revisión |
| `45762_juniw.jpg` | J-UNIWARD | `a0a4ad56d1b3141e86c16d3ac2726883c714b4d833e96dfd76a0ffd24ca3c8f7` | 0 | 0.601900 | señal específica; revisión |
| `64639_outguess.jpg` | OutGuess | `0db93148a38932871bc817abfb044370c39bbdd56e55174284530db90ff7be92` | 0 | 1.000000 | señal específica; revisión |
| `08929_nsf5.jpg` | nsF5 | `367f8d8646041883c9557eb4e601700cd2c88a4c462ebc88d3f57dc2fc717e67` | 0 | 0.999333 | señal específica; revisión |
| `76538_steghide.jpg` | Steghide | `61d85fb1742b0298f80c1fe24d92b890da9496367512d3b5d44f9952d637d1af` | 0 | 0.982487 | señal específica; revisión |
| `extradata.png` | bytes tras IEND | `204d396438f4c4b0fff68ede30d06ff5105ed6c05be4a59335c34377ea771c64` | 96 | — | extrae `foobar3` (7 bytes) |
| `openstego_q2.png` | OpenStego v1 | `2481eca264fba594eab6c60d057bcad0274c88c7f76c7e977ec2d744bfde7495` | 97 | — | valida 82 bytes y nombre `flag.txt`; carga cifrada |
| `flower_rgb3.png` | texto RGB, 3 bits | `68b1ce36036e996e07b4282a4c751262cfff45c09798bfc21d155e9dbd709188` | 95 | — | extrae `SuperSecretMessage` (18 bytes) |
| `wbsteg_noenc.png` | wbStego sin cifrar | `80532497c5ee14c6b5c1c1ecdd0c81a540176643e8db7cbaaa43897dad19378c` | 96 | — | extrae `SuperSecretMessage\n` (19 bytes) |

Resultado de aceptación: **11/11 muestras esteganográficas detectadas y 2/2 controles sin artefactos
recuperables**. No equivale a sensibilidad del 100 % fuera de este conjunto.

## Diferencia entre CLI y web

La aceptación 11/11 corresponde a la CLI con `stegotrace models install`: combina el núcleo Rust y
los ocho detectores específicos. El API público de Railway ejecuta el núcleo nativo sin esos pesos:
detecta y extrae las cuatro muestras con estructura o protocolo recuperable, pero no clasifica como
negativas las otras siete. En PNG/JPEG con score inferior a 50 y sin inferencia devuelve **“Análisis
no concluyente sin perfil científico”** y la web enlaza la instalación del perfil completo. Así se
evita convertir una capacidad no ejecutada en un falso negativo.

## Cambios derivados

1. El tallado LSB del backend dejó de aceptar una firma aislada. ZIP exige EOCD y directorio
   coherentes; PNG/JPEG deben decodificar; PDF exige catálogo y `startxref`; gzip debe cerrar el
   flujo. Se retiraron PE/ELF/7z/RAR de este tallado cuando no existe un final validable.
2. Se añadieron parsers acotados para OpenStego v1, wbStego sin cifrar y texto ASCII en 2–4 bits
   bajos RGB. Cada artefacto se recorta al tramo validado y se verifica otra vez por SHA-256 al
   extraerlo.
3. El perfil opcional añadió pesos oficiales para LSBR, SteganoGAN, OutGuess y nsF5, además de LSBM,
   HILL, J-UNIWARD y Steghide. Un modelo alto ya no puede convivir con el texto “sin indicios”, pero
   su respuesta sigue separada del score heurístico.
4. El benchmark informa tres rankings: heurístico nativo, respuesta científica y envolvente de
   ambas evidencias. Antes de estos cambios el AUC heurístico era `0.5682`; después es `0.6591`. La
   respuesta científica da `0.8636`; la envolvente de ambas evidencias da `1.0000`. Esta última
   separa por completo los controles y muestras de la matriz, pero sigue siendo específica de este
   corpus y no una calibración poblacional.
5. Cuando los detectores específicos no están disponibles, PNG y JPEG pasan a un estado de
   abstención verificable en vez de comunicar ausencia. Esta política de decisión selectiva no
   aumenta artificialmente el score y conserva por separado la disponibilidad del perfil.

## Límites que permanecen

- OpenStego cifrado se identifica y conserva, pero no puede descifrarse sin clave.
- wbStego cifrado, ordenamientos arbitrarios, cargas sin cabecera y canales seleccionados por clave
  no tienen un extractor universal.
- Los modelos pueden fallar por *cover-source mismatch*. DCI actor-based requiere grupos de una
  misma fuente; no debe simularse con una sola imagen.
- La garantía “ningún archivo sin detectar” se aplica a esta matriz fijada. Para un corpus nuevo se
  repite el mismo ciclo: etiqueta independiente, controles, regresión y medición antes de publicar
  una capacidad.
