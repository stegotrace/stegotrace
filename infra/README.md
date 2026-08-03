# Cloudflare

`cloudflare-language-router.mjs` se ejecuta delante del origen Vercel en la ruta proxied
`stegotrace.guillermozubikarai.dev/*`. Solo la entrada `/` redirige: elige `/es/` o `/en/` a partir
de `Accept-Language`, respeta prioridades `q` y usa español como fallback. El resto de solicitudes se
transmite al origen sin cambiar ruta, cuerpo ni cabeceras.

Las URLs con idioma son estables y el selector enlaza directamente a su equivalente, por lo que una
elección manual no vuelve a pasar por la detección automática. `/install.sh`, `/cli/*`, assets y las
rutas localizadas no reciben redirecciones.

Despliegue reproducible desde `infra/`:

```bash
npx wrangler deploy --config wrangler.jsonc
```

Comprobaciones públicas:

```bash
curl -I -H 'Accept-Language: en-US,en;q=0.9' https://stegotrace.guillermozubikarai.dev/
curl -I -H 'Accept-Language: es-ES,es;q=0.9' https://stegotrace.guillermozubikarai.dev/
curl -I https://stegotrace.guillermozubikarai.dev/install.sh
```
