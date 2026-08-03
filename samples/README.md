# Muestras de prueba

`stegotrace-lsb-zip.png` es una imagen PNG inocua de 64×32 píxeles. Su canal rojo contiene, en el
plano LSB y orden de bits big-endian, un ZIP válido de 256 bytes con un único archivo de texto.

- SHA-256 de la imagen: `632a2de6dd314ff1c9e7a2fbb6b6eb31b8e2822e97e9de03df7cd6153c043935`
- SHA-256 del ZIP oculto: `b8159a7907ff2271520197e3df2cbeaf050c0358b193af2c93669e56c5ee7e8b`

Uso:

```bash
stegotrace --json scan samples/stegotrace-lsb-zip.png
```

La muestra sirve para comprobar detección y extracción. No contiene ejecutables, claves ni datos
privados.
