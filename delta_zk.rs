// delta_zk.rs — lightweight constraint "builder" + verifier demo.
// Goal: assemble tiny R1CS-like constraints and verify a witness over a prime field.
// This is NOT a full zk system, but it's useful for prototyping circuits quickly.
//
// Usage pattern (as a lib or bin scaffold):
//   - Define variables (id => value) in the witness.
//   - Add constraints of the form:  (A·X) * (B·X) - (C·X) = 0 mod p
//     where (A·X) = sum(A_i * x_i), similar for B,C.
//   - Verify() checks all constraints modulo the field prime.
//
// Includes a toy Poseidon-like round (very simplified S-box + MDS) for experimentation.

use std::collections::HashMap;

const P: u128 = 0xffff_ffff_0000_0001; // 2^64-based BN-like toy prime (placeholder)
type F = u128;

#[derive(Clone, Debug, Default)]
pub struct LinComb {
    // linear combination: sum(coeff[i] * var[i]) + const_term
    pub terms: Vec<(usize, F)>,
    pub const_term: F,
}

impl LinComb {
    pub fn new() -> Self { Self { terms: vec![], const_term: 0 } }
    pub fn c(mut self, k: F) -> Self { self.const_term = add(self.const_term, k); self }
    pub fn t(mut self, var: usize, coeff: F) -> Self { self.terms.push((var, coeff)); self }
    pub fn eval(&self, w: &Witness) -> F {
        let mut acc = self.const_term;
        for (v, c) in &self.terms {
            let xv = *w.values.get(v).unwrap_or(&0);
            acc = add(acc, mul(*c, xv));
        }
        acc
    }
}

#[derive(Clone, Debug)]
pub struct Constraint {
    // (A·X) * (B·X) - (C·X) = 0  (mod P)
    pub a: LinComb,
    pub b: LinComb,
    pub c: LinComb,
}

#[derive(Clone, Debug, Default)]
pub struct Witness {
    // variable index -> value
    pub values: HashMap<usize, F>,
}

#[derive(Default)]
pub struct Builder {
    pub constraints: Vec<Constraint>,
    pub next_var: usize,
}

impl Builder {
    pub fn new() -> Self { Self { constraints: vec![], next_var: 0 } }
    pub fn alloc(&mut self, val: F) -> usize {
        let id = self.next_var; self.next_var += 1;
        id
    }
    pub fn constrain(&mut self, a: LinComb, b: LinComb, c: LinComb) {
        self.constraints.push(Constraint { a, b, c });
    }
    pub fn mul_gate(&mut self, x: usize, y: usize, z: usize) {
        // enforce: x * y - z = 0
        let a = LinComb::new().t(x, 1);
        let b = LinComb::new().t(y, 1);
        let c = LinComb::new().t(z, 1);
        self.constrain(a, b, c);
    }
    pub fn add_gate(&mut self, x: usize, y: usize, z: usize) {
        // enforce: (x + y) - z = 0  ==> (x + y) * 1 - z = 0
        let a = LinComb::new().t(x, 1).t(y, 1);
        let b = LinComb::new().c(1);
        let c = LinComb::new().t(z, 1);
        self.constrain(a, b, c);
    }
}

pub fn verify(builder: &Builder, wit: &Witness) -> bool {
    builder.constraints.iter().all(|con| {
        let a = con.a.eval(wit);
        let b = con.b.eval(wit);
        let c = con.c.eval(wit);
        sub(mul(a, b), c) % P == 0
    })
}

// ---- Tiny field ops ----
#[inline] fn add(a: F, b: F) -> F { let (s, o) = a.overflowing_add(b); (s as u128 + (o as u128)*0) % P }
#[inline] fn sub(a: F, b: F) -> F { (a + P - (b % P)) % P }
#[inline] fn mul(a: F, b: F) -> F {
    // schoolbook 128-bit mul mod P (naive; fine for small prototypes)
    let res = (a as u128).wrapping_mul(b as u128) % P;
    res
}
#[inline] fn exp(mut x: F, mut e: u128) -> F {
    let mut r: F = 1;
    while e > 0 {
        if e & 1 == 1 { r = mul(r, x); }
        x = mul(x, x); e >>= 1;
    }
    r
}

// ---- Super-simplified Poseidon-ish round for experimentation ----
pub fn poseidon_round(state: &mut [F; 3]) {
    // S-box: x^5
    for x in state.iter_mut() {
        *x = exp(*x, 5);
    }
    // MDS (toy 3x3)
    let m = [[2u128, 1, 1],
             [1, 2, 1],
             [1, 1, 2]];
    let s0 = add(add(mul(m[0][0], state[0]), mul(m[0][1], state[1])), mul(m[0][2], state[2]));
    let s1 = add(add(mul(m[1][0], state[0]), mul(m[1][1], state[1])), mul(m[1][2], state[2]));
    let s2 = add(add(mul(m[2][0], state[0]), mul(m[2][1], state[1])), mul(m[2][2], state[2]));
    state[0] = s0; state[1] = s1; state[2] = s2;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_and_mul_gate() {
        let mut b = Builder::new();
        let x = b.alloc(3);
        let y = b.alloc(5);
        let z = b.alloc(15);
        b.mul_gate(x, y, z);

        let mut w = Witness::default();
        w.values.insert(x, 3);
        w.values.insert(y, 5);
        w.values.insert(z, 15);
        assert!(verify(&b, &w));
    }
}
