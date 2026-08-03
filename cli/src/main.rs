use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use image::ImageReader;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use statrs::distribution::{ChiSquared, ContinuousCDF};
use walkdir::WalkDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_PIXELS: u64 = 40_000_000;

const SIGNATURES: &[(&[u8], &str, &str)] = &[
    (b"PK\x03\x04", "zip", "application/zip"),
    (b"%PDF-", "pdf", "application/pdf"),
    (b"\x89PNG\r\n\x1a\n", "png", "image/png"),
    (b"\xff\xd8\xff", "jpeg", "image/jpeg"),
    (b"7z\xbc\xaf'\x1c", "7z", "application/x-7z-compressed"),
    (b"Rar!\x1a\x07", "rar", "application/vnd.rar"),
    (b"\x1f\x8b\x08", "gzip", "application/gzip"),
];

#[derive(Parser)]
#[command(version, about = "Esteganálisis auditable y extracción segura.")]
struct Cli {
    #[arg(long, global = true, help = "Emite JSON estable")]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Comprueba el binario y la integración científica opcional.
    Doctor,
    /// Lista métodos y sus límites.
    Methods,
    /// Analiza un archivo sin modificarlo.
    Scan { path: PathBuf },
    /// Extrae bytes identificados por scan sin abrirlos ni ejecutarlos.
    Extract {
        path: PathBuf,
        #[arg(long)]
        artifact: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Analiza todos los archivos de un directorio.
    Batch {
        directory: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(short, long)]
        recursive: bool,
    },
    /// Calcula AUC sobre corpus cover/stego etiquetados.
    Benchmark {
        #[arg(long)]
        cover: PathBuf,
        #[arg(long)]
        stego: PathBuf,
    },
    /// Muestra el estado del adaptador neuronal externo.
    Models,
}

#[derive(Clone, Serialize)]
struct Finding {
    id: String,
    category: &'static str,
    title: String,
    severity: &'static str,
    method: &'static str,
    value: Value,
    interpretation: &'static str,
    confidence: u8,
}

#[derive(Clone, Serialize)]
struct Artifact {
    id: String,
    kind: String,
    suggested_name: String,
    size: usize,
    sha256: String,
    description: String,
    extractor: &'static str,
    parameters: Value,
    mime: String,
}

#[derive(Serialize)]
struct ScientificResult {
    available: bool,
    provider: &'static str,
    methods: Vec<String>,
    predictions: BTreeMap<String, f64>,
    limitation: Option<String>,
}

#[derive(Serialize)]
struct Report {
    schema_version: &'static str,
    engine_version: &'static str,
    filename: String,
    media_type: String,
    size: usize,
    sha256: String,
    verdict: &'static str,
    score: u8,
    score_kind: &'static str,
    findings: Vec<Finding>,
    artifacts: Vec<Artifact>,
    scientific: ScientificResult,
    methods: Vec<&'static str>,
    limitations: Vec<&'static str>,
}

fn sha256(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn identify(data: &[u8]) -> (&'static str, &'static str) {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        ("image/png", "png")
    } else if data.starts_with(b"\xff\xd8\xff") {
        ("image/jpeg", "jpeg")
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        ("image/gif", "gif")
    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WAVE") {
        ("audio/wav", "wav")
    } else if data.starts_with(b"%PDF-") {
        ("application/pdf", "pdf")
    } else if data.starts_with(b"PK\x03\x04") {
        ("application/zip", "zip")
    } else {
        ("application/octet-stream", "unknown")
    }
}

fn canonical_end(data: &[u8], format: &str) -> Option<usize> {
    match format {
        "png" => {
            let mut offset = 8;
            while offset + 12 <= data.len() {
                let length = u32::from_be_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
                let end = offset.checked_add(12 + length)?;
                if end > data.len() {
                    return None;
                }
                if &data[offset + 4..offset + 8] == b"IEND" {
                    return Some(end);
                }
                offset = end;
            }
            None
        }
        "jpeg" => data
            .windows(2)
            .position(|window| window == b"\xff\xd9")
            .map(|at| at + 2),
        "gif" => data.iter().rposition(|byte| *byte == 0x3b).map(|at| at + 1),
        "wav" if data.len() >= 8 => {
            Some((8 + u32::from_le_bytes(data[4..8].try_into().ok()?) as usize).min(data.len()))
        }
        "pdf" => data
            .windows(5)
            .rposition(|window| window == b"%%EOF")
            .map(|at| at + 5),
        _ => None,
    }
}

