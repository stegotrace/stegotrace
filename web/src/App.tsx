import { ChangeEvent, DragEvent, useRef, useState } from "react";
import type { Artifact, Report } from "./types";

const API_URL = (import.meta.env.VITE_API_URL as string | undefined)?.replace(/\/$/, "") || "http://localhost:8000";
const MAX_BYTES = 25 * 1024 * 1024;

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
          <h1>Rastrea datos ocultos</h1>
          <p className="lead">Inspección estructural, estadística y científica con extracción verificable.</p>
          <div
            className={`dropzone${dragging ? " is-dragging" : ""}`}
            onDragOver={(event) => { event.preventDefault(); setDragging(true); }}
            onDragLeave={() => setDragging(false)}
            onDrop={drop}
          >
            <div className="upload-mark" aria-hidden="true">↥</div>
            <p>Suelta un archivo aquí</p>
            <button className="text-action" onClick={() => input.current?.click()}>o selecciónalo</button>
            <small>PNG, JPEG, WAV, PDF y contenedores · máximo 25 MB</small>
            <span className="privacy-line">El archivo se elimina al terminar el análisis</span>
            <input
              ref={input}
              type="file"
              onChange={(event: ChangeEvent<HTMLInputElement>) => choose(event.target.files?.[0])}
              hidden
            />
          </div>
          <div className="upload-actions">
            <button className="primary" onClick={analyze} disabled={!file || status === "analyzing"}>
              {status === "analyzing" ? "Analizando…" : "Analizar archivo"}
            </button>
            <span>{file ? `${file.name} · ${formatBytes(file.size)}` : "Preparado para analizar"}</span>
          </div>
          {status === "analyzing" && <div className="progress" role="progressbar"><span /></div>}
          {error && <p className="error" role="alert">{error}</p>}
        </div>

        <aside className="methods" id="methods">
          <h2>Métodos</h2>
          <div className="method-group">
            <b>Estructura</b>
            <ul><li>Cabeceras y finales canónicos</li><li>Firmas y contenedores anexos</li><li>Chunks y metadatos</li></ul>
          </div>
          <div className="method-group">
            <b>Estadística</b>
            <ul><li>χ² de pares de valores</li><li>Grupos regulares y singulares</li><li>Entropía, runs y planos de bits</li></ul>
          </div>
          <div className="method-group">
            <b>Modelo científico</b>
            <ul><li>Aletheia / redes específicas</li><li>Confianza y procedencia</li><li>Advertencia de source mismatch</li></ul>
          </div>
          <div className="confidence-key">
            <span>Escala orientativa</span>
            <div><i /> <i /> <i /> <i /> <i /></div>
            <small>Baja</small><small>Media</small><small>Alta</small>
          </div>
        </aside>
      </section>
      <section className="cli-install" id="cli">
        <div className="cli-intro">
          <span>CLI local · macOS</span>
          <h2>Analiza sin subir el archivo</h2>
          <p>Binario nativo Rust para Apple Silicon e Intel. La instalación verifica la firma SHA-256 de la release antes de activar <code>stegotrace</code>.</p>
          <a href="https://github.com/kattulus1997/stegotrace/releases/latest">Ver release y checksums</a>
        </div>
        <div className="cli-commands">
          <CommandLine
            label="1 · Instala la CLI"
            command="curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/kattulus1997/stegotrace/main/install.sh | sh"
          />
          <CommandLine label="2 · Añade los modelos científicos" command="stegotrace models install" />
          <CommandLine label="3 · Analiza" command="stegotrace --json scan imagen.png > informe.json" />
          <p><code>models install</code> es opcional: descarga 205 MiB de pesos Aletheia fijados por commit y crea un entorno aislado; requiere unos 2,8 GiB en total. Sin pesos, StegoTrace declara que no hubo inferencia; nunca inventa una predicción.</p>
        </div>
      </section>
      <footer className="principles">
        <div><b>Privacidad por diseño</b><span>No almacenamos archivos ni resultados.</span></div>
        <div><b>Incertidumbre científica</b><span>La esteganálisis no ofrece certezas absolutas.</span></div>
        <div><b>Procesamiento europeo</b><span>El motor se ejecuta en Railway EU West.</span></div>
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
      <h1>Resultado del análisis</h1>
      <section className="summary">
        <div className="file-summary">
          <b>{report.filename}</b>
          <span>Tipo: {report.media_type}</span>
          <span>Tamaño: {formatBytes(report.size)}</span>
          <span>SHA-256: <code>{report.sha256}</code></span>
        </div>
        <div className="score-summary">
          <b>{report.verdict}</b>
          <span>Confianza orientativa</span>
          <strong><em>{report.score}</em> / 100</strong>
          <div className="score-line"><i style={{ width: `${report.score}%` }} /></div>
        </div>
      </section>

      <section className="evidence">
        <h2>Evidencia</h2>
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
        <h2>Datos recuperables</h2>
        {report.artifacts.length ? report.artifacts.map((artifact) => (
          <div className="artifact-row" key={artifact.id}>
            <div><b>{artifact.suggested_name}</b><span>{artifact.description}</span></div>
            <code>{formatBytes(artifact.size)} · {artifact.sha256.slice(0, 16)}…</code>
            <button onClick={() => extract(artifact)} disabled={extracting === artifact.id}>
              {extracting === artifact.id ? "Extrayendo…" : "Extraer de forma segura"}
            </button>
          </div>
        )) : <p className="empty">No se detectaron flujos recuperables con firma reconocible.</p>}
      </section>

      {error && <p className="error" role="alert">{error}</p>}
      <div className="result-actions">
        <button onClick={downloadReport}>Descargar informe JSON</button>
        <button onClick={reset}>Analizar otro archivo</button>
        <div><b>Métodos / Procedencia</b><span>{report.methods.join(" · ")}</span></div>
      </div>
      <footer className="limitation"><b>Limitación</b><span>{report.limitations.join(" ")}</span></footer>
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
