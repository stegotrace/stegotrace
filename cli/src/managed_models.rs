use std::{
    collections::BTreeMap,
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const ALETHEIA_COMMIT: &str = "1baf974ea8fcf0b51802935d9acbe59903d06845";
const PYTHON_VERSION: &str = "3.11.15";
const UV_VERSION: &str = "0.11.20";
const RESOLUTION_CUTOFF: &str = "2026-08-03T00:00:00Z";

struct FileSpec {
    filename: &'static str,
    label: &'static str,
    format: &'static str,
    algorithm: &'static str,
    bytes: u64,
    sha256: &'static str,
}

const FILES: &[FileSpec] = &[
    FileSpec {
        filename: "effnetb0-init.h5",
        label: "efficientnet_b0_base",
        format: "base",
        algorithm: "EfficientNet-B0",
        bytes: 16_804_768,
        sha256: "c1421ad80a9fc67c2cc4000f666aa50789ce39eedb4e06d531b0c593890ccff3",
    },
    FileSpec {
        filename: "effnetb0-A-alaska2-lsbm.h5",
        label: "lsbm_alaska2",
        format: "png",
        algorithm: "LSBM",
        bytes: 49_175_192,
        sha256: "ecfb2ad6ec5686032a4f081e9532ff0788b7fd04f594e787925089ebfa1b627b",
    },
    FileSpec {
        filename: "effnetb0-A-alaska2-lsbr.h5",
        label: "lsbr_alaska2",
        format: "png",
        algorithm: "LSBR",
        bytes: 49_175_192,
        sha256: "ce1812dd133cde2309fd75a4d96582bd6357d9dc3e3d9b70f69b8e1a02bf8e55",
    },
    FileSpec {
        filename: "effnetb0-A-alaska2-hill.h5",
        label: "hill_alaska2",
        format: "png",
        algorithm: "HILL",
        bytes: 49_175_192,
        sha256: "396043113c9a68dd85fba8a205ae5c7f8abd7873f3fc82c7e4a9f3950de8c4c9",
    },
    FileSpec {
        filename: "effnetb0-A-alaska2-steganogan.h5",
        label: "steganogan_alaska2",
        format: "png",
        algorithm: "SteganoGAN",
        bytes: 49_175_192,
        sha256: "b41c3e45dbb5987404781ed632d6009377a0631980e81ec2a592fc828ba546b1",
    },
    FileSpec {
        filename: "effnetb0-A-alaska2-juniw.h5",
        label: "j_uniward_alaska2",
        format: "jpeg",
        algorithm: "J-UNIWARD",
        bytes: 49_175_192,
        sha256: "17ce5501664d40c794356511a5346dd871cc62b8f25427a341677f63d03041a7",
    },
    FileSpec {
        filename: "effnetb0-A-alaska2-outguess.h5",
        label: "outguess_alaska2",
        format: "jpeg",
        algorithm: "OutGuess",
        bytes: 49_175_192,
        sha256: "28bfad0f826ade23d0e1e148fa356c0eb27897bb1b62e34e59f0b9d5dc55e498",
    },
    FileSpec {
        filename: "effnetb0-A-alaska2-nsf5.h5",
        label: "nsf5_alaska2",
        format: "jpeg",
        algorithm: "nsF5",
        bytes: 49_175_192,
        sha256: "1f4f55626fe76538877cd944dd8234acf0c1a510c48405e1cb06ec532ec3b30c",
    },
    FileSpec {
        filename: "effnetb0-A-alaska2-steghide.h5",
        label: "steghide_alaska2",
        format: "jpeg",
        algorithm: "Steghide",
        bytes: 49_175_192,
        sha256: "de4f7aa2fb7d6186f2b204e207b57e50fa7aabc96529211de0edac4177abac7e",
    },
];

#[derive(Deserialize)]
struct RunnerOutput {
    predictions: BTreeMap<String, f64>,
}

#[derive(Serialize)]
struct ManifestModel<'a> {
    label: &'a str,
    algorithm: &'a str,
    format: &'a str,
    filename: &'a str,
    bytes: u64,
    sha256: &'a str,
    source: String,
}

fn data_root() -> Result<PathBuf> {
    if let Some(custom) = env::var_os("STEGOTRACE_DATA_DIR") {
        return Ok(PathBuf::from(custom));
    }
    let user_home = env::var_os("HOME").context("HOME no está definido")?;
    Ok(PathBuf::from(user_home).join("Library/Application Support/StegoTrace"))
}

