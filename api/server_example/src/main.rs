#![allow(non_snake_case)]
use axum::{extract::State, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use base64::{engine::general_purpose, Engine as _};
use std::net::SocketAddr;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::cors::{Any, CorsLayer};

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_bn254::{g1::G1Affine, Fr as Scalar, G2Affine, G2Projective};
use ark_ec::{AffineRepr, CurveGroup};
use math_core::bls::sign as bls_sign;
use math_core::dkg::verify_share;
use math_core::schnorr::schnorr_verify;

#[derive(Clone)]
struct AppState {
    node_id: String,
    started_at_unix: u64,
}

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    fn fail(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

#[derive(Deserialize)]
struct ProofShareDTO {
    R: String,        // base64 of projective or affine serialization
    s: String,        // base64 of scalar
    pk_share: String, // base64 of affine
    nonce: u64,
    ts: u64,
    // optional: encoding field
}

#[derive(Deserialize)]
struct VerifyProofRequest {
    message: String, // base64-encoded message bytes
    proof: ProofShareDTO,
}

#[derive(Serialize)]
struct VerifyProofData {
    valid: bool,
    reason: Option<String>,
}

#[derive(Deserialize)]
struct DkgInitRequest {
    round_id: String,
    threshold: u32,
    participants: Vec<String>,
}

#[derive(Deserialize)]
struct CommitmentSubmitRequest {
    round_id: String,
    node_id: String,
    commitments: Vec<String>,
}

#[derive(Deserialize)]
struct ShareSubmitRequest {
    round_id: String,
    from_node: String,
    to_node: String,
    to_index: u64,
    share: String,
    commitments: Vec<String>,
}

#[derive(Deserialize)]
struct PartialSignRequest {
    node_id: String,
    message: String, // base64-encoded message bytes
    share: String,   // base64-encoded scalar bytes for demo-only local share
    proof: ProofShareDTO,
}

#[derive(Deserialize)]
struct AggregateRequest {
    message: String,
    partial_signatures: Vec<String>,
}

// Decode a point provided as affine or projective bytes into an affine point.
fn decode_point_to_affine_b64(s: &str) -> Result<G1Affine, String> {
    let bytes = general_purpose::STANDARD.decode(s).map_err(|e| e.to_string())?;
    let mut cur = Cursor::new(&bytes);
    // try affine compressed
    if let Ok(p) = G1Affine::deserialize_compressed(&mut cur) {
        return Ok(p);
    }
    let mut cur2 = Cursor::new(&bytes);
    if let Ok(p) = G1Affine::deserialize_uncompressed(&mut cur2) {
        return Ok(p);
    }
    Err("failed to decode point as affine compressed or uncompressed".to_string())
}

fn decode_scalar_b64(s: &str) -> Result<Scalar, String> {
    let bytes = general_purpose::STANDARD.decode(s).map_err(|e| e.to_string())?;
    let mut cur = Cursor::new(&bytes);
    Scalar::deserialize_uncompressed(&mut cur).map_err(|e| e.to_string())
}

fn encode_g2_b64(sig: G2Affine) -> Result<String, String> {
    let mut buf = Vec::new();
    sig.serialize_uncompressed(&mut buf)
        .map_err(|e| e.to_string())?;
    Ok(general_purpose::STANDARD.encode(buf))
}

fn decode_g2_to_affine_b64(s: &str) -> Result<G2Affine, String> {
    let bytes = general_purpose::STANDARD.decode(s).map_err(|e| e.to_string())?;
    let mut cur = Cursor::new(&bytes);
    if let Ok(p) = G2Affine::deserialize_compressed(&mut cur) {
        return Ok(p);
    }
    let mut cur2 = Cursor::new(&bytes);
    G2Affine::deserialize_uncompressed(&mut cur2).map_err(|e| e.to_string())
}

async fn verify_proof(Json(req): Json<VerifyProofRequest>) -> Json<ApiResponse<VerifyProofData>> {
    // decode fields
    let message = match general_purpose::STANDARD.decode(&req.message) {
        Ok(b) => b,
        Err(e) => {
            return Json(ApiResponse::fail(format!("bad base64 message: {}", e)));
        }
    };

    let pk_aff = match decode_point_to_affine_b64(&req.proof.pk_share) {
        Ok(p) => p,
        Err(e) => return Json(ApiResponse::fail(format!("pk decode: {}", e))),
    };

    let R_aff = match decode_point_to_affine_b64(&req.proof.R) {
        Ok(r) => r,
        Err(e) => return Json(ApiResponse::fail(format!("R decode: {}", e))),
    };

    let s_scalar = match decode_scalar_b64(&req.proof.s) {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::fail(format!("s decode: {}", e))),
    };

    let ok = schnorr_verify(pk_aff, R_aff, s_scalar, &message, req.proof.nonce, req.proof.ts);
    Json(ApiResponse::success(VerifyProofData {
        valid: ok,
        reason: if ok { None } else { Some("proof verification failed".to_string()) },
    }))
}

async fn healthz(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(json!({
        "service": "dkg-signer-api",
        "status": "ok",
        "node_id": state.node_id,
        "started_at_unix": state.started_at_unix,
    })))
}

async fn list_nodes(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(json!({
        "nodes": [
            {
                "node_id": state.node_id,
                "status": "local"
            }
        ]
    })))
}

async fn dkg_init(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<DkgInitRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let participants = req.participants.len() as u32;
    if req.round_id.is_empty() {
        return Json(ApiResponse::fail("round_id cannot be empty"));
    }
    if req.threshold == 0 || participants == 0 || req.threshold > participants {
        return Json(ApiResponse::fail("invalid threshold/participants"));
    }

    Json(ApiResponse::success(json!({
        "status": "accepted",
        "round_id": req.round_id,
        "threshold": req.threshold,
        "participants": req.participants,
        "note": "skeleton endpoint: wire real DKG coordinator next"
    })))
}

