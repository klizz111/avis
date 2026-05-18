#![allow(non_snake_case)]

use axum::{extract::State, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::env;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::cors::{Any, CorsLayer};

use ark_bn254::{g1::G1Affine, Fr as Scalar, G1Projective, G2Affine, G2Projective};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{One, Zero};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use math_core::bls::{sign as bls_sign, verify as bls_verify};
use math_core::dkg::{global_public_key_from_commitments, lagrange_coeff_at_zero, simulate_dkg, verify_share};
use math_core::schnorr::{schnorr_prove, schnorr_verify};
use rand::thread_rng;

const DEFAULT_DEMO_USER_SEED: u64 = 7;

#[derive(Clone)]
struct AppState {
    node_id: String,
    port: u16,
    started_at_unix: u64,
    cluster: Arc<Mutex<MockCluster>>,
}

#[derive(Clone, Serialize)]
struct MockNode {
    node_id: String,
    port: u16,
    status: String,
}

#[derive(Clone)]
struct MockRoundState {
    round_id: String,
    threshold: u32,
    participants: Vec<String>,
    user_index: usize,
    user_seed: u64,
    commitments: Vec<Vec<G1Affine>>,
    shares: Vec<Vec<Scalar>>,
    final_shares: Vec<Scalar>,
    public_shares: Vec<G1Affine>,
    public_key: G1Affine,
}

#[derive(Clone)]
struct MockCluster {
    nodes: Vec<MockNode>,
    rounds: HashMap<String, MockRoundState>,
    latest_round_id: Option<String>,
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
    R: String,
    s: String,
    pk_share: String,
    nonce: u64,
    ts: u64,
}

#[derive(Deserialize)]
struct VerifyProofRequest {
    message: String,
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
    #[serde(default)]
    round_id: Option<String>,
    message: String,
    share: String,
    proof: ProofShareDTO,
}

#[derive(Deserialize)]
struct AggregateRequest {
    #[serde(default)]
    round_id: Option<String>,
    message: String,
    partial_signatures: Vec<String>,
}

#[derive(Deserialize)]
struct DemoRunRequest {
    #[serde(default)]
    round_id: Option<String>,
    message: String,
    proof: ProofShareDTO,
}

#[derive(Deserialize)]
struct DemoProofRequest {
    #[serde(default)]
    round_id: Option<String>,
    message: String,
    #[serde(default)]
    seed: Option<u64>,
    nonce: u64,
    timestamp: u64,
}

#[derive(Serialize)]
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

impl MockCluster {
    fn new(nodes: Vec<MockNode>) -> Self {
        Self {
            nodes,
            rounds: HashMap::new(),
            latest_round_id: None,
        }
    }

    fn insert_round(&mut self, round: MockRoundState) {
        self.latest_round_id = Some(round.round_id.clone());
        self.rounds.insert(round.round_id.clone(), round);
    }

    fn round(&self, round_id: Option<&str>) -> Option<&MockRoundState> {
        match round_id {
            Some(id) => self.rounds.get(id),
            None => self.latest_round_id.as_deref().and_then(|id| self.rounds.get(id)),
        }
    }
}

impl MockRoundState {
    fn build(round_id: String, threshold: u32, participants: Vec<String>) -> Result<Self, String> {
        let participant_count = participants.len();
        if participant_count == 0 {
            return Err("participants cannot be empty".to_string());
        }

        let threshold_usize = threshold as usize;
        if threshold_usize == 0 || threshold_usize > participant_count {
            return Err("invalid threshold/participants".to_string());
        }

        let (mut polys, mut shares, mut final_shares, _sk, mut commitments) = simulate_dkg(participant_count, threshold_usize);

        let user_index = 1usize;
        let user_seed = DEFAULT_DEMO_USER_SEED;
        let desired_user_share = Scalar::from(user_seed);
        let current_user_share = final_shares[user_index - 1];
        let delta = desired_user_share - current_user_share;

        polys[0][0] += delta;
        commitments[0][0] = (G1Affine::generator() * polys[0][0]).into_affine();
        for j in 0..participant_count {
            shares[0][j] += delta;
            final_shares[j] += delta;
        }

        let public_shares = (1..=participant_count as u64)
            .map(|index| derive_public_share(&commitments, index))
            .collect::<Vec<_>>();
        let public_key = global_public_key_from_commitments(&commitments);

        Ok(Self {
            round_id,
            threshold,
            participants,
            user_index,
            user_seed,
            commitments,
            shares,
            final_shares,
            public_shares,
            public_key,
        })
    }
}

fn configured_nodes() -> Vec<MockNode> {
    let node_names = env::var("MOCK_NODES")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "node-a,node-b,node-c,node-d".to_string());
    let base_port = env::var("MOCK_BASE_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8443);

    node_names
        .split(',')
        .enumerate()
        .map(|(index, node_id)| MockNode {
            node_id: node_id.trim().to_string(),
            port: base_port.saturating_add(index as u16),
            status: "mock-ready".to_string(),
        })
        .filter(|node| !node.node_id.is_empty())
        .collect()
}

fn encode_scalar_b64(value: Scalar) -> Result<String, String> {
    let mut buf = Vec::new();
    value
        .serialize_uncompressed(&mut buf)
        .map_err(|error| error.to_string())?;
    Ok(general_purpose::STANDARD.encode(buf))
}

fn encode_point_b64(value: G1Affine) -> Result<String, String> {
    let mut buf = Vec::new();
    value
        .serialize_uncompressed(&mut buf)
        .map_err(|error| error.to_string())?;
    Ok(general_purpose::STANDARD.encode(buf))
}

fn encode_g2_b64(sig: G2Affine) -> Result<String, String> {
    let mut buf = Vec::new();
    sig.serialize_uncompressed(&mut buf)
        .map_err(|e| e.to_string())?;
    Ok(general_purpose::STANDARD.encode(buf))
}

fn build_demo_proof_bundle(round: Option<&MockRoundState>, req: &DemoProofRequest) -> Result<DemoProofBundle, String> {
    let (share, pk_share, seed) = if let Some(round) = round {
        let share = round.final_shares[round.user_index - 1];
        let pk_share = round.public_shares[round.user_index - 1];
        (share, pk_share, round.user_seed)
    } else {
        let seed = req.seed.unwrap_or(DEFAULT_DEMO_USER_SEED);
        let share = Scalar::from(seed);
        let pk_share = (G1Affine::generator() * share).into_affine();
        (share, pk_share, seed)
    };

    let mut rng = thread_rng();
    let (r, s) = schnorr_prove(&mut rng, share, pk_share, req.message.as_bytes(), req.nonce, req.timestamp);

    Ok(DemoProofBundle {
        message_text: req.message.clone(),
        message_b64: general_purpose::STANDARD.encode(req.message.as_bytes()),
        share_b64: encode_scalar_b64(share)?,
        pk_share_b64: encode_point_b64(pk_share)?,
        r_b64: encode_point_b64(r)?,
        s_b64: encode_scalar_b64(s)?,
        seed,
        nonce: req.nonce,
        timestamp: req.timestamp,
    })
}

fn encode_point_vec_b64(points: &[G1Affine]) -> Result<Vec<String>, String> {
    points.iter().copied().map(encode_point_b64).collect::<Result<Vec<_>, _>>()
}

fn decode_point_to_affine_b64(s: &str) -> Result<G1Affine, String> {
    let bytes = general_purpose::STANDARD.decode(s).map_err(|e| e.to_string())?;
    let mut cur = Cursor::new(&bytes);
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

fn decode_g2_to_affine_b64(s: &str) -> Result<G2Affine, String> {
    let bytes = general_purpose::STANDARD.decode(s).map_err(|e| e.to_string())?;
    let mut cur = Cursor::new(&bytes);
    if let Ok(p) = G2Affine::deserialize_compressed(&mut cur) {
        return Ok(p);
    }
    let mut cur2 = Cursor::new(&bytes);
    G2Affine::deserialize_uncompressed(&mut cur2).map_err(|e| e.to_string())
}

fn derive_public_share(commitments: &[Vec<G1Affine>], receiver_index: u64) -> G1Affine {
    let receiver = Scalar::from(receiver_index);
    let mut acc = G1Projective::zero();

    for participant_commitments in commitments {
        let mut power = Scalar::one();
        for commitment in participant_commitments {
            let term = G1Projective::from(*commitment) * power;
            acc += &term;
            power *= &receiver;
        }
    }

    acc.into_affine()
}

fn round_demo_payload(round: &MockRoundState) -> Result<serde_json::Value, String> {
    let nodes = round
        .participants
        .iter()
        .enumerate()
        .map(|(index, node_id)| {
            Ok(json!({
                "node_id": node_id,
                "commitments_b64": encode_point_vec_b64(&round.commitments[index])?,
                "share_to_user_b64": encode_scalar_b64(round.shares[index][round.user_index - 1])?,
                "final_share_b64": encode_scalar_b64(round.final_shares[index])?,
                "public_share_b64": encode_point_b64(round.public_shares[index])?,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(json!({
        "round_id": round.round_id,
        "threshold": round.threshold,
        "participants": round.participants,
        "user_index": round.user_index,
        "user_seed": round.user_seed,
        "user_share_b64": encode_scalar_b64(round.final_shares[round.user_index - 1])?,
        "user_pk_share_b64": encode_point_b64(round.public_shares[round.user_index - 1])?,
        "threshold_public_key_b64": encode_point_b64(round.public_key)?,
        "nodes": nodes,
    }))
}

async fn healthz(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let cluster = state.cluster.lock().expect("cluster lock poisoned");
    Json(ApiResponse::success(json!({
        "service": "dkg-signer-api",
        "status": "ok",
        "node_id": state.node_id,
        "port": state.port,
        "started_at_unix": state.started_at_unix,
        "known_nodes": cluster.nodes,
        "latest_round_id": cluster.latest_round_id,
    })))
}

async fn list_nodes(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let cluster = state.cluster.lock().expect("cluster lock poisoned");
    Json(ApiResponse::success(json!({
        "nodes": cluster.nodes,
        "latest_round_id": cluster.latest_round_id,
    })))
}

async fn dkg_init(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DkgInitRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let participants = req.participants.len() as u32;
    if req.round_id.is_empty() {
        return Json(ApiResponse::fail("round_id cannot be empty"));
    }
    if req.threshold == 0 || participants == 0 || req.threshold > participants {
        return Json(ApiResponse::fail("invalid threshold/participants"));
    }

    let round = match MockRoundState::build(req.round_id.clone(), req.threshold, req.participants.clone()) {
        Ok(round) => round,
        Err(error) => return Json(ApiResponse::fail(error)),
    };

    let demo_round = match round_demo_payload(&round) {
        Ok(payload) => payload,
        Err(error) => return Json(ApiResponse::fail(error)),
    };

    {
        let mut cluster = state.cluster.lock().expect("cluster lock poisoned");
        cluster.insert_round(round);
    }

    Json(ApiResponse::success(json!({
        "status": "accepted",
        "round_id": req.round_id,
        "threshold": req.threshold,
        "participants": req.participants,
        "note": "mock round accepted and stored in memory",
        "demo_round": demo_round,
    })))
}

async fn dkg_submit_commitment(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CommitmentSubmitRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    if req.round_id.is_empty() || req.node_id.is_empty() {
        return Json(ApiResponse::fail("round_id/node_id cannot be empty"));
    }
    if req.commitments.is_empty() {
        return Json(ApiResponse::fail("commitments cannot be empty"));
    }

    let cluster = state.cluster.lock().expect("cluster lock poisoned");
    let round = match cluster.round(Some(&req.round_id)) {
        Some(round) => round,
        None => return Json(ApiResponse::fail("unknown round_id")),
    };

    let expected_index = match round.participants.iter().position(|participant| participant == &req.node_id) {
        Some(index) => index,
        None => return Json(ApiResponse::fail("unknown node_id for this round")),
    };

    let mut decoded = 0usize;
    for c in &req.commitments {
        if decode_point_to_affine_b64(c).is_err() {
            return Json(ApiResponse::fail("invalid commitment encoding"));
        }
        decoded += 1;
    }

    if decoded != round.commitments[expected_index].len() {
        return Json(ApiResponse::fail("commitment vector length does not match the stored demo round"));
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
    State(state): State<Arc<AppState>>,
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

    let cluster = state.cluster.lock().expect("cluster lock poisoned");
    let round = match cluster.round(Some(&req.round_id)) {
        Some(round) => round,
        None => return Json(ApiResponse::fail("unknown round_id")),
    };

    let sender_index = match round.participants.iter().position(|participant| participant == &req.from_node) {
        Some(index) => index,
        None => return Json(ApiResponse::fail("unknown from_node for this round")),
    };

    let receiver_index = (req.to_index - 1) as usize;
    if receiver_index >= round.participants.len() {
        return Json(ApiResponse::fail("to_index is out of range for this round"));
    }

    if commitments.len() != round.commitments[sender_index].len() {
        return Json(ApiResponse::fail("commitment vector length does not match the stored demo round"));
    }

    let expected_share = round.shares[sender_index][receiver_index];
    if share_scalar != expected_share {
        return Json(ApiResponse::fail("share does not match the stored demo round"));
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

async fn verify_proof(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyProofRequest>,
) -> Json<ApiResponse<VerifyProofData>> {
    let message = match general_purpose::STANDARD.decode(&req.message) {
        Ok(b) => b,
        Err(e) => return Json(ApiResponse::fail(format!("bad base64 message: {}", e))),
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

    let expected_pk = {
        let cluster = state.cluster.lock().expect("cluster lock poisoned");
        cluster.round(None).map(|round| round.public_shares[round.user_index - 1])
    };

    if let Some(expected_pk) = expected_pk {
        if pk_aff != expected_pk {
            return Json(ApiResponse::success(VerifyProofData {
                valid: false,
                reason: Some("proof public key does not match the active demo round".to_string()),
            }));
        }
    }

    let ok = schnorr_verify(pk_aff, R_aff, s_scalar, &message, req.proof.nonce, req.proof.ts);
    Json(ApiResponse::success(VerifyProofData {
        valid: ok,
        reason: if ok { None } else { Some("proof verification failed".to_string()) },
    }))
}

async fn demo_proof(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DemoProofRequest>,
) -> Json<ApiResponse<DemoProofBundle>> {
    let cluster = state.cluster.lock().expect("cluster lock poisoned");
    let round = cluster.round(req.round_id.as_deref());

    match build_demo_proof_bundle(round, &req) {
        Ok(bundle) => Json(ApiResponse::success(bundle)),
        Err(error) => Json(ApiResponse::fail(error)),
    }
}

async fn sign_partial(
    State(state): State<Arc<AppState>>,
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

    let proof_r = match decode_point_to_affine_b64(&req.proof.R) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("R decode: {}", e))),
    };

    let proof_s = match decode_scalar_b64(&req.proof.s) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("s decode: {}", e))),
    };

    let cluster = state.cluster.lock().expect("cluster lock poisoned");
    let round = cluster.round(req.round_id.as_deref());

    let proof_pk = if let Some(round) = round {
        let node_index = match round.participants.iter().position(|participant| participant == &req.node_id) {
            Some(index) => index,
            None => return Json(ApiResponse::fail("unknown node_id for this round")),
        };

        let expected_share = round.final_shares[node_index];
        if share_scalar != expected_share {
            return Json(ApiResponse::fail("share does not match the active demo round"));
        }

        round.public_shares[node_index]
    } else {
        (G1Affine::generator() * share_scalar).into_affine()
    };

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
    State(state): State<Arc<AppState>>,
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

    let cluster = state.cluster.lock().expect("cluster lock poisoned");
    let round = cluster.round(req.round_id.as_deref());

    let mut aggregate = G2Projective::from(G2Affine::identity());
    let mut decoded_count = 0usize;
    let indices: Vec<u64> = (1..=req.partial_signatures.len() as u64).collect();
    for partial in &req.partial_signatures {
        let sig = match decode_g2_to_affine_b64(partial) {
            Ok(v) => v,
            Err(e) => return Json(ApiResponse::fail(format!("partial signature decode: {}", e))),
        };
        let lambda = lagrange_coeff_at_zero(&indices, decoded_count);
        let term = G2Projective::from(sig) * lambda;
        aggregate += &term;
        decoded_count += 1;
    }

    let aggregate_affine = aggregate.into_affine();
    let signature = match encode_g2_b64(aggregate_affine) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("aggregate encode: {}", e))),
    };

    let (verified, threshold_public_key_b64) = match round {
        Some(round) => (
            bls_verify(&round.public_key, &message, &aggregate_affine),
            Some(encode_point_b64(round.public_key).unwrap_or_default()),
        ),
        None => (false, None),
    };

    Json(ApiResponse::success(json!({
        "status": "aggregated",
        "message_size": message.len(),
        "partial_count": decoded_count,
        "signature": signature,
        "verified": verified,
        "threshold_public_key_b64": threshold_public_key_b64,
    })))
}

async fn demo_run(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DemoRunRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let message = match general_purpose::STANDARD.decode(&req.message) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::fail(format!("bad base64 message: {}", e))),
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

    let cluster = state.cluster.lock().expect("cluster lock poisoned");
    let round = match cluster.round(req.round_id.as_deref()) {
        Some(round) => round,
        None => return Json(ApiResponse::fail("no active demo round available")),
    };

    let user_public_share = round.public_shares[round.user_index - 1];
    if proof_pk != user_public_share {
        return Json(ApiResponse::success(json!({
            "status": "rejected",
            "round_id": round.round_id,
            "proof_valid": false,
            "reason": "proof public key does not match the active demo round"
        })));
    }

    let proof_ok = schnorr_verify(
        proof_pk,
        proof_r,
        proof_s,
        &message,
        req.proof.nonce,
        req.proof.ts,
    );
    if !proof_ok {
        return Json(ApiResponse::success(json!({
            "status": "rejected",
            "round_id": round.round_id,
            "proof_valid": false,
            "reason": "schnorr proof verification failed"
        })));
    }

    let threshold = round.threshold as usize;
    let mut partial_signatures = Vec::with_capacity(threshold);
    let mut indices = Vec::with_capacity(threshold);
    for index in 0..threshold {
        partial_signatures.push(bls_sign(&round.final_shares[index], &message));
        indices.push((index + 1) as u64);
    }

    let mut aggregate = G2Projective::from(G2Affine::identity());
    for (index, signature) in partial_signatures.iter().enumerate() {
        let lambda = lagrange_coeff_at_zero(&indices, index);
        let term = G2Projective::from(*signature) * lambda;
        aggregate += &term;
    }

    let aggregate_affine = aggregate.into_affine();
    let aggregated_signature = match encode_g2_b64(aggregate_affine) {
        Ok(value) => value,
        Err(error) => return Json(ApiResponse::fail(format!("aggregate encode: {}", error))),
    };

    let partial_signatures_b64 = partial_signatures
        .into_iter()
        .map(encode_g2_b64)
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    let verified = bls_verify(&round.public_key, &message, &aggregate_affine);

    Json(ApiResponse::success(json!({
        "status": "completed",
        "round_id": round.round_id,
        "threshold": round.threshold,
        "partial_count": partial_signatures_b64.len(),
        "partial_signatures": partial_signatures_b64,
        "signature": aggregated_signature,
        "verified": verified,
        "threshold_public_key_b64": encode_point_b64(round.public_key).unwrap_or_default(),
        "proof_valid": true,
        "user_seed": round.user_seed,
    })))
}

#[tokio::main]
async fn main() {
    let node_id = env::var("MOCK_NODE_ID").unwrap_or_else(|_| "node-local-1".to_string());
    let port = env::var("MOCK_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8443);

    let state = Arc::new(AppState {
        node_id,
        port,
        started_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        cluster: Arc::new(Mutex::new(MockCluster::new(configured_nodes()))),
    });

    let api_v1 = Router::new()
        .route("/healthz", get(healthz))
        .route("/nodes", get(list_nodes))
        .route("/dkg/init", post(dkg_init))
        .route("/dkg/commitment/submit", post(dkg_submit_commitment))
        .route("/dkg/share/submit", post(dkg_submit_share))
        .route("/proof/verify", post(verify_proof))
        .route("/demo/proof", post(demo_proof))
        .route("/demo/run", post(demo_run))
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

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