fn backend_root() -> Result<PathBuf> {
    Ok(data_root()?.join(format!("aletheia-{ALETHEIA_COMMIT}")))
}

fn file_sha256(path: &Path) -> Result<String> {
    let mut file =
        fs::File::open(path).with_context(|| format!("No se pudo leer {}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn source_url(filename: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/daniellerch/aletheia/{ALETHEIA_COMMIT}/aletheia-models/{filename}"
    )
}

fn download_verified(url: &str, destination: &Path, expected_sha256: &str) -> Result<()> {
    if destination.is_file() && file_sha256(destination)? == expected_sha256 {
        return Ok(());
    }
    let part = destination.with_extension(format!("part-{}", std::process::id()));
    eprintln!(
        "Descargando {}",
        destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    );
    let status = Command::new("curl")
        .args([
            "--proto",
            "=https",
            "--tlsv1.2",
            "--fail",
            "--location",
            "--progress-bar",
            url,
            "--output",
        ])
        .arg(&part)
        .status()
        .context("No se pudo ejecutar curl")?;
    if !status.success() {
        let _ = fs::remove_file(&part);
        bail!("Falló la descarga desde {url}");
    }
    let observed = file_sha256(&part)?;
    if observed != expected_sha256 {
        let _ = fs::remove_file(&part);
        bail!(
            "SHA-256 inválido para {}: {observed}",
            destination.display()
        );
    }
    fs::rename(&part, destination)
        .with_context(|| format!("No se pudo activar {}", destination.display()))?;
    Ok(())
}

fn command_works(program: &Path, argument: &str) -> bool {
    Command::new(program)
        .arg(argument)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn managed_uv(root: &Path) -> Result<PathBuf> {
    let system_uv = PathBuf::from("uv");
    if command_works(&system_uv, "--version") {
        return Ok(system_uv);
    }
    let (target, expected_sha256) = match env::consts::ARCH {
        "aarch64" => (
            "aarch64-apple-darwin",
            "0a2b6a757d5693671a7ce0002554ae869604e1e69acb10313ac14d08374be01a",
        ),
        "x86_64" => (
            "x86_64-apple-darwin",
            "bef01a86faab997f6022b45cfa29bfc5b090f2f72cd4a91d2ecefe641efdabe7",
        ),
        architecture => bail!("Arquitectura macOS no compatible: {architecture}"),
    };
    let tools = root.join("tools");
    fs::create_dir_all(&tools)?;
    let executable = tools.join(format!("uv-{target}/uv"));
    if command_works(&executable, "--version") {
        return Ok(executable);
    }
    let archive = tools.join(format!("uv-{target}.tar.gz"));
    download_verified(
        &format!(
            "https://github.com/astral-sh/uv/releases/download/{UV_VERSION}/uv-{target}.tar.gz"
        ),
        &archive,
        expected_sha256,
    )?;
    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(&archive)
        .args(["-C"])
        .arg(&tools)
        .status()
        .context("No se pudo extraer uv")?;
    if !status.success() || !command_works(&executable, "--version") {
        bail!("El binario gestionado de uv no quedó operativo");
    }
    Ok(executable)
}

fn run_uv(uv: &Path, root: &Path, arguments: &[&str]) -> Result<()> {
    let status = Command::new(uv)
        .args(arguments)
        .env("UV_PYTHON_INSTALL_DIR", root.join("python"))
        .env("UV_CACHE_DIR", root.join("cache"))
        .status()
        .with_context(|| format!("No se pudo ejecutar {}", uv.display()))?;
    if !status.success() {
        bail!("uv terminó con error al ejecutar: {}", arguments.join(" "));
    }
    Ok(())
}

fn install_runtime(root: &Path) -> Result<PathBuf> {
    let uv = managed_uv(root)?;
    run_uv(&uv, root, &["python", "install", PYTHON_VERSION])?;
    let venv = root.join("venv");
    let python = venv.join("bin/python");
    if !python.is_file() {
        run_uv(
            &uv,
            root,
            &["venv", "--python", PYTHON_VERSION, &venv.to_string_lossy()],
        )?;
    }
    let tensorflow = match env::consts::ARCH {
        "aarch64" => "tensorflow-macos==2.15.0",
        "x86_64" => "tensorflow==2.15.0",
        architecture => bail!("Arquitectura macOS no compatible: {architecture}"),
    };
    run_uv(
        &uv,
        root,
        &[
            "pip",
            "install",
            "--python",
            &python.to_string_lossy(),
            "--strict",
            "--exclude-newer",
            RESOLUTION_CUTOFF,
            "numpy==1.26.4",
            tensorflow,
            "efficientnet==1.1.1",
            "pillow==11.3.0",
        ],
    )?;
    Ok(python)
}

fn manifest() -> Value {
    let models: Vec<_> = FILES
        .iter()
        .map(|file| ManifestModel {
            label: file.label,
            algorithm: file.algorithm,
            format: file.format,
            filename: file.filename,
            bytes: file.bytes,
            sha256: file.sha256,
            source: source_url(file.filename),
        })
        .collect();
    json!({
        "schema_version": 1,
        "provider": "Aletheia",
        "provider_commit": ALETHEIA_COMMIT,
        "profile": "core",
        "python": PYTHON_VERSION,
        "uv": UV_VERSION,
        "resolution_cutoff": RESOLUTION_CUTOFF,
        "runner_sha256": hex::encode(Sha256::digest(ALETHEIA_RUNNER.as_bytes())),
        "models": models,
        "license": "Aletheia MIT",
        "license_file": "ALETHEIA-LICENSE.txt",
    })
}

pub fn configured() -> bool {
    backend_root().is_ok_and(|root| {
        root.join("manifest.json").is_file()
            && root.join("runner.py").is_file()
            && root.join("ALETHEIA-LICENSE.txt").is_file()
            && root.join("venv/bin/python").is_file()
            && FILES
                .iter()
                .all(|file| root.join("models").join(file.filename).is_file())
    })
}

pub fn status() -> Result<Value> {
    let root = backend_root()?;
    if !configured() {
        return Ok(json!({
            "provider": "Aletheia",
            "configured": false,
            "integrity": "not_installed",
            "path": root,
            "install_command": "stegotrace models install",
        }));
    }
    let models: Vec<_> = FILES
        .iter()
        .map(|file| {
            let path = root.join("models").join(file.filename);
            let observed = file_sha256(&path).unwrap_or_default();
            json!({
                "label": file.label,
                "algorithm": file.algorithm,
                "format": file.format,
                "bytes": file.bytes,
                "sha256": file.sha256,
                "verified": observed == file.sha256,
            })
        })
        .collect();
    let runner_verified = file_sha256(&root.join("runner.py"))?
        == hex::encode(Sha256::digest(ALETHEIA_RUNNER.as_bytes()));
    let verified = runner_verified && models.iter().all(|model| model["verified"] == true);
    Ok(json!({
        "provider": "Aletheia",
        "provider_commit": ALETHEIA_COMMIT,
        "configured": verified,
        "integrity": if verified { "verified" } else { "invalid" },
        "profile": "core",
        "path": root,
        "models": models,
        "runner_verified": runner_verified,
        "warning": "Las respuestas de red no son probabilidades calibradas y pueden degradarse por cover-source mismatch.",
    }))
}

pub fn install() -> Result<Value> {
    if env::consts::OS != "macos" {
        bail!("La instalación gestionada de modelos está publicada para macOS");
    }
    let root = backend_root()?;
    let model_dir = root.join("models");
    fs::create_dir_all(&model_dir)?;
    for file in FILES {
        download_verified(
            &source_url(file.filename),
            &model_dir.join(file.filename),
            file.sha256,
        )?;
    }
    let runner = root.join("runner.py");
    fs::write(&runner, ALETHEIA_RUNNER)?;
    fs::write(root.join("ALETHEIA-LICENSE.txt"), ALETHEIA_LICENSE)?;
    let python = install_runtime(&root)?;
    let smoke = Command::new(&python)
        .arg(&runner)
        .arg("--doctor")
        .arg(&model_dir)
        .arg(FILES[1].filename)
        .env("TF_CPP_MIN_LOG_LEVEL", "3")
        .output()
        .context("No se pudo verificar el entorno Aletheia")?;
    if !smoke.status.success() {
        bail!(
            "El entorno se instaló, pero la inferencia de control falló: {}",
            String::from_utf8_lossy(&smoke.stderr).trim()
        );
    }
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest())?,
    )?;
    status()
}