async fn dkg_submit_commitment(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CommitmentSubmitRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    if req.round_id.is_empty() || req.node_id.is_empty() {
        return Json(ApiResponse::fail("round_id/node_id cannot be empty"));
    }
    if req.commitments.is_empty() {
        return Json(ApiResponse::fail("commitments cannot be empty"));
    }

    let mut decoded = 0usize;
    for c in &req.commitments {
        if decode_point_to_affine_b64(c).is_err() {
            return Json(ApiResponse::fail("invalid commitment encoding"));
        }
        decoded += 1;
    }

    Json(ApiResponse::success(json!({
        "status": "received",
        "round_id": req.round_id,
        "node_id": req.node_id,
        "commitments_count": decoded,
        "note": "commitments decoded and accepted"
    })))
}

async fn dkg_submit_share(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ShareSubmitRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    if req.round_id.is_empty() || req.from_node.is_empty() || req.to_node.is_empty() {
        return Json(ApiResponse::fail("round_id/from_node/to_node cannot be empty"));
    }
    if req.to_index == 0 {
        return Json(ApiResponse::fail("to_index must be >= 1"));
    }
    if req.share.is_empty() {
        return Json(ApiResponse::fail("share cannot be empty"));
    }
    if req.commitments.is_empty() {
        return Json(ApiResponse::fail("commitments cannot be empty"));
    }

    let share_scalar = match decode_scalar_b64(&req.share) {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::fail(format!("share decode: {}", e))),
    };

    let mut commitments = Vec::with_capacity(req.commitments.len());
    for c in &req.commitments {
        let p = match decode_point_to_affine_b64(c) {
            Ok(v) => v,
            Err(e) => return Json(ApiResponse::fail(format!("commitment decode: {}", e))),
        };
        commitments.push(p);
    }

    let valid = verify_share(&commitments, share_scalar, req.to_index);

    Json(ApiResponse::success(json!({
        "status": "received",
        "round_id": req.round_id,
        "from": req.from_node,
        "to": req.to_node,
        "to_index": req.to_index,
        "commitments_count": req.commitments.len(),
        "vss_share_valid": valid
    })))
}

async fn sign_partial(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<PartialSignRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    if req.node_id.is_empty() || req.message.is_empty() || req.share.is_empty() {
        return Json(ApiResponse::fail("node_id/message/share cannot be empty"));
    }

    let message = match general_purpose::STANDARD.decode(&req.message) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("bad base64 message: {}", e))),
    };

    let share_scalar = match decode_scalar_b64(&req.share) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("share decode: {}", e))),
    };

    let proof_pk = match decode_point_to_affine_b64(&req.proof.pk_share) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("pk decode: {}", e))),
    };

    let proof_r = match decode_point_to_affine_b64(&req.proof.R) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("R decode: {}", e))),
    };

    let proof_s = match decode_scalar_b64(&req.proof.s) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("s decode: {}", e))),
    };

    // Bind the proof public key to the claimed share.
    let derived_pk = (G1Affine::generator() * share_scalar).into_affine();
    if proof_pk != derived_pk {
        return Json(ApiResponse::fail("pk_share does not match provided share"));
    }

    let proof_ok = schnorr_verify(
        derived_pk,
        proof_r,
        proof_s,
        &message,
        req.proof.nonce,
        req.proof.ts,
    );
    if !proof_ok {
        return Json(ApiResponse::fail("schnorr proof verification failed"));
    }

    let sigma = bls_sign(&share_scalar, &message);
    let sigma_b64 = match encode_g2_b64(sigma) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("sigma encode: {}", e))),
    };

    Json(ApiResponse::success(json!({
        "status": "issued",
        "node_id": req.node_id,
        "sigma": sigma_b64,
        "proof_valid": true
    })))
}

async fn bls_aggregate(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<AggregateRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    if req.message.is_empty() {
        return Json(ApiResponse::fail("message cannot be empty"));
    }
    if req.partial_signatures.is_empty() {
        return Json(ApiResponse::fail("partial_signatures cannot be empty"));
    }

    let message = match general_purpose::STANDARD.decode(&req.message) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("bad base64 message: {}", e))),
    };

    let mut aggregate = G2Projective::from(G2Affine::identity());
    let mut decoded_count = 0usize;
    for partial in &req.partial_signatures {
        let sig = match decode_g2_to_affine_b64(partial) {
            Ok(v) => v,
            Err(e) => return Json(ApiResponse::fail(format!("partial signature decode: {}", e))),
        };
        aggregate += G2Projective::from(sig);
        decoded_count += 1;
    }

    let aggregate_affine = aggregate.into_affine();
    let signature = match encode_g2_b64(aggregate_affine) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("aggregate encode: {}", e))),
    };

    Json(ApiResponse::success(json!({
        "status": "aggregated",
        "message_size": message.len(),
        "partial_count": decoded_count,
        "signature": signature
    })))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        node_id: "node-local-1".to_string(),
        started_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });

    let api_v1 = Router::new()
        .route("/healthz", get(healthz))
        .route("/nodes", get(list_nodes))
        .route("/dkg/init", post(dkg_init))
        .route("/dkg/commitment/submit", post(dkg_submit_commitment))
        .route("/dkg/share/submit", post(dkg_submit_share))
        .route("/proof/verify", post(verify_proof))
        .route("/sign/partial", post(sign_partial))
        .route("/bls/aggregate", post(bls_aggregate))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let app = Router::new().nest("/api/v1", api_v1);

    let addr = SocketAddr::from(([127,0,0,1], 8443));
    println!("listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
