use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use ark_bn254::{Fq as PolyScalar, Fr as Scalar, G1Affine, G2Affine, G2Projective};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{UniformRand, Zero};
use ark_std::rand::{rngs::StdRng, SeedableRng};
use math_core::{bls, dkg, poly, schnorr};

fn seeded_rng(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

fn sample_poly_coefficients(degree: usize) -> Vec<PolyScalar> {
    (0..=degree)
        .map(|index| PolyScalar::from((index as u64 + 1) * 17))
        .collect()
}

fn bench_poly_create(c: &mut Criterion) {
    c.bench_function("poly/create", |b| {
        let coefficients = sample_poly_coefficients(31);
        b.iter_batched(
            || coefficients.clone(),
            |coeffs| {
                let polynomial = poly::create_poly(black_box(coeffs));
                black_box(polynomial);
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_poly_evaluate(c: &mut Criterion) {
    c.bench_function("poly/evaluate", |b| {
        let polynomial = poly::create_poly(sample_poly_coefficients(31));
        let point = PolyScalar::from(19u64);

        b.iter(|| {
            let value = poly::evaluate_poly(black_box(&polynomial), black_box(point));
            black_box(value);
        });
    });
}

fn bench_dkg_simulate(c: &mut Criterion) {
    c.bench_function("dkg/simulate", |b| {
        b.iter(|| {
            let result = dkg::simulate_dkg(black_box(6), black_box(3));
            black_box(result);
        });
    });
}

fn bench_dkg_verify_share(c: &mut Criterion) {
    c.bench_function("dkg/verify_share", |b| {
        let (_, shares, _, _, commitments) = dkg::simulate_dkg(6, 3);
        let participant_index = 2usize;
        let share = shares[0][participant_index];
        let participant_commitments = commitments[0].clone();
        let participant_x = (participant_index + 1) as u64;

        b.iter(|| {
            let verified = dkg::verify_share(
                black_box(&participant_commitments),
                black_box(share),
                black_box(participant_x),
            );
            black_box(verified);
        });
    });
}

fn bench_dkg_reconstruct(c: &mut Criterion) {
    c.bench_function("dkg/reconstruct", |b| {
        let (_, _, final_shares, _, _) = dkg::simulate_dkg(6, 3);
        let indices = [1u64, 2, 3];
        let shares = [final_shares[0], final_shares[1], final_shares[2]];

        b.iter(|| {
            let secret = dkg::reconstruct_from_shares(black_box(&indices), black_box(&shares));
            black_box(secret);
        });
    });
}

fn bench_bls_key_gen(c: &mut Criterion) {
    c.bench_function("bls/key_gen", |b| {
        b.iter_batched(
            || seeded_rng(42),
            |mut rng| {
                let key_pair = bls::key_gen(&mut rng);
                let _ = black_box(key_pair);
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_bls_sign(c: &mut Criterion) {
    c.bench_function("bls/sign", |b| {
        let mut rng = seeded_rng(7);
        let secret_key = Scalar::rand(&mut rng);
        let message = b"math_core bls sign benchmark";

        b.iter(|| {
            let signature = bls::sign(black_box(&secret_key), black_box(message));
            let _ = black_box(signature);
        });
    });
}

fn bench_bls_verify(c: &mut Criterion) {
    c.bench_function("bls/verify", |b| {
        let mut rng = seeded_rng(11);
        let (secret_key, public_key) = bls::key_gen(&mut rng);
        let message = b"math_core bls verify benchmark";
        let signature = bls::sign(&secret_key, message);

        b.iter(|| {
            let verified = bls::verify(
                black_box(&public_key),
                black_box(message),
                black_box(&signature),
            );
            black_box(verified);
        });
    });
}

fn aggregate_partial_signatures(partial_sigs: &[G2Affine], indices: &[u64]) -> G2Affine {
    let mut aggregated = G2Projective::zero();

    for j in 0..partial_sigs.len() {
        let lambda = dkg::lagrange_coeff_at_zero(indices, j);
        aggregated += &(partial_sigs[j] * lambda);
    }

    aggregated.into_affine()
}

fn bench_bls_aggregate(c: &mut Criterion) {
    let mut group = c.benchmark_group("bls/aggregate");

    for &signature_count in &[5usize, 10, 20] {
        let (_, _, final_shares, _, _) = dkg::simulate_dkg(signature_count, signature_count);
        let message = b"math_core bls aggregate benchmark";

        let mut partial_sigs = Vec::with_capacity(signature_count);
        let mut indices = Vec::with_capacity(signature_count);

        for j in 0..signature_count {
            partial_sigs.push(bls::sign(&final_shares[j], message));
            indices.push((j + 1) as u64);
        }

        group.bench_with_input(BenchmarkId::new("aggregate", signature_count), &signature_count, |b, _| {
            b.iter(|| {
                let aggregated = aggregate_partial_signatures(
                    black_box(&partial_sigs),
                    black_box(&indices),
                );
                black_box(aggregated);
            });
        });
    }

    group.finish();
}

fn bench_schnorr_prove(c: &mut Criterion) {
    c.bench_function("schnorr/prove", |b| {
        let mut rng = seeded_rng(19);
        let share = Scalar::rand(&mut rng);
        let public_key = (G1Affine::generator() * share).into_affine();
        let message = b"math_core schnorr prove benchmark";
        let nonce = 17u64;
        let ts = 20240526u64;

        b.iter_batched(
            || seeded_rng(19),
            |mut rng| {
                let proof = schnorr::schnorr_prove(
                    &mut rng,
                    black_box(share),
                    black_box(public_key),
                    black_box(message),
                    black_box(nonce),
                    black_box(ts),
                );
                let _ = black_box(proof);
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_schnorr_verify(c: &mut Criterion) {
    c.bench_function("schnorr/verify", |b| {
        let mut rng = seeded_rng(23);
        let share = Scalar::rand(&mut rng);
        let public_key = (G1Affine::generator() * share).into_affine();
        let message = b"math_core schnorr verify benchmark";
        let nonce = 29u64;
        let ts = 20240526u64;
        let (proof_r, proof_s) = schnorr::schnorr_prove(&mut rng, share, public_key, message, nonce, ts);

        b.iter(|| {
            let verified = schnorr::schnorr_verify(
                black_box(public_key),
                black_box(proof_r),
                black_box(proof_s),
                black_box(message),
                black_box(nonce),
                black_box(ts),
            );
            black_box(verified);
        });
    });
}

criterion_group!(
    benches,
    bench_poly_create,
    bench_poly_evaluate,
    bench_dkg_simulate,
    bench_dkg_verify_share,
    bench_dkg_reconstruct,
    bench_bls_key_gen,
    bench_bls_sign,
    bench_bls_verify,
    bench_bls_aggregate,
    bench_schnorr_prove,
    bench_schnorr_verify,
);

criterion_main!(benches);