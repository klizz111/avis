use ark_bn254::{Fr as Scalar, G1Affine, G1Projective};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{One, Zero};
use ark_std::test_rng;
use math_core::bls::{sign, verify};
use math_core::dkg::{hash_to_field, lagrange_coeff_at_zero, simulate_dkg};
use math_core::schnorr::{schnorr_prove, schnorr_verify};

#[test]
fn full_protocol_demo() {
    let mut rng = test_rng();
    let n = 5;
    let t = 3;

    let (_polys, _shares, s_values, sk, commitments) = simulate_dkg(n, t);

    let user_idx = 1u64;
    let user_share = s_values[(user_idx - 1) as usize];

    let mut pk_share_proj = G1Projective::zero();
    for i in 0..n {
        let mut jpow = Scalar::one();
        let j_scalar = Scalar::from(user_idx);
        for k in 0..commitments[i].len() {
            let term_proj = commitments[i][k] * jpow;
            pk_share_proj += &term_proj;
            jpow *= &j_scalar;
        }
    }
    let pk_share_aff: G1Affine = pk_share_proj.into_affine();

    let message = b"message to sign";
    let nonce = 42u64;
    let ts = 123456u64;
    let (r, s) = schnorr_prove(&mut rng, user_share, pk_share_aff, message, nonce, ts);
    assert!(schnorr_verify(pk_share_aff, r, s, message, nonce, ts));

    let hm = hash_to_field(message);
    let mut partial_sigs: Vec<Scalar> = Vec::new();
    let mut indices: Vec<u64> = Vec::new();
    for j in 0..t {
        partial_sigs.push(s_values[j] * &hm);
        indices.push((j + 1) as u64);
    }

    let mut agg = Scalar::zero();
    for j in 0..t {
        let lambda = lagrange_coeff_at_zero(&indices, j);
        agg += &(lambda * &partial_sigs[j]);
    }

    let expected = sk * &hm;
    assert_eq!(agg, expected);

    let bls_public_key = (G1Affine::generator() * sk).into_affine();
    let bls_signature = sign(&sk, message);
    assert!(verify(&bls_public_key, message, &bls_signature));
}
