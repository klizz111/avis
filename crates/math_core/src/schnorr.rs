#![allow(non_snake_case)]
use ark_bn254::{Fr as Scalar, G1Affine, G1Projective};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::UniformRand;
use ark_serialize::CanonicalSerialize;
use ark_std::rand::Rng;
use blake2::{Blake2s256, Digest};

// Schnorr proof over group G1 proving knowledge of `share` such that PK_share = g^share
pub fn schnorr_prove<R: Rng>(rng: &mut R, share: Scalar, pk_share: G1Affine, message: &[u8], nonce: u64, ts: u64) -> (G1Affine, Scalar) {
    let k = Scalar::rand(rng);
    let g = G1Affine::generator();
    let R_proj = g * k; // projective
    let R = R_proj.into_affine();

    // serialize pk_share and R (affine) to bytes
    let mut buf = Vec::new();
    pk_share.serialize_uncompressed(&mut buf).unwrap();
    R.serialize_uncompressed(&mut buf).unwrap();
    buf.extend_from_slice(message);
    buf.extend_from_slice(&nonce.to_le_bytes());
    buf.extend_from_slice(&ts.to_le_bytes());
    let mut hasher = Blake2s256::new();
    hasher.update(&buf);
    let c_bytes = hasher.finalize();
    let mut b = [0u8; 8];
    b.copy_from_slice(&c_bytes[0..8]);
    let c = Scalar::from(u64::from_le_bytes(b));

    let s = k + &(c * &share);
    (R, s)
}

pub fn schnorr_verify(pk_share: G1Affine, R: G1Affine, s: Scalar, message: &[u8], nonce: u64, ts: u64) -> bool {
    // recompute c
    let mut buf = Vec::new();
    pk_share.serialize_uncompressed(&mut buf).unwrap();
    R.serialize_uncompressed(&mut buf).unwrap();
    buf.extend_from_slice(message);
    buf.extend_from_slice(&nonce.to_le_bytes());
    buf.extend_from_slice(&ts.to_le_bytes());
    let mut hasher = Blake2s256::new();
    hasher.update(&buf);
    let c_bytes = hasher.finalize();
    let mut b = [0u8; 8];
    b.copy_from_slice(&c_bytes[0..8]);
    let c = Scalar::from(u64::from_le_bytes(b));

    let g = G1Affine::generator();
    // check s*G == R + c*PK
    let left = g * s; // projective
    let R_proj = G1Projective::from(R);
    let right = R_proj + (pk_share * c);
    left == right
}