pub fn infer(path: &Path, format: &str) -> Result<BTreeMap<String, f64>> {
    if !matches!(format, "png" | "jpeg") {
        return Ok(BTreeMap::new());
    }
    let root = backend_root()?;
    if !configured() {
        return Ok(BTreeMap::new());
    }
    let mut command = Command::new(root.join("venv/bin/python"));
    command
        .arg(root.join("runner.py"))
        .arg(path)
        .arg(root.join("models"));
    for model in FILES.iter().filter(|model| model.format == format) {
        command.arg(format!("{}={}", model.label, model.filename));
    }
    let output = command
        .env("TF_CPP_MIN_LOG_LEVEL", "3")
        .output()
        .context("No se pudo iniciar la inferencia Aletheia")?;
    if !output.status.success() {
        bail!(
            "Aletheia terminó con error: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .context("Aletheia no devolvió resultados")?
        .to_owned();
    Ok(serde_json::from_str::<RunnerOutput>(&line)?.predictions)
}

const ALETHEIA_RUNNER: &str = r#"#!/usr/bin/env python3
"""Inference bridge derived from Aletheia's MIT-licensed EfficientNet-B0 path.

Upstream commit: 1baf974ea8fcf0b51802935d9acbe59903d06845
https://github.com/daniellerch/aletheia
"""
import json
import os
import signal
import sys
from pathlib import Path

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "-1")
os.environ.setdefault("TF_CPP_MIN_LOG_LEVEL", "3")
signal.alarm(300)

import numpy as np
from PIL import Image
import tensorflow as tf
import efficientnet.tfkeras as efn

SHAPE = (512, 512, 3)
Image.MAX_IMAGE_PIXELS = 40_000_000


def build_model(model_dir: Path):
    tf.config.optimizer.set_jit(False)
    tf.keras.mixed_precision.set_global_policy("float32")
    base = efn.EfficientNetB0(input_shape=SHAPE, weights=None, include_top=False)
    base.load_weights(model_dir / "effnetb0-init.h5")
    return tf.keras.Sequential([
        base,
        tf.keras.layers.GlobalAveragePooling2D(),
        tf.keras.layers.Dense(2, activation="softmax", dtype="float32"),
    ])


def starts(image_size: int, patch_size: int):
    if image_size <= patch_size:
        return [0]
    values = list(range(0, image_size - patch_size + 1, patch_size))
    last = image_size - patch_size
    if values[-1] != last:
        values.append(last)
    return values


def patches(image: np.ndarray):
    for row in starts(image.shape[0], SHAPE[0]):
        for col in starts(image.shape[1], SHAPE[1]):
            patch = np.zeros(SHAPE, dtype=np.uint8)
            height = min(SHAPE[0], image.shape[0] - row)
            width = min(SHAPE[1], image.shape[1] - col)
            patch[:height, :width] = image[row:row + height, col:col + width]
            yield patch


def predict(model, image: np.ndarray):
    total = 0.0
    count = 0
    batch = []
    for patch in patches(image):
        batch.append(patch)
        if len(batch) == 8:
            values = model.predict(np.asarray(batch, dtype="float32") / 255, verbose=0)[:, -1]
            total += float(values.sum())
            count += len(values)
            batch = []
    if batch:
        values = model.predict(np.asarray(batch, dtype="float32") / 255, verbose=0)[:, -1]
        total += float(values.sum())
        count += len(values)
    return total / count


def main():
    if sys.argv[1] == "--doctor":
        model_dir = Path(sys.argv[2])
        model = build_model(model_dir)
        model.load_weights(model_dir / sys.argv[3])
        print(json.dumps({"ok": True, "tensorflow": tf.__version__}, sort_keys=True))
        return
    image_path = Path(sys.argv[1])
    model_dir = Path(sys.argv[2])
    image = np.asarray(Image.open(image_path).convert("RGB"), dtype=np.uint8)
    if image.shape[0] * image.shape[1] > 40_000_000:
        raise ValueError("image exceeds 40 megapixels")
    model = build_model(model_dir)
    predictions = {}
    for item in sys.argv[3:]:
        label, filename = item.split("=", 1)
        model.load_weights(model_dir / filename)
        predictions[label] = round(predict(model, image), 6)
    print(json.dumps({"predictions": predictions}, sort_keys=True))


if __name__ == "__main__":
    main()
"#;

const ALETHEIA_LICENSE: &str = r#"MIT License

Copyright (c) 2017 Daniel Lerch

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"#;
