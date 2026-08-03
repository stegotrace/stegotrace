import { CommandLine } from "./App";
import SiteHeader from "./SiteHeader";
import type { Locale } from "./i18n";

const DOWNLOAD = "curl --proto '=https' --tlsv1.2 -LsSf https://stegotrace.guillermozubikarai.dev/install.sh";
const INSTALL = `${DOWNLOAD} | sh`;
const REPOSITORY = "https://github.com/stegotrace/stegotrace";
const RELEASE = `${REPOSITORY}/releases/tag/v0.3.0`;

type GuideSection = {
  id: string;
  title: string;
  intro: string;
  commands: Array<{ label: string; command: string; note: string }>;
  points?: readonly string[];
};

const guides: Record<Locale, { title: string; intro: string; start: string; contents: string; sections: GuideSection[]; footer: string; source: string; release: string }> = {
  es: {
    title: "Guía de la CLI",
    intro: "Instala StegoTrace en macOS, analiza uno o miles de archivos y extrae únicamente los flujos que superan una validación estructural. La CLI es local: scan, batch y extract no suben el archivo.",
    start: "Instalación rápida",
    contents: "En esta guía",
    sections: [
      {
        id: "instalacion", title: "Instalación y actualización", intro: "El instalador detecta Apple Silicon o Intel, descarga el binario correspondiente desde este dominio, comprueba SHA-256 y ejecuta doctor. No usa GitHub ni necesita Homebrew, Xcode, Rosetta, Rust o Python.",
        commands: [
          { label: "Instalar", command: INSTALL, note: "Instala la versión estable en /usr/local/bin. macOS puede pedir la contraseña de administrador." },
          { label: "Otra carpeta", command: `${DOWNLOAD} | STEGOTRACE_INSTALL_DIR="$HOME/.local/bin" sh`, note: "Usa una carpeta escribible si no quieres instalar en /usr/local/bin; añádela después a PATH." },
          { label: "Versión fija", command: `${DOWNLOAD} | STEGOTRACE_VERSION=v0.3.0 sh`, note: "Fija una versión reproducible en automatizaciones. Ejecutar el instalador de nuevo actualiza o reinstala." },
          { label: "Versión", command: "stegotrace --version", note: "Muestra la versión activa del binario." },
        ],
        points: ["El instalador rechaza sistemas distintos de macOS y arquitecturas desconocidas.", "Cada descarga usa HTTPS, un checksum servido por separado y una comprobación final del ejecutable."],
      },
      {
        id: "diagnostico", title: "Diagnóstico y métodos", intro: "Comprueba el binario antes de analizar evidencia y registra qué capacidades están realmente disponibles.",
        commands: [
          { label: "Estado", command: "stegotrace doctor", note: "Confirma runtime nativo, modo offline y disponibilidad de Aletheia." },
          { label: "Estado JSON", command: "stegotrace --json doctor", note: "Salida estable para inventario o soporte." },
          { label: "Métodos", command: "stegotrace methods", note: "Lista análisis estructurales, estadísticos, científicos y extractores." },
          { label: "Ayuda", command: "stegotrace --help", note: "Usa también stegotrace COMANDO --help para ver argumentos y opciones." },
        ],
      },
      {
        id: "analisis", title: "Analizar un archivo", intro: "scan solo lee el original. El resumen humano es útil para una revisión rápida; JSON conserva toda la evidencia, hashes, modelos, limitaciones y recetas de extracción.",
        commands: [
          { label: "Resumen", command: "stegotrace scan imagen.png", note: "Muestra veredicto, índice, SHA-256 y número de artefactos." },
          { label: "Informe JSON", command: "stegotrace --json scan imagen.png > imagen.stegotrace.json", note: "Recomendado para conservar o comparar resultados." },
          { label: "Ruta con espacios", command: "stegotrace --json scan \"Muestras/Foto 1.jpg\" > informe.json", note: "Pon la ruta entre comillas; el original no cambia." },
          { label: "Campos clave", command: "stegotrace --json scan imagen.png | jq '{score, verdict, artifacts, scientific}'", note: "jq es opcional y no forma parte de StegoTrace." },
        ],
        points: ["El índice heurístico no es una probabilidad.", "Una señal científica se mantiene separada porque depende del algoritmo y de la fuente de la imagen.", "Código de salida distinto de cero significa que el archivo no pudo analizarse o que el comando es inválido."],
      },
      {
        id: "modelos", title: "Modelos científicos opcionales", intro: "Los métodos nativos no necesitan modelos. El perfil gestionado instala pesos oficiales de Aletheia para LSBM, LSBR, HILL, SteganoGAN, J-UNIWARD, OutGuess, nsF5 y Steghide.",
        commands: [
          { label: "Instalar", command: "stegotrace models install", note: "Descarga 391 MiB de pesos fijados por commit y prepara un runtime aislado de unos 3 GiB." },
          { label: "Verificar", command: "stegotrace models status", note: "Comprueba versión, procedencia, tamaño, SHA-256 de cada peso y del puente de inferencia." },
          { label: "Estado JSON", command: "stegotrace --json models status > modelos.json", note: "Útil para una cadena de custodia o una máquina de laboratorio." },
        ],
        points: ["Las respuestas no son probabilidades calibradas y pueden degradarse por cover-source mismatch.", "Sin pesos, scan sigue funcionando y declara que no hubo inferencia; nunca inventa predicciones."],
      },
      {
        id: "extraccion", title: "Extraer un artefacto", intro: "Primero ejecuta scan y copia el id de artifacts. extract reconstruye exactamente la receta, verifica el SHA-256 y se niega a sobrescribir una salida existente.",
        commands: [
          { label: "Buscar ID", command: "stegotrace --json scan portadora.png | jq -r '.artifacts[] | [.id, .kind, .suggested_name] | @tsv'", note: "Revisa tipo, tamaño y descripción antes de extraer." },
          { label: "Extraer", command: "stegotrace extract portadora.png --artifact ID --out recuperado.bin", note: "Copia bytes; no abre, monta, descomprime ni ejecuta el resultado." },
          { label: "Verificar", command: "shasum -a 256 recuperado.bin", note: "Debe coincidir con artifacts[].sha256 del informe." },
        ],
        points: ["OpenStego cifrado puede identificarse y conservarse, pero no descifrarse sin clave.", "Trata cualquier archivo recuperado como evidencia hostil y analízalo en un entorno apropiado."],
      },
      {
        id: "lotes", title: "Directorios y lotes", intro: "batch escribe un informe JSON por archivo y un resumen de éxitos y errores. El directorio de salida debe estar vacío para evitar mezclar ejecuciones.",
        commands: [
          { label: "Una carpeta", command: "stegotrace batch muestras/ --out informes/", note: "Analiza solo los archivos del primer nivel." },
          { label: "Recursivo", command: "stegotrace batch muestras/ --out informes/ --recursive", note: "La forma corta de la opción es -r." },
          { label: "Resumen JSON", command: "stegotrace --json batch muestras/ --out informes/ -r > lote.json", note: "Cada elemento indica path, ok, score e informe, o el error acotado." },
        ],
      },
      {
        id: "benchmark", title: "Evaluar un corpus etiquetado", intro: "benchmark compara una carpeta de controles cover con otra de archivos stego. Informa AUC nativo, científico y de la envolvente de evidencia cuando existen modelos.",
        commands: [
          { label: "Benchmark", command: "stegotrace --json benchmark --cover corpus/cover --stego corpus/stego > benchmark.json", note: "Ambas carpetas necesitan al menos un archivo; la clasificación debe proceder de una fuente independiente." },
        ],
        points: ["AUC describe únicamente ese corpus; no mide sensibilidad universal.", "La envolvente combina rankings para evaluar cobertura, pero tampoco es una probabilidad calibrada.", "Conserva commits, hashes, técnica, controles y versiones junto al resultado."],
      },
      {
        id: "automatizacion", title: "Automatización y JSON", intro: "--json es una opción global y puede colocarse antes o después del subcomando. La salida estándar contiene datos; los errores y el progreso de descargas usan la salida de error.",
        commands: [
          { label: "Filtrar altos", command: "stegotrace --json scan archivo | jq -e '.score >= 75 or (.scientific.predictions | any(. >= 0.5))'", note: "El umbral del modelo es una regla operativa propia, no una calibración de StegoTrace." },
          { label: "Guardar versión", command: "printf '%s\n' \"$(stegotrace --version)\" > version.txt", note: "Registra también models status si usas inferencia." },
        ],
        points: ["Esquema actual del informe: 1.0.", "No ejecutes automáticamente un artefacto por el mero hecho de haberlo extraído."],
      },
      {
        id: "formatos", title: "Formatos, límites y problemas comunes", intro: "PNG, JPEG, GIF, WAV/PCM, PDF, ZIP y archivos desconocidos reciben análisis estructural; los métodos especializados se eligen por formato.",
        commands: [
          { label: "Ayuda scan", command: "stegotrace scan --help", note: "Confirma la sintaxis de la versión instalada." },
          { label: "Reinstalar", command: INSTALL, note: "Si doctor falla, reinstala y vuelve a comprobar. Los informes y originales no se tocan." },
          { label: "Desinstalar", command: "sudo rm /usr/local/bin/stegotrace", note: "Elimina solo el binario. Los modelos opcionales permanecen en Library/Application Support/StegoTrace hasta que los retires explícitamente." },
        ],
        points: ["Las imágenes decodificadas se limitan a 40 megapíxeles; la web acepta 25 MB por archivo.", "No existe un detector universal: una carga pequeña, cifrada, recomprimida o desconocida puede quedar fuera de alcance.", "extract falla si el id no pertenece al archivo o si el hash reconstruido no coincide."],
      },
    ],
    footer: "Para reproducir la evaluación pública con muestras reales, consulta REAL_WORLD_EVALUATION.md en el repositorio.",
    source: "Abrir el repositorio",
    release: "Ver la release v0.3.0",
  },
  en: {
    title: "CLI guide",
    intro: "Install StegoTrace on macOS, inspect one file or thousands, and extract only streams that pass structural validation. The CLI is local: scan, batch, and extract never upload the file.",
    start: "Quick install",
    contents: "In this guide",
    sections: [
      {
        id: "installation", title: "Install and update", intro: "The installer detects Apple Silicon or Intel, downloads the matching binary from this domain, checks SHA-256, and runs doctor. It does not use GitHub and needs no Homebrew, Xcode, Rosetta, Rust, or Python.",
        commands: [
          { label: "Install", command: INSTALL, note: "Installs the stable release in /usr/local/bin. macOS may request an administrator password." },
          { label: "Other folder", command: `${DOWNLOAD} | STEGOTRACE_INSTALL_DIR="$HOME/.local/bin" sh`, note: "Choose a writable folder to avoid /usr/local/bin, then add it to PATH." },
          { label: "Pinned release", command: `${DOWNLOAD} | STEGOTRACE_VERSION=v0.3.0 sh`, note: "Pins a reproducible release. Running the installer again updates or reinstalls it." },
          { label: "Version", command: "stegotrace --version", note: "Prints the active binary version." },
        ],
        points: ["The installer rejects non-macOS systems and unknown architectures.", "Every download uses HTTPS, a separately served checksum, and a final executable check."],
      },
      {
        id: "diagnostics", title: "Diagnostics and methods", intro: "Check the binary before examining evidence and record which capabilities are actually available.",
        commands: [
          { label: "Status", command: "stegotrace doctor", note: "Confirms the native runtime, offline mode, and Aletheia availability." },
          { label: "JSON status", command: "stegotrace --json doctor", note: "Stable output for inventory and support." },
          { label: "Methods", command: "stegotrace methods", note: "Lists structural, statistical, scientific, and extraction methods." },
          { label: "Help", command: "stegotrace --help", note: "Use stegotrace COMMAND --help for command-specific arguments and options." },
        ],
      },
      {
        id: "scan", title: "Scan one file", intro: "scan only reads the original. The human summary supports a quick review; JSON retains evidence, hashes, model responses, limitations, and extraction recipes.",
        commands: [
          { label: "Summary", command: "stegotrace scan image.png", note: "Prints the verdict, score, SHA-256, and artifact count." },
          { label: "JSON report", command: "stegotrace --json scan image.png > image.stegotrace.json", note: "Recommended for retaining or comparing results." },
          { label: "Spaces in path", command: "stegotrace --json scan \"Samples/Photo 1.jpg\" > report.json", note: "Quote the path; the original remains unchanged." },
          { label: "Key fields", command: "stegotrace --json scan image.png | jq '{score, verdict, artifacts, scientific}'", note: "jq is optional and is not part of StegoTrace." },
        ],
        points: ["The heuristic score is not a probability.", "Scientific responses stay separate because validity depends on the algorithm and image source.", "A non-zero exit status means the file could not be analyzed or the command was invalid."],
      },
      {
        id: "models", title: "Optional scientific models", intro: "Native methods require no models. The managed profile installs official Aletheia weights for LSBM, LSBR, HILL, SteganoGAN, J-UNIWARD, OutGuess, nsF5, and Steghide.",
        commands: [
          { label: "Install", command: "stegotrace models install", note: "Downloads 391 MiB of commit-pinned weights and prepares an isolated runtime of about 3 GiB." },
          { label: "Verify", command: "stegotrace models status", note: "Checks versions, provenance, sizes, every weight SHA-256, and the inference bridge." },
          { label: "JSON status", command: "stegotrace --json models status > models.json", note: "Useful for chain-of-custody records or a lab workstation." },
        ],
        points: ["Responses are not calibrated probabilities and may degrade under cover-source mismatch.", "Without weights, scan continues and states that no inference ran; it never fabricates predictions."],
      },
      {
        id: "extract", title: "Extract an artifact", intro: "Run scan first and copy an id from artifacts. extract rebuilds the exact recipe, verifies SHA-256, and refuses to overwrite an existing output.",
        commands: [
          { label: "Find the ID", command: "stegotrace --json scan carrier.png | jq -r '.artifacts[] | [.id, .kind, .suggested_name] | @tsv'", note: "Review type, size, and description before extraction." },
          { label: "Extract", command: "stegotrace extract carrier.png --artifact ID --out recovered.bin", note: "Copies bytes; it never opens, mounts, decompresses, or executes the result." },
          { label: "Verify", command: "shasum -a 256 recovered.bin", note: "The value must match artifacts[].sha256 in the report." },
        ],
        points: ["Encrypted OpenStego can be identified and preserved, but not decrypted without its key.", "Treat every recovered file as hostile evidence and inspect it in an appropriate environment."],
      },
      {
        id: "batch", title: "Directories and batches", intro: "batch writes one JSON report per file and summarizes successes and errors. The output directory must be empty so separate runs cannot be mixed.",
        commands: [
          { label: "One folder", command: "stegotrace batch samples/ --out reports/", note: "Scans only files at the first level." },
          { label: "Recursive", command: "stegotrace batch samples/ --out reports/ --recursive", note: "The short option is -r." },
          { label: "JSON summary", command: "stegotrace --json batch samples/ --out reports/ -r > batch.json", note: "Each item contains path, ok, score and report, or a bounded error." },
        ],
      },
      {
        id: "benchmark", title: "Evaluate a labeled corpus", intro: "benchmark compares a cover-control folder with a stego folder. It reports native, scientific, and evidence-envelope AUC when models are installed.",
        commands: [
          { label: "Benchmark", command: "stegotrace --json benchmark --cover corpus/cover --stego corpus/stego > benchmark.json", note: "Both folders need a file, and labels must come from an independent source." },
        ],
        points: ["AUC only describes that corpus; it is not universal sensitivity.", "The envelope combines rankings to measure coverage and is still not a calibrated probability.", "Retain commits, hashes, techniques, controls, and versions with the result."],
      },
      {
        id: "automation", title: "Automation and JSON", intro: "--json is global and may appear before or after the subcommand. Standard output contains data; download progress and errors use standard error.",
        commands: [
          { label: "Filter signals", command: "stegotrace --json scan file | jq -e '.score >= 75 or (.scientific.predictions | any(. >= 0.5))'", note: "A model threshold is your operational rule, not StegoTrace calibration." },
          { label: "Record version", command: "printf '%s\n' \"$(stegotrace --version)\" > version.txt", note: "Record models status as well whenever inference is used." },
        ],
        points: ["Current report schema: 1.0.", "Never execute an artifact automatically merely because extraction succeeded."],
      },
      {
        id: "formats", title: "Formats, limits, and common problems", intro: "PNG, JPEG, GIF, WAV/PCM, PDF, ZIP, and unknown files receive structural checks; specialized methods are selected by format.",
        commands: [
          { label: "Scan help", command: "stegotrace scan --help", note: "Confirms syntax for the installed release." },
          { label: "Reinstall", command: INSTALL, note: "If doctor fails, reinstall and check again. Reports and original files are untouched." },
          { label: "Uninstall", command: "sudo rm /usr/local/bin/stegotrace", note: "Removes only the binary. Optional models remain in Library/Application Support/StegoTrace until explicitly removed." },
        ],
        points: ["Decoded images are capped at 40 megapixels; the web accepts 25 MB per file.", "No universal detector exists: tiny, encrypted, recompressed, or unknown payloads may remain out of reach.", "extract fails if the id does not belong to the file or the rebuilt hash differs."],
      },
    ],
    footer: "See REAL_WORLD_EVALUATION.md in the repository to reproduce the public real-sample evaluation.",
    source: "Open the repository",
    release: "View release v0.3.0",
  },
};

