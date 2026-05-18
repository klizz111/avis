use ark_bn254::{Fr as Scalar, G1Affine, G1Projective};
use ark_ec::CurveGroup;
use ark_ff::{One, Zero};
use ark_std::test_rng;
use math_core::bls::{sign, verify};
use math_core::dkg::{global_public_key_from_commitments, lagrange_coeff_at_zero, simulate_dkg, verify_share};
use math_core::schnorr::{schnorr_prove, schnorr_verify};

#[test]
fn full_protocol_demo() {
    let mut rng = test_rng();
    let n = 5;
    let t = 3;

    // `shares` is the matrix of Feldman VSS sub-shares s_{i->j}.
    // `sk_user` below is only the user's final threshold share S_user, not a full private key.
    let (_polys, shares, final_shares, _sk_global, commitments) = simulate_dkg(n, t);
    let threshold_public_key = global_public_key_from_commitments(&commitments);

    let user_index = 1u64;
    let sk_user = final_shares[(user_index - 1) as usize];

    // Rebuild the user's public share key from all participants' commitments.
    let mut pk_share_proj = G1Projective::zero();
    for i in 0..n {
        let mut jpow = Scalar::one();
        let j_scalar = Scalar::from(user_index);
        for k in 0..commitments[i].len() {
            let term_proj = commitments[i][k] * jpow;
            pk_share_proj += &term_proj;
            jpow *= &j_scalar;
        }
    }
    let pk_user: G1Affine = pk_share_proj.into_affine();

    // Verify each participant's sub-share against its own commitments.
    for i in 0..n {
        assert!(verify_share(&commitments[i], shares[i][user_index as usize - 1], user_index));
    }

    let message = b"message to sign";
    let nonce = 42u64;
    let ts = 123456u64;
    // Schnorr proves control of the user's threshold secret share, paired with the user's share public key.
    let (r, s) = schnorr_prove(&mut rng, sk_user, pk_user, message, nonce, ts);
    assert!(schnorr_verify(pk_user, r, s, message, nonce, ts));

    // Collect t partial BLS signatures and aggregate them with Lagrange coefficients.
    let mut partial_sigs = Vec::new();
    let mut indices: Vec<u64> = Vec::new();
    for j in 0..t {
        partial_sigs.push(sign(&final_shares[j], message));
        indices.push((j + 1) as u64);
    }

    let mut agg = ark_bn254::G2Projective::zero();
    for j in 0..t {
        let lambda = lagrange_coeff_at_zero(&indices, j);
        agg += &(partial_sigs[j] * lambda);
    }

    assert!(verify(&threshold_public_key, message, &agg.into_affine()));
}
