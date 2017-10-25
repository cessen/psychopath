#![allow(dead_code)]

pub fn balance_heuristic(a: f32, b: f32) -> f32 {
    if a.is_infinite() {
        a
    } else {
        let mis_fac = a / (a + b);
        a / mis_fac
    }
}

pub fn power_heuristic(a: f32, b: f32) -> f32 {
    if a.is_infinite() {
        a
    } else {
        let a2 = a * a;
        let b2 = b * b;
        let mis_fac = a2 / (a2 + b2);
        a / mis_fac
    }
}