fn slice_artifact(data: &[u8], name: &str, kind: &str, mime: &str, start: usize) -> Artifact {
    let payload = &data[start..];
    let id = sha256(format!("slice:{start}:{}:{kind}", data.len()).as_bytes())[..16].to_owned();
    Artifact {
        id,
        kind: kind.to_owned(),
        suggested_name: format!("{}-recovered.{kind}", file_stem(name)),
        size: payload.len(),
        sha256: sha256(payload),
        description: "Bytes tallados desde una firma o final canónico.".into(),
        extractor: "slice",
        parameters: json!({"start": start, "end": data.len()}),
        mime: mime.to_owned(),
    }
}

fn file_stem(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("recovered")
        .to_owned()
}

fn structural(data: &[u8], name: &str, format: &str) -> (Vec<Finding>, Vec<Artifact>) {
    let mut findings = Vec::new();
    let mut artifacts = Vec::new();
    let end = canonical_end(data, format);
    if let Some(start) = end.filter(|end| *end < data.len()) {
        let trailing = &data[start..];
        if trailing.iter().any(|byte| !b"\0\r\n \t".contains(byte)) {
            findings.push(Finding {
                id: "structure.trailing-data".into(), category: "structure", title: "Datos después del final canónico".into(),
                severity: "high", method: "container-boundary", value: json!({"offset": start, "bytes": trailing.len()}),
                interpretation: "El contenedor termina antes que el archivo; hay bytes adicionales recuperables.", confidence: 96,
            });
            let (kind, mime) = SIGNATURES
                .iter()
                .find(|(sig, _, _)| trailing.starts_with(sig))
                .map(|(_, kind, mime)| (*kind, *mime))
                .unwrap_or(("bin", "application/octet-stream"));
            artifacts.push(slice_artifact(data, name, kind, mime, start));
        }
    }
    let search_start = end.unwrap_or(16).max(16);
    for (signature, kind, mime) in SIGNATURES {
        if let Some(relative) = data[search_start.min(data.len())..]
            .windows(signature.len())
            .position(|window| window == *signature)
        {
            let offset = search_start + relative;
            if artifacts
                .iter()
                .any(|item| item.parameters["start"] == offset)
            {
                continue;
            }
            findings.push(Finding {
                id: format!("structure.embedded-{kind}-{offset}"),
                category: "structure",
                title: format!("Firma {} embebida", kind.to_uppercase()),
                severity: "high",
                method: "signature-carving",
                value: json!({"offset": offset, "signature": hex::encode(signature)}),
                interpretation: "Se encontró una cabecera conocida fuera de la cabecera principal.",
                confidence: 90,
            });
            artifacts.push(slice_artifact(data, name, kind, mime, offset));
        }
    }
    (findings, artifacts)
}

fn entropy(bits: &[u8]) -> f64 {
    if bits.is_empty() {
        return 0.0;
    }
    let ones = bits.iter().filter(|bit| **bit == 1).count() as f64;
    let p = ones / bits.len() as f64;
    [p, 1.0 - p]
        .into_iter()
        .filter(|value| *value > 0.0)
        .map(|value| -value * value.log2())
        .sum()
}

fn chi_square(values: &[u8]) -> (f64, f64) {
    let mut counts = [0_u64; 256];
    for value in values {
        counts[*value as usize] += 1;
    }
    let mut statistic = 0.0;
    let mut degrees: f64 = 0.0;
    for pair in counts.chunks_exact(2) {
        let expected = (pair[0] + pair[1]) as f64 / 2.0;
        if expected > 0.0 {
            statistic += (pair[0] as f64 - expected).powi(2) / expected
                + (pair[1] as f64 - expected).powi(2) / expected;
            degrees += 1.0;
        }
    }
    let p = ChiSquared::new(degrees.max(1.0))
        .map(|distribution| 1.0 - distribution.cdf(statistic))
        .unwrap_or(0.0);
    (statistic, p.clamp(0.0, 1.0))
}

fn runs(bits: &[u8]) -> (i64, f64) {
    if bits.len() < 2 {
        return (bits.len() as i64, 0.0);
    }
    let count = 1 + bits.windows(2).filter(|pair| pair[0] != pair[1]).count() as i64;
    let ones = bits.iter().filter(|bit| **bit == 1).count() as f64;
    let zeros = bits.len() as f64 - ones;
    if ones == 0.0 || zeros == 0.0 {
        return (count, 0.0);
    }
    let n = bits.len() as f64;
    let expected = 1.0 + 2.0 * ones * zeros / n;
    let variance = 2.0 * ones * zeros * (2.0 * ones * zeros - n) / (n * n * (n - 1.0));
    (
        count,
        (count as f64 - expected) / variance.max(1e-12).sqrt(),
    )
}

