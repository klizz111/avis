//! BLS operations on the `bn254` curve.

use ark_bn254::{Bn254, Fr as Scalar, G1Affine as G1, G2Affine as G2};
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_ff::{One, PrimeField, Zero};
use ark_std::{rand::Rng, UniformRand};

/// Generate a BLS secret key and public key pair.
pub fn key_gen<R: Rng>(rng: &mut R) -> (Scalar, G1) {
    let secret_key = Scalar::rand(rng);
    let public_key = G1::generator() * secret_key;
    (secret_key, public_key.into_affine())
}

/// Deterministically map a message into a scalar field element.
fn hash_to_scalar(message: &[u8]) -> Scalar {
    let mut scalar = Scalar::from_le_bytes_mod_order(message);

    if scalar.is_zero() {
        scalar = Scalar::one();
    }

    scalar
}

/// Hash a message to the G2 subgroup by multiplying the generator with a field element.
pub fn hash_to_g2(message: &[u8]) -> G2 {
    (G2::generator() * hash_to_scalar(message)).into_affine()
}

/// Sign a message with the secret key.
pub fn sign(secret_key: &Scalar, message: &[u8]) -> G2 {
    (hash_to_g2(message) * secret_key).into_affine()
}

/// Verify a BLS signature using pairing checks.
pub fn verify(public_key: &G1, message: &[u8], signature: &G2) -> bool {
    let generator = G1::generator();
    let hashed_message = hash_to_g2(message);

    Bn254::pairing(*public_key, hashed_message) == Bn254::pairing(generator, *signature)
}