use math_core::bls::{key_gen, sign, verify};
use ark_std::test_rng;

#[test]
fn bls_sign_and_verify_round_trip() {
    let mut rng = test_rng();
    let (secret_key, public_key) = key_gen(&mut rng);
    let message = b"hello bls";
    let signature = sign(&secret_key, message);

    assert!(verify(&public_key, message, &signature));
    assert!(!verify(&public_key, b"tampered", &signature));
}
