//! PoC mínimo: captura 1 frame da câmera do celular (via scrcpy) e envia para o Telegram.
//!
//! Requisitos no host/container:
//!   - adb
//!   - scrcpy (versão recente com suporte a --video-source=camera e --time-limit)
//!   - ffmpeg
//!   - celular Android 12+ com USB debugging (ou ADB over TCP) autorizado
//!
//! Uso:
//!   TELEGRAM_BOT_TOKEN=xxx TELEGRAM_CHAT_ID=123 \
//!     phone-cam-telegram --facing back
//!
//!   phone-cam-telegram --facing front --caption "Selfie de teste"

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use reqwest::blocking::multipart;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, ValueEnum)]
enum Facing {
    Front,
    Back,
}

#[derive(Parser, Debug)]
#[command(name = "phonecam", about = "Captura câmera via scrcpy e envia ao Telegram")]
struct Args {
    /// Câmera a usar
    #[arg(long, value_enum, default_value_t = Facing::Back)]
    facing: Facing,

    /// Token do bot Telegram (ou env TELEGRAM_BOT_TOKEN)
    #[arg(long, env = "TELEGRAM_BOT_TOKEN")]
    bot_token: String,

    /// Chat ID de destino (ou env TELEGRAM_CHAT_ID)
    #[arg(long, env = "TELEGRAM_CHAT_ID")]
    chat_id: String,

    /// Legenda opcional da foto
    #[arg(long, default_value = "")]
    caption: String,

    /// Duração da gravação em segundos (scrcpy --time-limit)
    #[arg(long, default_value_t = 3)]
    duration: u32,

    /// Resolução máxima (scrcpy --max-size)
    #[arg(long, default_value_t = 1280)]
    max_size: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("▶ Câmera: {:?}", args.facing);
    println!("▶ Gravando {}s via scrcpy...", args.duration);

    let dir = tempfile::tempdir().context("criar diretório temporário")?;
    let video_path = dir.path().join("capture.mp4");
    let frame_path = dir.path().join("frame.jpg");

    capture_camera(&args, &video_path)?;
    extract_frame(&video_path, &frame_path)?;
    send_telegram(&args, &frame_path)?;

    println!("✅ Foto enviada com sucesso para o chat {}", args.chat_id);
    Ok(())
}

fn capture_camera(args: &Args, out: &Path) -> Result<()> {
    let facing = match args.facing {
        Facing::Front => "front",
        Facing::Back => "back",
    };

    // scrcpy grava a câmera por N segundos e encerra sozinho (--time-limit)
    let status = Command::new("scrcpy")
        .args([
            "--video-source=camera",
            &format!("--camera-facing={facing}"),
            "--no-audio",
            "--no-playback",
            "--no-window",
            &format!("--max-size={}", args.max_size),
            &format!("--time-limit={}", args.duration),
            &format!("--record={}", out.display()),
        ])
        .status()
        .context("falha ao executar scrcpy (está instalado e no PATH?)")?;

    if !status.success() {
        bail!("scrcpy terminou com código {:?}", status.code());
    }

    if !out.exists() || out.metadata()?.len() == 0 {
        bail!("arquivo de vídeo não foi gerado: {}", out.display());
    }

    println!("  vídeo gerado: {} ({} bytes)", out.display(), out.metadata()?.len());
    Ok(())
}

fn extract_frame(video: &Path, frame: &Path) -> Result<()> {
    // Pega um frame perto do meio da gravação (mais estável que o primeiro)
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-ss",
            "1",
            "-i",
            &video.display().to_string(),
            "-frames:v",
            "1",
            "-q:v",
            "2",
            &frame.display().to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("falha ao executar ffmpeg")?;

    if !status.success() {
        bail!("ffmpeg falhou ao extrair frame");
    }

    if !frame.exists() {
        bail!("frame não foi gerado");
    }

    println!("  frame extraído: {}", frame.display());
    Ok(())
}

fn send_telegram(args: &Args, photo: &Path) -> Result<()> {
    let url = format!(
        "https://api.telegram.org/bot{}/sendPhoto",
        args.bot_token
    );

    let form = multipart::Form::new()
        .text("chat_id", args.chat_id.clone())
        .text("caption", args.caption.clone())
        .file("photo", photo)
        .context("abrir arquivo da foto para upload")?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    let resp = client
        .post(&url)
        .multipart(form)
        .send()
        .context("requisição ao Telegram")?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();

    if !status.is_success() {
        bail!("Telegram retornou {}: {}", status, body);
    }

    // Resposta típica: {"ok":true,...}
    if !body.contains("\"ok\":true") {
        bail!("resposta inesperada do Telegram: {}", body);
    }

    Ok(())
}
