use ark_bn254::Fr as Scalar;
use math_core::dkg::{lagrange_coeff_at_zero, reconstruct_from_shares, simulate_dkg, hash_to_field};
use ark_ff::Zero;

#[test]
fn dkg_reconstructs_secret() {
    let n = 5;
    let t = 3;
    let (_, _shares, s_values, sk, _commitments) = simulate_dkg(n, t);
    let indices: Vec<u64> = (1..=t as u64).collect();
    let shares: Vec<Scalar> = (0..t).map(|i| s_values[i]).collect();
    let rec = reconstruct_from_shares(&indices, &shares);
    assert_eq!(rec, sk);
}

#[test]
fn threshold_signing_matches_full_key() {
    let n = 5;
    let t = 3;
    let (_polys, _shares, s_values, sk, _commitments) = simulate_dkg(n, t);
    let m = b"hello";
    let hm = hash_to_field(m);

    let mut part = Vec::new();
    let mut indices = Vec::new();
    for i in 0..t {
        part.push(s_values[i] * &hm);
        indices.push((i + 1) as u64);
    }

    let mut agg = Scalar::zero();
    for j in 0..t {
        let lambda = lagrange_coeff_at_zero(&indices, j);
        agg += &(lambda * &part[j]);
    }

    let expected = sk * &hm;
    assert_eq!(agg, expected);
}
