//! Polynomial operations based on the `bn254` base field.
use ark_bn254::Fq as Scalar;
use ark_poly::{DenseUVPolynomial, Polynomial, univariate::DensePolynomial};

/// Create a univariate polynomial from coefficients in ascending order.
pub fn create_poly(coefficients: Vec<Scalar>) -> DensePolynomial<Scalar> {
    DensePolynomial::from_coefficients_vec(coefficients)
}

/// Evaluate a univariate polynomial at the given point.
pub fn evaluate_poly(poly: &DensePolynomial<Scalar>, point: Scalar) -> Scalar {
    poly.evaluate(&point)
}
