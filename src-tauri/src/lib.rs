// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use ark_bn254::{Fr as Scalar, G1Affine};
use ark_ec::{AffineRepr, CurveGroup};
use ark_serialize::CanonicalSerialize;
use base64::{engine::general_purpose, Engine as _};
use math_core::schnorr::schnorr_prove;
use rand::thread_rng;

#[derive(serde::Serialize)]
struct DemoProofBundle {
    message_text: String,
    message_b64: String,
    share_b64: String,
    pk_share_b64: String,
    r_b64: String,
    s_b64: String,
    seed: u64,
    nonce: u64,
    timestamp: u64,
}

fn encode_scalar_b64(value: Scalar) -> Result<String, String> {
    let mut buffer = Vec::new();
    value
        .serialize_uncompressed(&mut buffer)
        .map_err(|error| error.to_string())?;
    Ok(general_purpose::STANDARD.encode(buffer))
}

fn encode_point_b64(value: G1Affine) -> Result<String, String> {
    let mut buffer = Vec::new();
    value
        .serialize_uncompressed(&mut buffer)
        .map_err(|error| error.to_string())?;
    Ok(general_purpose::STANDARD.encode(buffer))
}

#[tauri::command]
fn generate_demo_proof(message: String, seed: u64, nonce: u64, timestamp: u64) -> Result<DemoProofBundle, String> {
    let share = Scalar::from(seed);
    let pk_share = (G1Affine::generator() * share).into_affine();
    let mut rng = thread_rng();
    let (r, s) = schnorr_prove(&mut rng, share, pk_share, message.as_bytes(), nonce, timestamp);

    Ok(DemoProofBundle {
        message_text: message.clone(),
        message_b64: general_purpose::STANDARD.encode(message.as_bytes()),
        share_b64: encode_scalar_b64(share)?,
        pk_share_b64: encode_point_b64(pk_share)?,
        r_b64: encode_point_b64(r)?,
        s_b64: encode_scalar_b64(s)?,
        seed,
        nonce,
        timestamp,
    })
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn add_numbers(a: f64, b: f64) -> f64 {
    a + b
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_macos_permissions::init())
        .invoke_handler(tauri::generate_handler![greet, add_numbers, generate_demo_proof])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
