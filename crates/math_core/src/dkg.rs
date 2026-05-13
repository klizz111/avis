#![allow(non_snake_case)]
use ark_bn254::{Fr as Scalar, G1Affine, G1Projective};
use ark_ff::{Field, One, Zero};
use ark_ec::{CurveGroup, AffineRepr};

/// Deterministic polynomial generator for demo purposes.
/// coeffs[k] corresponds to a_{k} for degree t-1.
pub fn generate_poly(id: u64, degree: usize) -> Vec<Scalar> {
    (0..=degree)
        .map(|k| Scalar::from((id + 1) * (k as u64 + 3)))
        .collect()
}

/// Evaluate polynomial (coefficients in ascending order) at integer x.
pub fn eval_poly_at(coeffs: &[Scalar], x: u64) -> Scalar {
    let mut res = Scalar::zero();
    let xv: Scalar = x.into();
    // Horner from highest degree
    for a in coeffs.iter().rev() {
        res *= &xv;
        res += a;
    }
    res
}

/// Simulate a simple DKG: n participants, threshold t (degree = t-1)
/// Returns (coefficients per participant, shares matrix, final_shares S_j, SK)
pub fn simulate_dkg(n: usize, t: usize) -> (Vec<Vec<Scalar>>, Vec<Vec<Scalar>>, Vec<Scalar>, Scalar, Vec<Vec<G1Affine>>) {
    let degree = t - 1;
    // each participant generates a polynomial
    let mut polys: Vec<Vec<Scalar>> = Vec::with_capacity(n);
    for i in 0..n {
        polys.push(generate_poly(i as u64, degree));
    }

    // compute commitments for each participant (Feldman VSS): C_{i,k} = [a_{i,k}]G
    let mut commitments: Vec<Vec<G1Affine>> = vec![vec![G1Projective::zero().into_affine(); degree+1]; n];
    // get affine generator
    let g_aff = G1Affine::generator();
    for i in 0..n {
        for k in 0..=degree {
            // multiply affine generator by scalar -> projective, then convert to affine for storage
            commitments[i][k] = (g_aff * polys[i][k]).into_affine();
        }
    }

    // compute shares s_{i->j} where participants/indexing use 1..=n as x
    let mut shares = vec![vec![Scalar::zero(); n]; n];
    for i in 0..n {
        for j in 0..n {
            shares[i][j] = eval_poly_at(&polys[i], (j + 1) as u64);
        }
    }

    // each participant j computes S_j = sum_i s_{i->j}
    let mut S = vec![Scalar::zero(); n];
    for j in 0..n {
        let mut sum = Scalar::zero();
        for i in 0..n {
            sum += &shares[i][j];
        }
        S[j] = sum;
    }

    // global secret SK = sum_i a_{i,0}
    let mut sk = Scalar::zero();
    for i in 0..n {
        sk += &polys[i][0];
    }

    (polys, shares, S, sk, commitments)
}

/// Compute Lagrange coefficient for x = 0 using given x_values (1-based) and target index j (index into x_values)
pub fn lagrange_coeff_at_zero(x_values: &[u64], j: usize) -> Scalar {
    let xj = Scalar::from(x_values[j]);
    let mut num = Scalar::one();
    let mut den = Scalar::one();
    for (m, xm_u) in x_values.iter().enumerate() {
        if m == j { continue; }
        let xm = Scalar::from(*xm_u);
        num *= &(-xm);
        den *= &(xj - xm);
    }
    num * den.inverse().unwrap()
}

/// Reconstruct secret at 0 from t shares given their indices (1-based)
pub fn reconstruct_from_shares(indices: &[u64], shares: &[Scalar]) -> Scalar {
    assert_eq!(indices.len(), shares.len());
    let x_values: Vec<u64> = indices.to_vec();
    let mut acc = Scalar::zero();
    for j in 0..shares.len() {
        let lambda = lagrange_coeff_at_zero(&x_values, j);
        acc += &(lambda * &shares[j]);
    }
    acc
}

/// Commitments-based verification: given commitments C_k = [a_k]G, verify that s = f(j)
/// by checking [s]G == sum_k j^k * C_k (additive group)
pub fn verify_share(commitments: &[G1Affine], share: Scalar, j: u64) -> bool {
    // compute [share]G and compare to sum_k j^k * C_k
    let g_aff = G1Affine::generator();
    let left = g_aff * share; // projective

    let mut rhs = G1Projective::zero();
    let mut jpow = Scalar::one();
    let j_scalar = Scalar::from(j);
    for Ck in commitments.iter() {
        let term = (*Ck) * jpow; // projective
        rhs += &term;
        jpow *= &j_scalar;
    }

    left == rhs
}

/// Simple hash-to-field: map a message bytes to a Scalar by folding bytes into u64 then into field.
pub fn hash_to_field(msg: &[u8]) -> Scalar {
    use blake2::{Blake2s256, Digest};
    let mut hasher = Blake2s256::new();
    hasher.update(msg);
    let out = hasher.finalize();
    // take first 8 bytes as u64 little endian
    let mut b = [0u8; 8];
    b.copy_from_slice(&out[0..8]);
    let v = u64::from_le_bytes(b);
    Scalar::from(v)
}