fn pixel_analysis(
    data: &[u8],
    name: &str,
    format: &str,
) -> Result<(Vec<Finding>, Vec<Artifact>, f64)> {
    if !matches!(format, "png" | "gif") {
        return Ok((vec![], vec![], 0.0));
    }
    let image = ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()?
        .decode()?;
    if image.width() as u64 * image.height() as u64 > MAX_PIXELS {
        bail!("La imagen excede 40 megapíxeles");
    }
    let rgb = image.to_rgb8();
    let raw = rgb.as_raw();
    let mut findings = Vec::new();
    let mut scores = Vec::new();
    for (channel, label) in ["R", "G", "B"].iter().enumerate() {
        let values: Vec<u8> = raw.iter().skip(channel).step_by(3).copied().collect();
        let bits: Vec<u8> = values.iter().map(|value| value & 1).collect();
        let (chi, p) = chi_square(&values);
        let (run_count, z) = runs(&bits);
        let bit_entropy = entropy(&bits);
        scores.push(100.0 * (0.55 * p + 0.45 * bit_entropy));
        findings.push(Finding {
            id: format!("statistics.lsb-{}", label.to_lowercase()), category: "statistics", title: format!("LSB · {label}"),
            severity: if p > 0.95 { "medium" } else { "info" }, method: "chi-square-entropy-runs",
            value: json!({"chi_square": chi, "p_value": p, "entropy": bit_entropy, "runs": run_count, "z_score": z}),
            interpretation: "Equiprobabilidad y aleatoriedad son compatibles con sustitución LSB, pero no son específicas.", confidence: (35.0 + 47.0 * p).min(82.0) as u8,
        });
    }
    let mut artifacts = Vec::new();
    for plane in 0..2 {
        for channels in [&[0_usize, 1, 2][..], &[0][..], &[1][..], &[2][..]] {
            for little in [false, true] {
                let stream = lsb_stream(raw, plane, channels, little);
                for (signature, kind, mime) in SIGNATURES {
                    if let Some(offset) = stream
                        .windows(signature.len())
                        .position(|window| window == *signature)
                    {
                        let parameters = json!({"plane": plane, "channels": channels, "bit_order": if little {"little"} else {"big"}, "start": offset, "end": stream.len()});
                        let id =
                            sha256(serde_json::to_string(&parameters)?.as_bytes())[..16].to_owned();
                        let payload = &stream[offset..];
                        findings.push(Finding { id: format!("extraction.lsb-{id}"), category: "extraction", title: format!("Firma {} en flujo LSB", kind.to_uppercase()), severity: "high", method: "lsb-signature-carving", value: parameters.clone(), interpretation: "La reconstrucción de bits contiene una firma conocida.", confidence: 94 });
                        artifacts.push(Artifact {
                            id,
                            kind: format!("lsb-{kind}"),
                            suggested_name: format!("{}-lsb.{kind}", file_stem(name)),
                            size: payload.len(),
                            sha256: sha256(payload),
                            description: "Flujo reconstruido desde un plano de bits.".into(),
                            extractor: "lsb",
                            parameters,
                            mime: (*mime).into(),
                        });
                    }
                }
            }
        }
    }
    Ok((findings, artifacts, scores.into_iter().fold(0.0, f64::max)))
}

fn lsb_stream(raw: &[u8], plane: u8, channels: &[usize], little: bool) -> Vec<u8> {
    let mut output = Vec::with_capacity(raw.len() / 8);
    let mut byte = 0_u8;
    let mut used = 0;
    for pixel in raw.chunks_exact(3) {
        for channel in channels {
            let bit = (pixel[*channel] >> plane) & 1;
            byte |= if little {
                bit << used
            } else {
                bit << (7 - used)
            };
            used += 1;
            if used == 8 {
                output.push(byte);
                byte = 0;
                used = 0;
            }
        }
    }
    output
}

