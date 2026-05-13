use ark_bn254::Fq as Scalar;
use math_core::poly::{create_poly, evaluate_poly};

#[test]
fn create_and_evaluate_poly() {
    let poly = create_poly(vec![Scalar::from(3), Scalar::from(2), Scalar::from(1)]);
    let value = evaluate_poly(&poly, Scalar::from(2));

    assert_eq!(value, Scalar::from(11));
}
