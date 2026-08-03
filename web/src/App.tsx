import { ChangeEvent, DragEvent, useRef, useState } from "react";
import SiteHeader from "./SiteHeader";
import type { Locale } from "./i18n";
import { copy, route } from "./i18n";
import type { Artifact, Report } from "./types";

const API_URL = (import.meta.env.VITE_API_URL as string | undefined)?.replace(/\/$/, "") || "http://localhost:8000";
const MAX_BYTES = 25 * 1024 * 1024;
const SAMPLE_URL = "/samples/stegotrace-lsb-zip.png";
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

export function CommandLine({ command, label, locale }: { command: string; label: string; locale: Locale }) {
  const idle = locale === "es" ? "Copiar" : "Copy";
  const [copyLabel, setCopyLabel] = useState(idle);

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      setCopyLabel(locale === "es" ? "Copiado" : "Copied");
      window.setTimeout(() => setCopyLabel(idle), 1800);
    } catch {
      setCopyLabel(locale === "es" ? "Selecciona el comando" : "Select the command");
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

function UploadView({ locale, onReport }: { locale: Locale; onReport: (file: File, report: Report) => void }) {
  const text = copy[locale].home;
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
      setError(text.tooLarge);
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
        throw new Error(detail?.detail || `${locale === "es" ? "El análisis falló" : "Analysis failed"} (${response.status}).`);
      }
      onReport(file, (await response.json()) as Report);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : text.failed);
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
          <h1>{text.title}</h1>
          <p className="lead">{text.lead}</p>
          <div
            className={`dropzone${dragging ? " is-dragging" : ""}`}
            onDragOver={(event) => { event.preventDefault(); setDragging(true); }}
            onDragLeave={() => setDragging(false)}
            onDrop={drop}
          >
            <div className="upload-mark" aria-hidden="true">↥</div>
            <p>{text.drag}</p>
            <button className="text-action" onClick={() => input.current?.click()}>{text.choose}</button>
            <small>{text.formats}</small>
            <span className="privacy-line">{text.privacy}</span>
            <input
              ref={input}
              type="file"
              aria-label={text.choose}
              onChange={(event: ChangeEvent<HTMLInputElement>) => choose(event.target.files?.[0])}
              hidden
            />
          </div>
          <div className="upload-actions">
            <button className="primary" onClick={analyze} disabled={!file || status === "analyzing"}>
              {status === "analyzing" ? text.analyzing : text.analyze}
            </button>
            <span>{file ? `${file.name} · ${formatBytes(file.size)}` : text.none}</span>
          </div>
          {status === "analyzing" && <div className="progress" role="progressbar"><span /></div>}
          {error && <p className="error" role="alert">{error}</p>}
          <p className="sample-link">{text.samplePrefix} <a href={SAMPLE_URL}>{text.sampleLink}</a>.</p>
        </div>

        <aside className="methods" id="methods">
          <h2>{text.methodsTitle}</h2>
          {text.methods.map(([title, description]) => <div className="method-group" key={title}><b>{title}</b><p>{description}</p></div>)}
          <div className="method-note">
            <b>{text.scoreTitle}</b>
            <p>{text.scoreText}</p>
          </div>
        </aside>
      </section>
      <section className="cli-install" id="cli">
        <div className="cli-intro">
          <span>{text.cliEyebrow}</span>
          <h2>{text.cliTitle}</h2>
          <p>{text.cliText}</p>
          <div className="cli-links"><a href={route(locale, "cli")} className="guide-link">{text.guide}</a><a href="/cli/v0.3.0/SHA256SUMS.txt">{text.checksums}</a></div>
        </div>
        <div className="cli-commands">
          <CommandLine
            label={text.install}
            command={INSTALL_COMMAND}
            locale={locale}
          />
          <CommandLine label={text.scan} command="stegotrace --json scan imagen.png > informe.json" locale={locale} />
          <CommandLine label={text.models} command="stegotrace models install" locale={locale} />
          <p>{text.modelText}</p>
        </div>
      </section>
      <footer className="principles" aria-label="Condiciones del servicio">
        {text.principles.map(([title, description]) => <div key={title}><b>{title}</b><span>{description}</span></div>)}
      </footer>
    </main>
  );
}

function ResultsView({ locale, file, report, reset }: { locale: Locale; file: File; report: Report; reset: () => void }) {
  const text = copy[locale].results;
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
      if (!response.ok) throw new Error(`${text.extractFailed} (${response.status}).`);
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = artifact.suggested_name;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : text.extractFailed);
    } finally {
      setExtracting(null);
    }
  }

  return (
    <main className="results">
      <h1>{text.title}</h1>
      <section className="summary">
        <div className="file-summary">
          <b>{report.filename}</b>
          <span>{text.type}: {report.media_type}</span>
          <span>{text.size}: {formatBytes(report.size)}</span>
          <span>SHA-256: <code>{report.sha256}</code></span>
        </div>
        <div className="score-summary">
          <b>{report.verdict}</b>
          <span>{text.score}</span>
          <strong><em>{report.score}</em><small>/ 100</small></strong>
          <div className="score-line"><i style={{ width: `${report.score}%` }} /></div>
        </div>
      </section>

      {!report.scientific.available && ["image/png", "image/jpeg"].includes(report.media_type) && (
        <section className="inconclusive-note">
          <b>{text.inconclusiveTitle}</b>
          <p>{text.inconclusiveText} <a href={route(locale, "cli")}>{text.inconclusiveLink}</a></p>
        </section>
      )}

      <section className="evidence">
        <h2>{text.evidence}</h2>
        <p className="table-hint">{text.tableHint}</p>
        <div className="table-wrap">
          <table>
            <thead><tr>{text.headers.map((header) => <th key={header}>{header}</th>)}</tr></thead>
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
        <h2>{text.artifacts}</h2>
        {report.artifacts.length ? report.artifacts.map((artifact) => (
          <div className="artifact-row" key={artifact.id}>
            <div><b>{artifact.suggested_name}</b><span>{artifact.description}</span></div>
            <code>{formatBytes(artifact.size)} · {artifact.sha256.slice(0, 16)}…</code>
            <button onClick={() => extract(artifact)} disabled={extracting === artifact.id}>
              {extracting === artifact.id ? text.extracting : text.extract}
            </button>
          </div>
        )) : <p className="empty">{text.noArtifacts}</p>}
      </section>

      {error && <p className="error" role="alert">{error}</p>}
      <div className="result-actions">
        <button onClick={downloadReport}>{text.report}</button>
        <button onClick={reset}>{text.another}</button>
        <div><b>{text.methods}</b><span>{report.methods.join(" · ")}</span></div>
      </div>
      <footer className="limitation"><b>{text.limit}</b><span>{report.limitations.join(" ")}</span></footer>
    </main>
  );
}

export default function App({ locale }: { locale: Locale }) {
  const [result, setResult] = useState<{ file: File; report: Report } | null>(null);
  const reset = () => setResult(null);
  return (
    <div className="app-shell">
      <SiteHeader locale={locale} page="home" />
      {result ? <ResultsView locale={locale} {...result} reset={reset} /> : <UploadView locale={locale} onReport={(file, report) => setResult({ file, report })} />}
    </div>
  );
}