fn scientific(path: &Path, format: &str) -> ScientificResult {
    let executable = std::env::var("STEGOTRACE_ALETHEIA_BIN").ok();
    if !matches!(format, "png" | "jpeg") || executable.is_none() {
        return ScientificResult {
            available: false,
            provider: "Aletheia",
            methods: vec![],
            predictions: BTreeMap::new(),
            limitation: Some(
                "Aletheia no está configurado; no se ejecutó inferencia neuronal.".into(),
            ),
        };
    }
    let result = Command::new(executable.unwrap())
        .args(["auto", &path.to_string_lossy()])
        .env("TF_CPP_MIN_LOG_LEVEL", "3")
        .output();
    match result {
        Ok(output) if output.status.success() => ScientificResult { available: true, provider: "Aletheia", methods: vec!["auto".into()], predictions: BTreeMap::new(), limitation: Some("La salida textual queda en el proveedor; use modelos específicos para puntuaciones reproducibles.".into()) },
        _ => ScientificResult { available: false, provider: "Aletheia", methods: vec![], predictions: BTreeMap::new(), limitation: Some("Aletheia terminó con error o no pudo ejecutarse.".into()) },
    }
}

fn analyze(path: &Path) -> Result<Report> {
    let data = fs::read(path).with_context(|| format!("No se pudo leer {}", path.display()))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("upload.bin")
        .to_owned();
    let (media_type, format) = identify(&data);
    let (mut findings, mut artifacts) = structural(&data, &name, format);
    let (pixel_findings, pixel_artifacts, statistical_score) =
        pixel_analysis(&data, &name, format)?;
    findings.extend(pixel_findings);
    artifacts.extend(pixel_artifacts);
    let structural_score = findings
        .iter()
        .filter(|item| item.severity == "high")
        .map(|item| item.confidence)
        .max()
        .unwrap_or(0);
    let score = structural_score.max((statistical_score * 0.72).min(72.0) as u8);
    let verdict = if score >= 75 {
        "Indicios fuertes compatibles con esteganografía"
    } else if score >= 50 {
        "Indicios que requieren revisión"
    } else if score >= 25 {
        "Indicios débiles o inespecíficos"
    } else {
        "Sin indicios relevantes en los métodos ejecutados"
    };
    let mut methods = vec!["container-boundary", "signature-carving"];
    if matches!(format, "png" | "gif") {
        methods.extend(["chi-square-entropy-runs", "lsb-signature-carving"]);
    }
    Ok(Report {
        schema_version: "1.0",
        engine_version: VERSION,
        filename: name,
        media_type: media_type.into(),
        size: data.len(),
        sha256: sha256(&data),
        verdict,
        score,
        score_kind: "heuristic_evidence_score",
        findings,
        artifacts,
        scientific: scientific(path, format),
        methods,
        limitations: vec![
            "La puntuación no es una probabilidad calibrada.",
            "Un negativo no demuestra ausencia y un positivo no identifica por sí solo el algoritmo.",
            "La extracción genérica no puede descifrar cargas protegidas por clave.",
        ],
    })
}

fn artifact_bytes(path: &Path, artifact_id: &str) -> Result<(Artifact, Vec<u8>)> {
    let report = analyze(path)?;
    let artifact = report
        .artifacts
        .into_iter()
        .find(|item| item.id == artifact_id)
        .ok_or_else(|| anyhow!("Artefacto no encontrado: {artifact_id}"))?;
    let data = fs::read(path)?;
    let payload = if artifact.extractor == "slice" {
        data[artifact.parameters["start"].as_u64().unwrap() as usize
            ..artifact.parameters["end"].as_u64().unwrap() as usize]
            .to_vec()
    } else {
        let image = ImageReader::new(std::io::Cursor::new(data))
            .with_guessed_format()?
            .decode()?
            .to_rgb8();
        let channels: Vec<usize> = serde_json::from_value(artifact.parameters["channels"].clone())?;
        let stream = lsb_stream(
            image.as_raw(),
            artifact.parameters["plane"].as_u64().unwrap() as u8,
            &channels,
            artifact.parameters["bit_order"] == "little",
        );
        stream[artifact.parameters["start"].as_u64().unwrap() as usize..].to_vec()
    };
    if sha256(&payload) != artifact.sha256 {
        bail!("La verificación SHA-256 del artefacto ha fallado");
    }
    Ok((artifact, payload))
}

fn auc(negative: &[u8], positive: &[u8]) -> Result<f64> {
    if negative.is_empty() || positive.is_empty() {
        bail!("Se necesita al menos un archivo cover y otro stego");
    }
    let favorable: f64 = positive
        .iter()
        .flat_map(|p| {
            negative.iter().map(move |n| {
                if p > n {
                    1.0
                } else if p == n {
                    0.5
                } else {
                    0.0
                }
            })
        })
        .sum();
    Ok(favorable / (negative.len() * positive.len()) as f64)
}

fn files(directory: &Path, recursive: bool) -> Vec<PathBuf> {
    WalkDir::new(directory)
        .max_depth(if recursive { usize::MAX } else { 1 })
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect()
}

