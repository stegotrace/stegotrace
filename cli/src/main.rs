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

mod managed_models;

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
    /// Instala y consulta modelos científicos gestionados.
    Models {
        #[command(subcommand)]
        command: ModelCommands,
    },
}

#[derive(Subcommand)]
enum ModelCommands {
    /// Instala el perfil Aletheia core con pesos verificados.
    Install,
    /// Verifica entorno, pesos, versión y procedencia.
    Status,
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

fn evidence_verdict(
    score: u8,
    scientific_available: bool,
    neural_score: f64,
    format: &str,
) -> &'static str {
    if score >= 75 {
        "Indicios fuertes compatibles con esteganografía"
    } else if score >= 50 {
        "Indicios que requieren revisión"
    } else if scientific_available && neural_score >= 50.0 {
        "Señal científica específica de fuente; requiere revisión"
    } else if matches!(format, "png" | "jpeg") && !scientific_available {
        "Análisis no concluyente sin perfil científico"
    } else if score >= 25 {
        "Indicios débiles o inespecíficos"
    } else {
        "Sin indicios relevantes en los métodos ejecutados"
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

fn rs_symmetry(values: &[u8]) -> f64 {
    let mut regular_positive = 0_i64;
    let mut singular_positive = 0_i64;
    let mut regular_negative = 0_i64;
    let mut singular_negative = 0_i64;
    for group in values.chunks_exact(4) {
        let discrimination = |samples: &[u8; 4]| {
            samples
                .windows(2)
                .map(|pair| (pair[0] as i16 - pair[1] as i16).unsigned_abs() as i64)
                .sum::<i64>()
        };
        let base: [u8; 4] = group.try_into().expect("chunk size");
        let positive = base.map(|value| {
            if value % 2 == 0 {
                value.saturating_add(1)
            } else {
                value.saturating_sub(1)
            }
        });
        let negative = base.map(|value| {
            if value % 2 == 0 {
                value.saturating_sub(1)
            } else {
                value.saturating_add(1)
            }
        });
        let reference = discrimination(&base);
        match discrimination(&positive).cmp(&reference) {
            std::cmp::Ordering::Greater => regular_positive += 1,
            std::cmp::Ordering::Less => singular_positive += 1,
            _ => {}
        }
        match discrimination(&negative).cmp(&reference) {
            std::cmp::Ordering::Greater => regular_negative += 1,
            std::cmp::Ordering::Less => singular_negative += 1,
            _ => {}
        }
    }
    let groups = (values.len() / 4).max(1) as f64;
    (1.0 - ((regular_positive - regular_negative).abs()
        + (singular_positive - singular_negative).abs()) as f64
        / (2.0 * groups))
        .clamp(0.0, 1.0)
}

fn low_order_score(raw: &[u8]) -> f64 {
    (0..3)
        .map(|channel| {
            let values: Vec<u8> = raw.iter().skip(channel).step_by(3).copied().collect();
            let bits: Vec<u8> = values.iter().map(|value| value & 1).collect();
            let (_, p) = chi_square(&values);
            100.0 * (0.55 * p + 0.25 * entropy(&bits) + 0.20 * rs_symmetry(&values))
        })
        .fold(0.0, f64::max)
}

fn counterfactual(raw: &[u8]) -> Value {
    let base = low_order_score(raw);
    let count = (raw.len() / 10).max(1);
    let mut scores = Vec::new();
    for seed in 0..5_usize {
        let mut candidate = raw.to_vec();
        for step in 0..count {
            let index = step
                .wrapping_mul(2_654_435_761)
                .wrapping_add(seed.wrapping_mul(97_531))
                % candidate.len();
            let bit =
                ((step.wrapping_mul(1_103_515_245).wrapping_add(seed * 12_345)) >> 16) as u8 & 1;
            candidate[index] = (candidate[index] & 0xfe) | bit;
        }
        scores.push(low_order_score(&candidate));
    }
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let deviation = (scores
        .iter()
        .map(|score| (score - mean).powi(2))
        .sum::<f64>()
        / scores.len() as f64)
        .sqrt();
    json!({"payload_fraction": 0.10, "repeats": 5, "base_score": base, "reembedded_mean_score": mean, "response_delta": mean - base, "response_std": deviation})
}

fn local_map(raw: &[u8], width: usize, height: usize) -> Value {
    let tile_size = 128;
    let mut tiles = Vec::new();
    for y in (0..height).step_by(tile_size) {
        for x in (0..width).step_by(tile_size) {
            let tile_width = tile_size.min(width - x);
            let tile_height = tile_size.min(height - y);
            if tile_width * tile_height < 1024 {
                continue;
            }
            let mut tile = Vec::with_capacity(tile_width * tile_height * 3);
            for row in y..y + tile_height {
                let start = (row * width + x) * 3;
                tile.extend_from_slice(&raw[start..start + tile_width * 3]);
            }
            tiles.push(json!({"x": x, "y": y, "width": tile_width, "height": tile_height, "score": low_order_score(&tile)}));
        }
    }
    tiles.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .partial_cmp(&a["score"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let analyzed = tiles.len();
    tiles.truncate(12);
    json!({"tile_size": tile_size, "tiles_analyzed": analyzed, "top_tiles": tiles})
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
    let width = rgb.width() as usize;
    let height = rgb.height() as usize;
    let raw = rgb.as_raw();
    let mut findings = Vec::new();
    let mut scores = Vec::new();
    for (channel, label) in ["R", "G", "B"].iter().enumerate() {
        let values: Vec<u8> = raw.iter().skip(channel).step_by(3).copied().collect();
        let bits: Vec<u8> = values.iter().map(|value| value & 1).collect();
        let (chi, p) = chi_square(&values);
        let (run_count, z) = runs(&bits);
        let bit_entropy = entropy(&bits);
        let rs = rs_symmetry(&values);
        scores.push(100.0 * (0.55 * p + 0.25 * bit_entropy + 0.20 * rs));
        findings.push(Finding {
            id: format!("statistics.lsb-{}", label.to_lowercase()), category: "statistics", title: format!("LSB · {label}"),
            severity: if p > 0.95 { "medium" } else { "info" }, method: "chi-square-entropy-runs",
            value: json!({"chi_square": chi, "p_value": p, "entropy": bit_entropy, "runs": run_count, "z_score": z, "rs_symmetry": rs}),
            interpretation: "Equiprobabilidad y aleatoriedad son compatibles con sustitución LSB, pero no son específicas.", confidence: (35.0 + 47.0 * p).min(82.0) as u8,
        });
    }
    let counterfactual_value = counterfactual(raw);
    let saturated = counterfactual_value["response_delta"]
        .as_f64()
        .unwrap_or_default()
        < 2.0;
    findings.push(Finding {
        id: "frontier.counterfactual-reembedding".into(), category: "frontier", title: "Calibración contrafactual por re-embebido".into(), severity: if saturated { "medium" } else { "info" }, method: "subsequent-embedding-calibration", value: counterfactual_value,
        interpretation: "Una respuesta saturada es compatible con modificación previa, pero depende de fuente y algoritmo.", confidence: if saturated { 66 } else { 48 },
    });
    findings.push(Finding {
        id: "frontier.local-evidence-map".into(), category: "frontier", title: "Mapa local de evidencia".into(), severity: "info", method: "tiled-low-order-steganalysis", value: local_map(raw, width, height),
        interpretation: "Localiza regiones para revisión; no segmenta de forma concluyente los bits modificados.", confidence: 58,
    });
    let mut artifacts = Vec::new();
    for plane in 0..2 {
        for channels in [&[0_usize, 1, 2][..], &[0][..], &[1][..], &[2][..]] {
            for little in [false, true] {
                let stream = lsb_stream(raw, plane, channels, little);
                for (signature, kind, mime) in SIGNATURES {
                    if let Some((start, end)) = valid_signature(&stream, signature, kind) {
                        let parameters = json!({"plane": plane, "channels": channels, "bit_order": if little {"little"} else {"big"}, "start": start, "end": end});
                        let id =
                            sha256(serde_json::to_string(&parameters)?.as_bytes())[..16].to_owned();
                        let payload = &stream[start..end];
                        findings.push(Finding { id: format!("extraction.lsb-{id}"), category: "extraction", title: format!("Contenedor {} válido en flujo LSB", kind.to_uppercase()), severity: "high", method: "lsb-signature-carving", value: parameters.clone(), interpretation: "La reconstrucción de bits contiene una firma y un final de contenedor coherentes.", confidence: 96 });
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
    let (protocol_findings, protocol_artifacts) = protocol_artifacts(raw, width, height, name)?;
    findings.extend(protocol_findings);
    artifacts.extend(protocol_artifacts);
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

fn zsteg_stream(
    raw: &[u8],
    width: usize,
    height: usize,
    bit_depth: u8,
    channels: &[usize],
    reverse_rows: bool,
) -> Vec<u8> {
    let mut output = Vec::with_capacity(raw.len() * bit_depth as usize / 8);
    let mut byte = 0_u8;
    let mut used = 0;
    for row in 0..height {
        let y = if reverse_rows { height - 1 - row } else { row };
        for x in 0..width {
            let pixel = &raw[(y * width + x) * 3..][..3];
            for channel in channels {
                for plane in (0..bit_depth).rev() {
                    byte |= ((pixel[*channel] >> plane) & 1) << (7 - used);
                    used += 1;
                    if used == 8 {
                        output.push(byte);
                        byte = 0;
                        used = 0;
                    }
                }
            }
        }
    }
    output
}

fn protocol_artifacts(
    raw: &[u8],
    width: usize,
    height: usize,
    name: &str,
) -> Result<(Vec<Finding>, Vec<Artifact>)> {
    let mut findings = Vec::new();
    let mut artifacts = Vec::new();

    let openstego = zsteg_stream(raw, width, height, 1, &[0, 1, 2], false);
    if let Some(start) = openstego
        .windows(9)
        .position(|window| window == b"OPENSTEGO")
        .filter(|start| start + 18 <= openstego.len())
    {
        let version = openstego[start + 9];
        let data_size = u32::from_le_bytes(openstego[start + 10..start + 14].try_into()?) as usize;
        let channel_bits = openstego[start + 14];
        let name_size = openstego[start + 15] as usize;
        let compressed = openstego[start + 16] != 0;
        let encrypted = openstego[start + 17] != 0;
        let name_end = start + 18 + name_size;
        let end = name_end.saturating_add(data_size);
        if matches!(version, 1 | 2)
            && (1..=8).contains(&channel_bits)
            && data_size > 0
            && end <= openstego.len()
            && name_end <= openstego.len()
            && openstego[start + 18..name_end]
                .iter()
                .all(u8::is_ascii_graphic)
        {
            let internal_name = String::from_utf8_lossy(&openstego[start + 18..name_end]);
            let parameters = json!({"plane": 0, "channels": [0, 1, 2], "bit_order": "big", "start": start, "end": end});
            let id = sha256(serde_json::to_string(&parameters)?.as_bytes())[..16].to_owned();
            let payload = &openstego[start..end];
            findings.push(Finding {
                id: format!("extraction.openstego-{id}"),
                category: "extraction",
                title: "Contenedor OpenStego validado".into(),
                severity: "high",
                method: "openstego-v1-header",
                value: json!({"version": version, "data_bytes": data_size, "channel_bits": channel_bits, "filename": internal_name, "compressed": compressed, "encrypted": encrypted}),
                interpretation: "La cabecera, longitudes y nombre interno forman un contenedor OpenStego coherente.",
                confidence: 97,
            });
            artifacts.push(Artifact {
                id,
                kind: "openstego".into(),
                suggested_name: format!("{}-openstego.bin", file_stem(name)),
                size: payload.len(),
                sha256: sha256(payload),
                description: format!("Contenedor OpenStego; nombre interno: {internal_name}."),
                extractor: "lsb",
                parameters,
                mime: "application/octet-stream".into(),
            });
        }
    }

    let wbstego = zsteg_stream(raw, width, height, 1, &[2, 1, 0], true);
    if wbstego.len() >= 6 {
        let declared =
            wbstego[0] as usize | ((wbstego[1] as usize) << 8) | ((wbstego[2] as usize) << 16);
        let end = 3_usize.saturating_add(declared);
        if declared >= 4 && end <= wbstego.len() {
            let extension = &wbstego[3..6];
            let message = &wbstego[6..end];
            let printable = message
                .iter()
                .all(|byte| matches!(byte, b'\t' | b'\n' | b'\r' | 0x20..=0x7e));
            let meaningful = message
                .iter()
                .filter(|byte| !matches!(byte, b'\t' | b'\n' | b'\r' | b' '))
                .count();
            if extension.iter().all(u8::is_ascii_alphanumeric) && printable && meaningful >= 8 {
                let extension = String::from_utf8_lossy(extension).to_ascii_lowercase();
                let parameters = json!({"bit_depth": 1, "channels": [2, 1, 0], "reverse_rows": true, "start": 6, "end": end});
                let id = sha256(serde_json::to_string(&parameters)?.as_bytes())[..16].to_owned();
                findings.push(Finding {
                    id: format!("extraction.wbstego-{id}"),
                    category: "extraction",
                    title: "Carga wbStego sin cifrar validada".into(),
                    severity: "high",
                    method: "wbstego-plain-header",
                    value: json!({"declared_bytes": declared, "extension": extension}),
                    interpretation: "El tamaño declarado, la extensión y el contenido forman una carga wbStego coherente.",
                    confidence: 96,
                });
                artifacts.push(Artifact {
                    id,
                    kind: "wbstego-text".into(),
                    suggested_name: format!("{}-wbstego.{extension}", file_stem(name)),
                    size: message.len(),
                    sha256: sha256(message),
                    description: "Texto sin cifrar recuperado de una carga wbStego.".into(),
                    extractor: "lsb-zsteg",
                    parameters,
                    mime: "text/plain".into(),
                });
            }
        }
    }

    for bit_depth in 2..=4 {
        let stream = zsteg_stream(raw, width, height, bit_depth, &[0, 1, 2], false);
        let Some(end) = stream.iter().position(|byte| *byte == 0) else {
            continue;
        };
        if end < 12
            || stream.get(end..end + 4) != Some(&[0, 0, 0, 0])
            || !stream[..end]
                .iter()
                .all(|byte| matches!(byte, b'\t' | b'\n' | b'\r' | 0x20..=0x7e))
        {
            continue;
        }
        let parameters = json!({"bit_depth": bit_depth, "channels": [0, 1, 2], "reverse_rows": false, "start": 0, "end": end});
        let id = sha256(serde_json::to_string(&parameters)?.as_bytes())[..16].to_owned();
        let payload = &stream[..end];
        findings.push(Finding {
            id: format!("extraction.multibit-text-{id}"),
            category: "extraction",
            title: format!("Texto en {bit_depth} bits bajos por canal"),
            severity: "high",
            method: "multibit-lsb-text",
            value: json!({"bit_depth": bit_depth, "bytes": payload.len(), "channels": "RGB"}),
            interpretation: "Una secuencia ASCII terminada en nulos ocupa varios bits bajos de cada canal.",
            confidence: 95,
        });
        artifacts.push(Artifact {
            id,
            kind: "lsb-text".into(),
            suggested_name: format!("{}-{bit_depth}bit-lsb.txt", file_stem(name)),
            size: payload.len(),
            sha256: sha256(payload),
            description: format!("Texto recuperado de los {bit_depth} bits bajos RGB."),
            extractor: "lsb-zsteg",
            parameters,
            mime: "text/plain".into(),
        });
        break;
    }

    Ok((findings, artifacts))
}

fn validated_payload_end(stream: &[u8], start: usize, kind: &str) -> Option<usize> {
    let payload = stream.get(start..)?;
    let relative_end = match kind {
        "jpeg" | "png" | "gif" => {
            let end = canonical_end(payload, kind)?;
            ImageReader::new(std::io::Cursor::new(&payload[..end]))
                .with_guessed_format()
                .ok()?
                .decode()
                .ok()?;
            end
        }
        "pdf" => {
            let end = canonical_end(payload, "pdf")?;
            let document = &payload[..end];
            if !document.windows(9).any(|window| window == b"startxref")
                || !document.windows(8).any(|window| window == b"/Catalog")
            {
                return None;
            }
            end
        }
        "zip" => {
            let eocd = payload
                .windows(4)
                .position(|window| window == b"PK\x05\x06")?;
            if eocd + 22 > payload.len() {
                return None;
            }
            let comment =
                u16::from_le_bytes(payload[eocd + 20..eocd + 22].try_into().ok()?) as usize;
            let directory_size =
                u32::from_le_bytes(payload[eocd + 12..eocd + 16].try_into().ok()?) as usize;
            let directory_start =
                u32::from_le_bytes(payload[eocd + 16..eocd + 20].try_into().ok()?) as usize;
            if directory_start.checked_add(directory_size)? > eocd {
                return None;
            }
            eocd.checked_add(22 + comment)?
        }
        _ => return None,
    };
    start
        .checked_add(relative_end)
        .filter(|end| *end <= stream.len())
}

fn valid_signature(stream: &[u8], signature: &[u8], kind: &str) -> Option<(usize, usize)> {
    let mut search = 0;
    let mut attempts = 0;
    while search + signature.len() <= stream.len() && attempts < 64 {
        let relative = stream[search..]
            .windows(signature.len())
            .position(|window| window == signature)?;
        let start = search + relative;
        if let Some(end) = validated_payload_end(stream, start, kind) {
            return Some((start, end));
        }
        attempts += 1;
        search = start + 1;
    }
    None
}

fn scientific(path: &Path, format: &str) -> ScientificResult {
    if managed_models::configured() && matches!(format, "png" | "jpeg") {
        return match managed_models::infer(path, format) {
            Ok(predictions) if !predictions.is_empty() => ScientificResult {
                available: true,
                provider: "Aletheia",
                methods: predictions.keys().cloned().collect(),
                predictions,
                limitation: Some(
                    "Respuestas específicas de ALASKA2; no son probabilidades calibradas y pueden degradarse por cover-source mismatch."
                        .into(),
                ),
            },
            Ok(_) => ScientificResult {
                available: false,
                provider: "Aletheia",
                methods: vec![],
                predictions: BTreeMap::new(),
                limitation: Some("No hay un modelo gestionado para este formato.".into()),
            },
            Err(error) => ScientificResult {
                available: false,
                provider: "Aletheia",
                methods: vec![],
                predictions: BTreeMap::new(),
                limitation: Some(format!("La inferencia gestionada falló: {error}")),
            },
        };
    }
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
        Ok(output) if output.status.success() => ScientificResult { available: false, provider: "Aletheia", methods: vec!["auto".into()], predictions: BTreeMap::new(), limitation: Some("El adaptador externo terminó, pero no devolvió predicciones estructuradas; use `stegotrace models install`.".into()) },
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
    let scientific = scientific(path, format);
    let neural_score = scientific.predictions.values().copied().fold(0.0, f64::max) * 100.0;
    if scientific.available {
        findings.push(Finding {
            id: "scientific.aletheia-effnetb0".into(),
            category: "scientific",
            title: "Respuestas de detectores Aletheia".into(),
            severity: if neural_score >= 50.0 { "medium" } else { "info" },
            method: "aletheia-effnetb0-alaska2",
            value: json!(&scientific.predictions),
            interpretation: "Cada valor es la respuesta de un detector específico; el dominio de entrenamiento y la fuente condicionan su validez.",
            confidence: 65,
        });
    }
    let score = structural_score.max((statistical_score * 0.72).min(72.0) as u8);
    let verdict = evidence_verdict(score, scientific.available, neural_score, format);
    let mut methods = vec!["container-boundary", "signature-carving"];
    if matches!(format, "png" | "gif") {
        methods.extend([
            "chi-square-entropy-runs",
            "regular-singular-analysis",
            "subsequent-embedding-calibration",
            "tiled-low-order-steganalysis",
            "lsb-signature-carving",
            "openstego-v1-header",
            "wbstego-plain-header",
            "multibit-lsb-text",
        ]);
    }
    if scientific.available {
        methods.push("aletheia-effnetb0-alaska2");
    }
    let mut limitations = vec![
        "La puntuación no es una probabilidad calibrada.",
        "Un negativo no demuestra ausencia y un positivo no identifica por sí solo el algoritmo.",
        "La extracción genérica no puede descifrar cargas protegidas por clave.",
    ];
    if matches!(format, "png" | "jpeg") && !scientific.available {
        limitations.push(
            "No se ejecutaron detectores neuronales específicos de fuente; el resultado PNG/JPEG no puede considerarse negativo.",
        );
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
        scientific,
        methods,
        limitations,
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
    } else if artifact.extractor == "lsb" {
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
        stream[artifact.parameters["start"].as_u64().unwrap() as usize
            ..artifact.parameters["end"].as_u64().unwrap() as usize]
            .to_vec()
    } else {
        let image = ImageReader::new(std::io::Cursor::new(data))
            .with_guessed_format()?
            .decode()?
            .to_rgb8();
        let channels: Vec<usize> = serde_json::from_value(artifact.parameters["channels"].clone())?;
        let stream = zsteg_stream(
            image.as_raw(),
            image.width() as usize,
            image.height() as usize,
            artifact.parameters["bit_depth"].as_u64().unwrap() as u8,
            &channels,
            artifact.parameters["reverse_rows"].as_bool().unwrap(),
        );
        stream[artifact.parameters["start"].as_u64().unwrap() as usize
            ..artifact.parameters["end"].as_u64().unwrap() as usize]
            .to_vec()
    };
    if sha256(&payload) != artifact.sha256 {
        bail!("La verificación SHA-256 del artefacto ha fallado");
    }
    Ok((artifact, payload))
}

fn auc(negative: &[f64], positive: &[f64]) -> Result<f64> {
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
            &json!({"ok": true, "version": VERSION, "runtime": "native-rust", "offline": true, "auth_required": false, "aletheia": managed_models::configured() || std::env::var_os("STEGOTRACE_ALETHEIA_BIN").is_some()}),
            cli.json,
        ),
        Commands::Methods => print_value(
            &json!({"structure": ["container-boundary", "signature-carving"], "statistics": ["chi-square", "RS", "entropy", "runs"], "frontier_native": ["counterfactual re-embedding", "local evidence map"], "frontier_api": ["bounded JPEG antecedent search"], "scientific": ["Aletheia adapter (optional)"], "extraction": ["byte slices", "signature-anchored LSB streams"]}),
            cli.json,
        ),
        Commands::Models { command } => match command {
            ModelCommands::Install => print_value(&managed_models::install()?, cli.json),
            ModelCommands::Status => print_value(&managed_models::status()?, cli.json),
        },
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
            let covers: Vec<Report> = files(&cover, false)
                .iter()
                .map(|path| analyze(path))
                .collect::<Result<_>>()?;
            let stegos: Vec<Report> = files(&stego, false)
                .iter()
                .map(|path| analyze(path))
                .collect::<Result<_>>()?;
            let heuristic_covers: Vec<_> =
                covers.iter().map(|report| report.score as f64).collect();
            let heuristic_stegos: Vec<_> =
                stegos.iter().map(|report| report.score as f64).collect();
            let scientific_covers: Vec<_> = covers
                .iter()
                .filter(|report| report.scientific.available)
                .map(|report| {
                    report
                        .scientific
                        .predictions
                        .values()
                        .copied()
                        .fold(0.0, f64::max)
                })
                .collect();
            let scientific_stegos: Vec<_> = stegos
                .iter()
                .filter(|report| report.scientific.available)
                .map(|report| {
                    report
                        .scientific
                        .predictions
                        .values()
                        .copied()
                        .fold(0.0, f64::max)
                })
                .collect();
            let combined_covers: Vec<_> = covers
                .iter()
                .map(|report| {
                    (report.score as f64 / 100.0).max(
                        report
                            .scientific
                            .predictions
                            .values()
                            .copied()
                            .fold(0.0, f64::max),
                    )
                })
                .collect();
            let combined_stegos: Vec<_> = stegos
                .iter()
                .map(|report| {
                    (report.score as f64 / 100.0).max(
                        report
                            .scientific
                            .predictions
                            .values()
                            .copied()
                            .fold(0.0, f64::max),
                    )
                })
                .collect();
            let scientific = if scientific_covers.is_empty() || scientific_stegos.is_empty() {
                Value::Null
            } else {
                json!({
                    "score_kind": "source_specific_model_response",
                    "cover_files": scientific_covers.len(),
                    "stego_files": scientific_stegos.len(),
                    "roc_auc": auc(&scientific_covers, &scientific_stegos)?,
                    "warning": "Respuesta máxima entre modelos; no es una probabilidad calibrada y el resultado es específico del corpus."
                })
            };
            print_value(
                &json!({
                    "score_kind": "heuristic_evidence_score",
                    "cover_files": covers.len(),
                    "stego_files": stegos.len(),
                    "roc_auc": auc(&heuristic_covers, &heuristic_stegos)?,
                    "scientific": scientific,
                    "combined": {
                        "score_kind": "evidence_envelope_rank",
                        "roc_auc": auc(&combined_covers, &combined_stegos)?,
                        "warning": "Máximo entre evidencia nativa normalizada y respuesta científica; sirve para ordenar este corpus, no es una probabilidad calibrada."
                    },
                    "warning": "Resultado específico de este corpus."
                }),
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

    fn embed_zsteg(
        payload: &[u8],
        bit_depth: u8,
        channels: &[usize],
        reverse_rows: bool,
    ) -> Vec<u8> {
        let width = 96;
        let height = 96;
        let mut raw = vec![128_u8; width * height * 3];
        let slots_per_pixel = bit_depth as usize * channels.len();
        for index in 0..payload.len() * 8 {
            let bit = (payload[index / 8] >> (7 - index % 8)) & 1;
            let pixel_index = index / slots_per_pixel;
            let slot = index % slots_per_pixel;
            let row = pixel_index / width;
            let y = if reverse_rows { height - 1 - row } else { row };
            let x = pixel_index % width;
            let channel = channels[slot / bit_depth as usize];
            let plane = bit_depth - 1 - (slot % bit_depth as usize) as u8;
            let target = (y * width + x) * 3 + channel;
            raw[target] = (raw[target] & !(1 << plane)) | (bit << plane);
        }
        raw
    }

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

    #[test]
    fn jpeg_carving_requires_a_decodable_container() {
        let mut encoded = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut encoded)
            .encode(&[32, 64, 96], 1, 1, image::ExtendedColorType::Rgb8)
            .unwrap();
        let mut stream = b"\xff\xd8\xffnot-a-jpeg\xff\xd9padding".to_vec();
        let expected_start = stream.len();
        stream.extend_from_slice(&encoded);
        let (start, end) = valid_signature(&stream, b"\xff\xd8\xff", "jpeg").unwrap();
        assert_eq!(start, expected_start);
        assert_eq!(end, stream.len());
    }

    #[test]
    fn source_specific_signal_never_reads_as_absence() {
        assert_eq!(
            evidence_verdict(0, true, 98.0, "jpeg"),
            "Señal científica específica de fuente; requiere revisión"
        );
        assert_eq!(
            evidence_verdict(0, false, 98.0, "jpeg"),
            "Análisis no concluyente sin perfil científico"
        );
        assert_eq!(
            evidence_verdict(0, false, 0.0, "pdf"),
            "Sin indicios relevantes en los métodos ejecutados"
        );
    }

    #[test]
    fn protocol_extractors_cover_openstego_wbstego_and_multibit_text() {
        let mut openstego = b"OPENSTEGO\x01\x04\x00\x00\x00\x01\x08\x01\x01flag.txt".to_vec();
        openstego.extend_from_slice(b"data");
        let raw = embed_zsteg(&openstego, 1, &[0, 1, 2], false);
        let (_, artifacts) = protocol_artifacts(&raw, 96, 96, "open.png").unwrap();
        assert!(artifacts.iter().any(|item| item.kind == "openstego"));

        let message = b"SuperSecretMessage\n";
        let mut wbstego = (message.len() as u32 + 3).to_le_bytes()[..3].to_vec();
        wbstego.extend_from_slice(b"txt");
        wbstego.extend_from_slice(message);
        let raw = embed_zsteg(&wbstego, 1, &[2, 1, 0], true);
        let (_, artifacts) = protocol_artifacts(&raw, 96, 96, "wb.png").unwrap();
        assert!(artifacts.iter().any(|item| item.kind == "wbstego-text"));

        let raw = embed_zsteg(b"SuperSecretMessage\0\0\0\0", 3, &[0, 1, 2], false);
        let (_, artifacts) = protocol_artifacts(&raw, 96, 96, "rgb3.png").unwrap();
        assert!(artifacts.iter().any(|item| item.kind == "lsb-text"));
    }
}