export default function Guide({ locale }: { locale: Locale }) {
  const guide = guides[locale];
  return (
    <div className="app-shell">
      <SiteHeader locale={locale} page="cli" />
      <main className="guide">
        <header className="guide-hero">
          <span>StegoTrace / CLI</span>
          <h1>{guide.title}</h1>
          <p>{guide.intro}</p>
          <CommandLine locale={locale} label={guide.start} command={INSTALL} />
        </header>
        <div className="guide-layout">
          <aside className="guide-index">
            <b>{guide.contents}</b>
            <nav>{guide.sections.map((section) => <a href={`#${section.id}`} key={section.id}>{section.title}</a>)}</nav>
          </aside>
          <article className="guide-body">
            {guide.sections.map((section) => (
              <section id={section.id} key={section.id}>
                <h2>{section.title}</h2>
                <p className="section-intro">{section.intro}</p>
                <div className="guide-commands">
                  {section.commands.map((item) => <div className="guide-command" key={item.command}><CommandLine locale={locale} label={item.label} command={item.command} /><p>{item.note}</p></div>)}
                </div>
                {section.points && <ul>{section.points.map((point) => <li key={point}>{point}</li>)}</ul>}
              </section>
            ))}
            <footer><p>{guide.footer}</p><a href={REPOSITORY}>{guide.source}</a><a href={RELEASE}>{guide.release}</a></footer>
          </article>
        </div>
      </main>
    </div>
  );
}
