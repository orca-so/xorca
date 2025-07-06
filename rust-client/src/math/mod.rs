use wasm_bindgen::prelude::*;

/// TODO: Remove these test functions once we have a proper math library. These are for testing the WASM build.

/// A simple math utility function that adds two numbers
#[wasm_bindgen]
pub fn add(a: f64, b: f64) -> f64 {
    a + b
}

/// A simple math utility function that multiplies two numbers
#[wasm_bindgen]
pub fn multiply(a: f64, b: f64) -> f64 {
    a * b
}

/// A simple math utility function that calculates the square of a number
#[wasm_bindgen]
pub fn square(x: f64) -> f64 {
    x * x
}

/// A simple math utility function that calculates the power of a number
#[wasm_bindgen]
pub fn power(base: f64, exponent: f64) -> f64 {
    base.powf(exponent)
}