fn print_value(value: &impl Serialize, json_output: bool) -> Result<()> {
    if json_output {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor => print_value(
            &json!({"ok": true, "version": VERSION, "runtime": "native-rust", "offline": true, "auth_required": false, "aletheia": std::env::var_os("STEGOTRACE_ALETHEIA_BIN").is_some()}),
            cli.json,
        ),
        Commands::Methods => print_value(
            &json!({"structure": ["container-boundary", "signature-carving"], "statistics": ["chi-square", "entropy", "runs"], "frontier_web": ["bounded JPEG antecedent search", "counterfactual re-embedding", "local evidence map"], "scientific": ["Aletheia adapter (optional)"], "extraction": ["byte slices", "signature-anchored LSB streams"]}),
            cli.json,
        ),
        Commands::Models => print_value(
            &json!({"provider": "Aletheia", "configured": std::env::var_os("STEGOTRACE_ALETHEIA_BIN").is_some(), "policy": "Pesos no redistribuidos; configure STEGOTRACE_ALETHEIA_BIN."}),
            cli.json,
        ),
        Commands::Scan { path } => {
            let report = analyze(&path)?;
            if cli.json {
                print_value(&report, true)
            } else {
                println!(
                    "{} · {}/100\nSHA-256: {}\nArtefactos recuperables: {}",
                    report.verdict,
                    report.score,
                    report.sha256,
                    report.artifacts.len()
                );
                Ok(())
            }
        }
        Commands::Extract {
            path,
            artifact,
            out,
        } => {
            if out.exists() {
                bail!("La salida ya existe; elige otra ruta");
            }
            let (metadata, payload) = artifact_bytes(&path, &artifact)?;
            fs::write(&out, &payload)?;
            print_value(
                &json!({"ok": true, "path": out, "bytes": payload.len(), "sha256": metadata.sha256}),
                cli.json,
            )
        }
        Commands::Batch {
            directory,
            out,
            recursive,
        } => {
            if out.exists() && out.read_dir()?.next().is_some() {
                bail!("El directorio de salida debe estar vacío");
            }
            fs::create_dir_all(&out)?;
            let mut results = Vec::new();
            for (index, path) in files(&directory, recursive).into_iter().enumerate() {
                match analyze(&path) {
                    Ok(report) => {
                        let destination = out.join(format!(
                            "{index:05}-{}.json",
                            path.file_name().unwrap().to_string_lossy()
                        ));
                        fs::write(&destination, serde_json::to_vec_pretty(&report)?)?;
                        results.push(json!({"path": path, "ok": true, "score": report.score, "report": destination}));
                    }
                    Err(error) => {
                        results.push(json!({"path": path, "ok": false, "error": error.to_string()}))
                    }
                }
            }
            print_value(
                &json!({"files": results.len(), "results": results}),
                cli.json,
            )
        }
        Commands::Benchmark { cover, stego } => {
            let covers: Vec<u8> = files(&cover, false)
                .iter()
                .map(|path| analyze(path).map(|report| report.score))
                .collect::<Result<_>>()?;
            let stegos: Vec<u8> = files(&stego, false)
                .iter()
                .map(|path| analyze(path).map(|report| report.score))
                .collect::<Result<_>>()?;
            print_value(
                &json!({"score_kind": "heuristic_evidence_score", "cover_files": covers.len(), "stego_files": stegos.len(), "roc_auc": auc(&covers, &stegos)?, "warning": "Resultado específico de este corpus."}),
                cli.json,
            )
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_png_and_trailing_zip() {
        let mut data = b"\x89PNG\r\n\x1a\n\0\0\0\0IEND\xaeB`\x82".to_vec();
        data.extend_from_slice(b"PK\x03\x04payload");
        let (findings, artifacts) = structural(&data, "test.png", "png");
        assert!(
            findings
                .iter()
                .any(|item| item.id == "structure.trailing-data")
        );
        assert_eq!(artifacts[0].kind, "zip");
    }

    #[test]
    fn lsb_stream_round_trip() {
        let payload = b"PK\x03\x04";
        let bits: Vec<u8> = payload
            .iter()
            .flat_map(|byte| (0..8).rev().map(move |shift| (byte >> shift) & 1))
            .collect();
        let mut raw = vec![128; bits.len() * 3];
        for (value, bit) in raw.iter_mut().step_by(3).zip(bits) {
            *value |= bit;
        }
        assert_eq!(&lsb_stream(&raw, 0, &[0], false)[..4], payload);
    }
}
