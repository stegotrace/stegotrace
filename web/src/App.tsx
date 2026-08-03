import { ChangeEvent, DragEvent, useRef, useState } from "react";
import type { Artifact, Report } from "./types";

const API_URL = (import.meta.env.VITE_API_URL as string | undefined)?.replace(/\/$/, "") || "http://localhost:8000";
const MAX_BYTES = 25 * 1024 * 1024;
const SAMPLE_URL = "https://github.com/kattulus1997/stegotrace/raw/refs/heads/main/samples/stegotrace-lsb-zip.png";
const INSTALL_COMMAND = "curl --proto '=https' --tlsv1.2 -LsSf https://stegotrace.guillermozubikarai.dev/install.sh | sh";

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 ** 2).toFixed(2)} MB`;
}

function valueText(value: unknown) {
  if (value === null || value === undefined) return "—";
  if (typeof value === "string" || typeof value === "number") return String(value);
  return JSON.stringify(value);
}

function Header({ reset }: { reset: () => void }) {
  return (
    <header className="header">
      <button className="wordmark" onClick={reset} aria-label="StegoTrace · Volver al inicio">
        Stego<span>Trace</span>
      </button>
      <nav aria-label="Navegación principal">
        <a href="#methods">Métodos</a>
        <a href="https://github.com/kattulus1997/stegotrace/blob/main/docs/RESEARCH.md">Investigación</a>
        <a href="#cli">CLI</a>
      </nav>
    </header>
  );
}

function CommandLine({ command, label }: { command: string; label: string }) {
  const [copyLabel, setCopyLabel] = useState("Copiar");

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      setCopyLabel("Copiado");
      window.setTimeout(() => setCopyLabel("Copiar"), 1800);
    } catch {
      setCopyLabel("Selecciona el comando");
    }
  }

  return (
    <div className="command-line">
      <span>{label}</span>
      <code>{command}</code>
      <button onClick={copy}>{copyLabel}</button>
    </div>
  );
}

function UploadView({ onReport }: { onReport: (file: File, report: Report) => void }) {
  const input = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [dragging, setDragging] = useState(false);
  const [status, setStatus] = useState<"idle" | "analyzing">("idle");
  const [error, setError] = useState("");

  function choose(candidate?: File) {
    setError("");
    if (!candidate) return;
    if (candidate.size > MAX_BYTES) {
      setFile(null);
      setError("El archivo supera el máximo de 25 MB.");
      return;
    }
    setFile(candidate);
  }

  async function analyze() {
    if (!file || status === "analyzing") return;
    setStatus("analyzing");
    setError("");
    const body = new FormData();
    body.append("file", file);
    try {
      const response = await fetch(`${API_URL}/v1/analyze`, { method: "POST", body });
      if (!response.ok) {
        const detail = (await response.json().catch(() => null)) as { detail?: string } | null;
        throw new Error(detail?.detail || `El análisis falló (${response.status}).`);
      }
      onReport(file, (await response.json()) as Report);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "No se pudo completar el análisis.");
    } finally {
      setStatus("idle");
    }
  }

  function drop(event: DragEvent<HTMLDivElement>) {
    event.preventDefault();
    setDragging(false);
    choose(event.dataTransfer.files[0]);
  }

  return (
    <main>
      <section className="upload-layout">
        <div className="upload-column">
          <h1>Encuentra datos ocultos</h1>
          <p className="lead">Busca archivos anexos, cargas LSB y anomalías estadísticas. Si encuentra un flujo recuperable, puedes descargarlo sin abrirlo.</p>
          <div
            className={`dropzone${dragging ? " is-dragging" : ""}`}
            onDragOver={(event) => { event.preventDefault(); setDragging(true); }}
            onDragLeave={() => setDragging(false)}
            onDrop={drop}
          >
            <div className="upload-mark" aria-hidden="true">↥</div>
            <p>Arrastra un archivo</p>
            <button className="text-action" onClick={() => input.current?.click()}>Seleccionar desde el Mac</button>
            <small>PNG, JPEG, WAV, PDF y otros contenedores · hasta 25 MB</small>
            <span className="privacy-line">La API borra el archivo temporal al responder</span>
            <input
              ref={input}
              type="file"
              aria-label="Seleccionar archivo para analizar"
              onChange={(event: ChangeEvent<HTMLInputElement>) => choose(event.target.files?.[0])}
              hidden
            />
          </div>
          <div className="upload-actions">
            <button className="primary" onClick={analyze} disabled={!file || status === "analyzing"}>
              {status === "analyzing" ? "Examinando…" : "Examinar archivo"}
            </button>
            <span>{file ? `${file.name} · ${formatBytes(file.size)}` : "Ningún archivo seleccionado"}</span>
          </div>
          {status === "analyzing" && <div className="progress" role="progressbar"><span /></div>}
          {error && <p className="error" role="alert">{error}</p>}
          <p className="sample-link">¿Quieres comprobarlo primero? <a href={SAMPLE_URL}>Descarga el PNG de prueba con un ZIP oculto</a>.</p>
        </div>

        <aside className="methods" id="methods">
          <h2>Qué comprueba</h2>
          <div className="method-group">
            <b>Estructura del archivo</b>
            <p>Contrasta cabeceras, finales canónicos, firmas, chunks y metadatos para localizar bytes añadidos.</p>
          </div>
          <div className="method-group">
            <b>Pruebas estadísticas</b>
            <p>Calcula χ², análisis RS, entropía, runs y planos de bits. Los métodos aplicados quedan registrados en el JSON.</p>
          </div>
          <div className="method-group">
            <b>Modelos opcionales</b>
            <p>Aletheia solo interviene cuando existen pesos compatibles. El informe identifica el modelo y avisa si la fuente no coincide.</p>
          </div>
          <div className="method-note">
            <b>Cómo leer el índice</b>
            <p>Ordena indicios heurísticos de 0 a 100. No expresa la probabilidad de que el archivo contenga esteganografía.</p>
          </div>
        </aside>
      </section>
      <section className="cli-install" id="cli">
        <div className="cli-intro">
          <span>CLI para macOS · Rust nativo</span>
          <h2>Procesa el archivo en tu Mac</h2>
          <p>La descarga es gratuita y se sirve desde este dominio. Instala el binario nativo para Apple Silicon o Intel, comprueba su SHA-256 y no requiere Rosetta, Rust ni Python.</p>
          <a href="/cli/v0.2.0/SHA256SUMS.txt">Ver binarios y checksums</a>
        </div>
        <div className="cli-commands">
          <CommandLine
            label="Instalar"
            command={INSTALL_COMMAND}
          />
          <CommandLine label="Analizar" command="stegotrace --json scan imagen.png > informe.json" />
          <CommandLine label="Añadir Aletheia" command="stegotrace models install" />
          <p>La detección estructural y estadística funciona sin Aletheia. <code>models install</code> añade 205 MiB de pesos fijados por commit en un entorno aislado y requiere unos 2,8 GiB. Si no hubo inferencia, el informe lo declara; no rellena predicciones.</p>
        </div>
      </section>
      <footer className="principles" aria-label="Condiciones del servicio">
        <div><b>Archivos efímeros</b><span>La API elimina cada temporal al responder.</span></div>
        <div><b>Resultados auditables</b><span>El JSON conserva método, valor y procedencia.</span></div>
        <div><b>Railway EU West</b><span>El procesamiento web se ejecuta en Europa.</span></div>
      </footer>
    </main>
  );
}

function ResultsView({ file, report, reset }: { file: File; report: Report; reset: () => void }) {
  const [extracting, setExtracting] = useState<string | null>(null);
  const [error, setError] = useState("");

  function downloadReport() {
    const blob = new Blob([JSON.stringify(report, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${file.name}.stegotrace.json`;
    anchor.click();
    URL.revokeObjectURL(url);
  }

  async function extract(artifact: Artifact) {
    setExtracting(artifact.id);
    setError("");
    const body = new FormData();
    body.append("artifact_id", artifact.id);
    body.append("file", file);
    try {
      const response = await fetch(`${API_URL}/v1/extract`, { method: "POST", body });
      if (!response.ok) throw new Error(`No se pudo extraer el artefacto (${response.status}).`);
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = artifact.suggested_name;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "La extracción ha fallado.");
    } finally {
      setExtracting(null);
    }
  }

  return (
    <main className="results">
      <h1>Informe forense</h1>
      <section className="summary">
        <div className="file-summary">
          <b>{report.filename}</b>
          <span>Tipo: {report.media_type}</span>
          <span>Tamaño: {formatBytes(report.size)}</span>
          <span>SHA-256: <code>{report.sha256}</code></span>
        </div>
        <div className="score-summary">
          <b>{report.verdict}</b>
          <span>Índice heurístico · no es una probabilidad</span>
          <strong><em>{report.score}</em><small>/ 100</small></strong>
          <div className="score-line"><i style={{ width: `${report.score}%` }} /></div>
        </div>
      </section>

      <section className="evidence">
        <h2>Indicios observados</h2>
        <p className="table-hint">Desliza la tabla para ver valores e interpretaciones.</p>
        <div className="table-wrap">
          <table>
            <thead><tr><th>Categoría</th><th>Indicio</th><th>Severidad</th><th>Método</th><th>Valor observado</th><th>Interpretación</th><th>Conf.</th></tr></thead>
            <tbody>
              {report.findings.map((finding) => (
                <tr key={finding.id}>
                  <td>{finding.category}</td><td>{finding.title}</td><td className={`severity ${finding.severity}`}>{finding.severity}</td>
                  <td><code>{finding.method}</code></td><td><code>{valueText(finding.value)}</code></td><td>{finding.interpretation}</td><td>{finding.confidence}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <section className="artifacts">
        <h2>Flujos recuperables</h2>
        {report.artifacts.length ? report.artifacts.map((artifact) => (
          <div className="artifact-row" key={artifact.id}>
            <div><b>{artifact.suggested_name}</b><span>{artifact.description}</span></div>
            <code>{formatBytes(artifact.size)} · {artifact.sha256.slice(0, 16)}…</code>
            <button onClick={() => extract(artifact)} disabled={extracting === artifact.id}>
              {extracting === artifact.id ? "Extrayendo…" : "Descargar sin abrir"}
            </button>
          </div>
        )) : <p className="empty">No se detectaron flujos recuperables con firma reconocible.</p>}
      </section>

      {error && <p className="error" role="alert">{error}</p>}
      <div className="result-actions">
        <button onClick={downloadReport}>Descargar informe JSON</button>
        <button onClick={reset}>Analizar otro archivo</button>
        <div><b>Métodos aplicados</b><span>{report.methods.join(" · ")}</span></div>
      </div>
      <footer className="limitation"><b>Límite del informe</b><span>{report.limitations.join(" ")}</span></footer>
    </main>
  );
}

export default function App() {
  const [result, setResult] = useState<{ file: File; report: Report } | null>(null);
  const reset = () => setResult(null);
  return (
    <div className="app-shell">
      <Header reset={reset} />
      {result ? <ResultsView {...result} reset={reset} /> : <UploadView onReport={(file, report) => setResult({ file, report })} />}
    </div>
  );
}
