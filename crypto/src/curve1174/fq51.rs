//! fq51.rs - Bernstein encoding of arithmetic on Fq embedding field.

//
// Copyright (c) 2018 Stegos
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use super::*;
use crate::CryptoError;

const BOT_51_BITS: i64 = ((1 << 51) - 1); // Fq51 frames contain 51 bits
const BOT_47_BITS: i64 = ((1 << 47) - 1); // MSB frame only has 47 bits

// -----------------------------------------------------------------
// field Fq51 is Fq broken into 51-bit frames
// in little-endian order, of i64 (vs u64 in Fq)
// 47 bits in last frame

#[derive(Copy, Clone)]
pub struct Fq51(pub [i64; 5]);

pub const FQ51_0: Fq51 = Fq51([0; 5]);
pub const FQ51_1: Fq51 = Fq51([1, 0, 0, 0, 0]);

impl Fq51 {
    pub fn zero() -> Fq51 {
        FQ51_0
    }

    pub fn one() -> Fq51 {
        FQ51_1
    }

    pub fn is_zero(&self) -> bool {
        // faster than test == FQ51_0
        // avoid scr(FQ51_0);
        let mut tmp = *self;
        scr(&mut tmp);
        tmp.0 == FQ51_0.0
    }

    pub fn is_odd(&self) -> bool {
        (self.0[0] & 1) != 0
    }

    pub fn sqr(self) -> Fq51 {
        let mut tmp = Fq51::zero();
        gsqr(&self, &mut tmp);
        tmp
    }

    fn nbr_str(&self) -> String {
        let U256(yv) = U256::from(*self);
        basic_nbr_str(&yv)
    }
}

impl From<i64> for Fq51 {
    fn from(x: i64) -> Fq51 {
        Fq51::from(Fq::from(x))
    }
}

impl From<Fq> for Fq51 {
    fn from(x: Fq) -> Fq51 {
        let mut tmp = Fq51::zero();
        bin_to_elt(&x.unscaled().bits(), &mut tmp);
        tmp
    }
}

impl From<Fq51> for Fq {
    fn from(x: Fq51) -> Fq {
        let mut tmp = U256::from(x);
        while tmp >= *Q {
            u256::sub_noborrow(&mut tmp.0, &(*Q).0);
        }
        Fq::from(tmp)
    }
}

impl fmt::Debug for Fq51 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Fq51([{:016x}, {:016x}, {:016x}, {:016x}, {:016x}])",
            self.0[0] as u64,
            self.0[1] as u64,
            self.0[2] as u64,
            self.0[3] as u64,
            self.0[4] as u64
        )
    }
}

impl fmt::Display for Fq51 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fq51({})", self.nbr_str())
    }
}

impl Add<Fq51> for Fq51 {
    type Output = Fq51;
    fn add(self, other: Fq51) -> Fq51 {
        let mut dst = Fq51::zero();
        for i in 0..5 {
            dst.0[i] = self.0[i] + other.0[i];
        }
        dst
    }
}

impl Add<i64> for Fq51 {
    type Output = Fq51;
    fn add(self, other: i64) -> Fq51 {
        let mut dst = self;
        dst.0[0] += other;
        dst
    }
}

impl Add<Fq51> for i64 {
    type Output = Fq51;
    fn add(self, other: Fq51) -> Fq51 {
        let mut dst = other;
        dst.0[0] += self;
        dst
    }
}

impl AddAssign<Fq51> for Fq51 {
    fn add_assign(&mut self, other: Fq51) {
        for i in 0..5 {
            self.0[i] += other.0[i];
        }
    }
}

impl AddAssign<i64> for Fq51 {
    fn add_assign(&mut self, other: i64) {
        self.0[0] += other;
    }
}

impl Sub<Fq51> for Fq51 {
    type Output = Fq51;
    fn sub(self, other: Fq51) -> Fq51 {
        let mut dst = Fq51::zero();
        for i in 0..5 {
            dst.0[i] = self.0[i] - other.0[i];
        }
        dst
    }
}

impl Sub<i64> for Fq51 {
    type Output = Fq51;
    fn sub(self, other: i64) -> Fq51 {
        let mut dst = self;
        dst.0[0] -= other;
        dst
    }
}

impl Sub<Fq51> for i64 {
    type Output = Fq51;
    fn sub(self, other: Fq51) -> Fq51 {
        Fq51::from(self) - other
    }
}

impl SubAssign<Fq51> for Fq51 {
    fn sub_assign(&mut self, other: Fq51) {
        for i in 0..5 {
            self.0[i] -= other.0[i];
        }
    }
}

impl SubAssign<i64> for Fq51 {
    fn sub_assign(&mut self, other: i64) {
        self.0[0] -= other;
    }
}

impl Neg for Fq51 {
    type Output = Fq51;
    fn neg(self) -> Fq51 {
        let mut dst = Fq51::zero();
        for i in 0..5 {
            dst.0[i] = -self.0[i];
        }
        dst
    }
}

impl Mul<Fq51> for Fq51 {
    type Output = Fq51;
    fn mul(self, other: Fq51) -> Fq51 {
        let mut dst = Fq51::zero();
        gmul(&self, &other, &mut dst);
        dst
    }
}

impl MulAssign<Fq51> for Fq51 {
    fn mul_assign(&mut self, other: Fq51) {
        let tmp = *self;
        gmul(&tmp, &other, self);
    }
}

impl Mul<Fq51> for i64 {
    type Output = Fq51;
    fn mul(self, other: Fq51) -> Fq51 {
        let mut dst = other;
        gmuli(&mut dst, self);
        dst
    }
}

impl Mul<i64> for Fq51 {
    type Output = Fq51;
    fn mul(self, other: i64) -> Fq51 {
        let mut dst = self;
        gmuli(&mut dst, other);
        dst
    }
}

impl MulAssign<i64> for Fq51 {
    fn mul_assign(&mut self, other: i64) {
        gmuli(self, other);
    }
}

impl Div<Fq51> for Fq51 {
    type Output = Fq51;
    fn div(self, x: Fq51) -> Fq51 {
        let mut tmp = x;
        ginv(&mut tmp);
        let mut ans = Fq51::zero();
        gmul(&self, &tmp, &mut ans);
        ans
    }
}

impl PartialEq for Fq51 {
    fn eq(&self, other: &Fq51) -> bool {
        Fq::from(*self) == Fq::from(*other)
    }
}

// ---------------------------------------------------------
// Group primitive operators

pub fn gadd(x: &Fq51, y: &Fq51, w: &mut Fq51) {
    w.0[0] = x.0[0] + y.0[0];
    w.0[1] = x.0[1] + y.0[1];
    w.0[2] = x.0[2] + y.0[2];
    w.0[3] = x.0[3] + y.0[3];
    w.0[4] = x.0[4] + y.0[4];
}

pub fn gsub(x: &Fq51, y: &Fq51, w: &mut Fq51) {
    w.0[0] = x.0[0] - y.0[0];
    w.0[1] = x.0[1] - y.0[1];
    w.0[2] = x.0[2] - y.0[2];
    w.0[3] = x.0[3] - y.0[3];
    w.0[4] = x.0[4] - y.0[4];
}

pub fn gdec(x: &Fq51, w: &mut Fq51) {
    w.0[0] -= x.0[0];
    w.0[1] -= x.0[1];
    w.0[2] -= x.0[2];
    w.0[3] -= x.0[3];
    w.0[4] -= x.0[4];
}

pub fn gneg(w: &Fq51, x: &mut Fq51) {
    x.0[0] = -w.0[0];
    x.0[1] = -w.0[1];
    x.0[2] = -w.0[2];
    x.0[3] = -w.0[3];
    x.0[4] = -w.0[4];
}

// w*=2
pub fn gmul2(w: &mut Fq51) {
    w.0[0] *= 2;
    w.0[1] *= 2;
    w.0[2] *= 2;
    w.0[3] *= 2;
    w.0[4] *= 2;
}

// w-=2*x
pub fn gsb2(x: &Fq51, w: &mut Fq51) {
    w.0[0] -= 2 * x.0[0];
    w.0[1] -= 2 * x.0[1];
    w.0[2] -= 2 * x.0[2];
    w.0[3] -= 2 * x.0[3];
    w.0[4] -= 2 * x.0[4];
}

// reduce w - Short Coefficient Reduction
pub fn scr(w: &mut Fq51) {
    let w0 = w.0[0];
    let t0 = w0 & BOT_51_BITS;

    let t1 = w.0[1] + (w0 >> 51);
    w.0[1] = t1 & BOT_51_BITS;

    let t2 = w.0[2] + (t1 >> 51);
    w.0[2] = t2 & BOT_51_BITS;

    let t3 = w.0[3] + (t2 >> 51);
    w.0[3] = t3 & BOT_51_BITS;

    let t4 = w.0[4] + (t3 >> 51);
    w.0[4] = t4 & BOT_47_BITS;
    w.0[0] = t0 + 9 * (t4 >> 47);
}

// multiply w by a constant, w*=i

pub fn gmuli(w: &mut Fq51, i: i64) {
    let ii = i as i128;
    let t0 = (w.0[0] as i128) * ii;
    w.0[0] = (t0 as i64) & BOT_51_BITS;

    let t1 = (w.0[1] as i128) * ii + (t0 >> 51);
    w.0[1] = (t1 as i64) & BOT_51_BITS;

    let t2 = (w.0[2] as i128) * ii + (t1 >> 51);
    w.0[2] = (t2 as i64) & BOT_51_BITS;

    let t3 = (w.0[3] as i128) * ii + (t2 >> 51);
    w.0[3] = (t3 as i64) & BOT_51_BITS;

    let t4 = (w.0[4] as i128) * ii + (t3 >> 51);
    w.0[4] = (t4 as i64) & BOT_47_BITS;
    w.0[0] += (9 * (t4 >> 47)) as i64;
}

// z=x^2

#[inline(never)]
pub fn gsqr(x: &Fq51, z: &mut Fq51) {
    let t4 = 2 * ((x.0[0] as i128) * (x.0[4] as i128) + (x.0[1] as i128) * (x.0[3] as i128))
        + (x.0[2] as i128) * (x.0[2] as i128);
    z.0[4] = (t4 as i64) & BOT_47_BITS;

    let t0 = (x.0[0] as i128) * (x.0[0] as i128)
        + 288 * ((x.0[1] as i128) * (x.0[4] as i128) + (x.0[2] as i128) * (x.0[3] as i128))
        + 9 * (t4 >> 47);
    z.0[0] = (t0 as i64) & BOT_51_BITS;

    let t1 = 2 * (x.0[0] as i128) * (x.0[1] as i128)
        + 288 * (x.0[2] as i128) * (x.0[4] as i128)
        + 144 * (x.0[3] as i128) * (x.0[3] as i128)
        + (t0 >> 51);
    z.0[1] = (t1 as i64) & BOT_51_BITS;

    let t2 = (x.0[1] as i128) * (x.0[1] as i128)
        + 2 * (x.0[0] as i128) * (x.0[2] as i128)
        + 288 * (x.0[3] as i128) * (x.0[4] as i128)
        + (t1 >> 51);
    z.0[2] = (t2 as i64) & BOT_51_BITS;

    let t3 = 144 * (x.0[4] as i128) * (x.0[4] as i128)
        + 2 * ((x.0[0] as i128) * (x.0[3] as i128) + (x.0[1] as i128) * (x.0[2] as i128))
        + (t2 >> 51);
    z.0[3] = (t3 as i64) & BOT_51_BITS;

    let t4 = (z.0[4] as i128) + (t3 >> 51);
    z.0[4] = (t4 as i64) & BOT_47_BITS;
    z.0[0] += (9 * (t4 >> 47)) as i64;
}

#[inline(never)]
pub fn gmul(x: &Fq51, y: &Fq51, z: &mut Fq51) {
    // 5M + 4A
    let t4 = (x.0[0] as i128) * (y.0[4] as i128)
        + (x.0[4] as i128) * (y.0[0] as i128)
        + (x.0[1] as i128) * (y.0[3] as i128)
        + (x.0[3] as i128) * (y.0[1] as i128)
        + (x.0[2] as i128) * (y.0[2] as i128);
    z.0[4] = (t4 as i64) & BOT_47_BITS;

    // 7M + 5A
    let t0 = (x.0[0] as i128) * (y.0[0] as i128)
        + 144
            * ((x.0[1] as i128) * (y.0[4] as i128)
                + (x.0[4] as i128) * (y.0[1] as i128)
                + (x.0[2] as i128) * (y.0[3] as i128)
                + (x.0[3] as i128) * (y.0[2] as i128))
        + 9 * (t4 >> 47);
    z.0[0] = (t0 as i64) & BOT_51_BITS;

    // 6M + 5A
    let t1 = (x.0[0] as i128) * (y.0[1] as i128)
        + (x.0[1] as i128) * (y.0[0] as i128)
        + 144
            * ((x.0[3] as i128) * (y.0[3] as i128)
                + (x.0[2] as i128) * (y.0[4] as i128)
                + (x.0[4] as i128) * (y.0[2] as i128))
        + (t0 >> 51);
    z.0[1] = (t1 as i64) & BOT_51_BITS;

    // 6M + 5A
    let t2 = (x.0[1] as i128) * (y.0[1] as i128)
        + (x.0[0] as i128) * (y.0[2] as i128)
        + (x.0[2] as i128) * (y.0[0] as i128)
        + 144 * ((x.0[3] as i128) * (y.0[4] as i128) + (x.0[4] as i128) * (y.0[3] as i128))
        + (t1 >> 51);
    z.0[2] = (t2 as i64) & BOT_51_BITS;

    // 6M + 5A
    let t3 = 144 * ((x.0[4] as i128) * (y.0[4] as i128))
        + (x.0[0] as i128) * (y.0[3] as i128)
        + (x.0[3] as i128) * (y.0[0] as i128)
        + (x.0[1] as i128) * (y.0[2] as i128)
        + (x.0[2] as i128) * (y.0[1] as i128)
        + (t2 >> 51);
    z.0[3] = (t3 as i64) & BOT_51_BITS;

    // -------- to this point = 30M + 24A => this clocks as faster than Granger's method for Curve1174
    let t4 = (z.0[4] as i128) + (t3 >> 51);
    z.0[4] = (t4 as i64) & BOT_47_BITS;
    z.0[0] += (9 * (t4 >> 47)) as i64;
}

// Inverse x = 1/x = x^(p-2) mod p
// the exponent (p-2) = "07FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5"
// (61 F's)
#[inline(never)]
pub fn ginv(x: &mut Fq51) {
    let mut w = FQ51_0;
    let mut t1 = FQ51_0;
    let mut t2;
    let mut x3 = FQ51_0;
    let mut x5 = FQ51_0;

    // --------------------------------------
    // 265*M
    gsqr(x, &mut w); // w = x^2
    gmul(x, &w, &mut x3); // x3 = x^3 = x^(2^2-1)
    gmul(&w, &x3, &mut x5); // x5 = x^5

    gsqr(&x3, &mut w);
    gsqr(&w, &mut t1); // t1 = x^(2^4-4)
    gmul(&x3, &t1, &mut w); // w = x^(2^4-1)

    gsqr(&w, &mut t1);
    gsqr(&t1, &mut w); // w = x^(2^6-4)
    gmul(&x3, &w, &mut t1); // t1 = x^(2^6-1)
    t2 = t1;
    for _ in 0..3 {
        gsqr(&t1, &mut w);
        gsqr(&w, &mut t1);
    }
    gmul(&t1, &t2, &mut w); // w = x^(2^12-1)

    gsqr(&w, &mut t1);
    gsqr(&t1, &mut w); // w = x^(2^14-4)
    gmul(&x3, &w, &mut t1); // t1 = x^(2^14-1)
    t2 = t1;
    for _ in 0..7 {
        gsqr(&t1, &mut w);
        gsqr(&w, &mut t1);
    }
    gmul(&t1, &t2, &mut w); // w = x^(2^28-1)

    gsqr(&w, &mut t1);
    gsqr(&t1, &mut w); // w = x^(2^30-4)
    gmul(&x3, &w, &mut t1); // t1 = x^(2^30-1)
    t2 = t1;
    for _ in 0..15 {
        gsqr(&t1, &mut w);
        gsqr(&w, &mut t1);
    }
    gmul(&t1, &t2, &mut w); // w = x^(2^60-1)

    t2 = w;
    for _ in 0..30 {
        gsqr(&w, &mut t1);
        gsqr(&t1, &mut w);
    }
    gmul(&w, &t2, &mut t1); // t1 = x^(2^120-1)

    gsqr(&t1, &mut w);
    gsqr(&w, &mut t1); // t1 = x^(2^122 - 4)
    gmul(&x3, &t1, &mut w); // w = x^(2^122-1)
    t2 = w;
    for _ in 0..61 {
        gsqr(&w, &mut t1);
        gsqr(&t1, &mut w);
    }
    gmul(&w, &t2, &mut t1); // t1 = x^(2^244-1)

    gsqr(&t1, &mut w); // w = x^(2^245-2)
    gmul(&x, &w, &mut t1); // t1 = x^(2^245-1)
    gsqr(&t1, &mut w);
    gsqr(&w, &mut t1); // t1 = x^(2^247-4)
    gmul(&x3, &t1, &mut w); // w = x^(2^247-1)

    gsqr(&w, &mut t1);
    gsqr(&t1, &mut w);
    gsqr(&w, &mut t1);
    gsqr(&t1, &mut w); // w = x^(2^251-16)
    gmul(&x5, &w, x); // x = x^(2^251-11)
}

pub fn gdec2(x: &mut Fq51) {
    x.0[0] -= 2;
}

#[inline(never)]
pub fn gsqrt(x: &Fq51) -> Result<Fq51, CryptoError> {
    // we need to perform (x^((q+1)/4) mod q)
    // for (q + 1)/4 = 0x01FF_FFFF_FFFF_FFFF__FFFF_FFFF_FFFF_FFFF__FFFF_FFFF_FFFF_FFFF__FFFF_FFFF_FFFF_FFFE
    //               = 2^(2*(248-1))
    // At end we will verify: sqrt(x)^2 mod q == x, bypassing usual check for Legendre symbol == +1.

    let mut w = FQ51_0;
    let mut t1 = FQ51_0;
    let mut t2 = FQ51_0;

    let mut t8 = FQ51_0;
    let mut t16 = FQ51_0;
    let mut t32 = FQ51_0;
    let mut t64 = FQ51_0;

    // --------------------------------------
    // 260*M

    gsqr(x, &mut w); // w = x^2
    gmul(x, &w, &mut t1); // t1 = x^3
    gsqr(&t1, &mut w); // w = x^6
    gsqr(&w, &mut t2); // t2 = x^12
    gmul(&t1, &t2, &mut w); // w = x^15 = x^(2^4-1)

    t2 = w;
    for _ in 0..2 {
        // shift exponent left 4 bits
        gsqr(&w, &mut t1);
        gsqr(&t1, &mut w);
    }
    gmul(&t2, &w, &mut t8); // t8 = x^(2^8-1)

    w = t8;
    for _ in 0..4 {
        // shift exponent left 8 bits
        gsqr(&w, &mut t1);
        gsqr(&t1, &mut w);
    }
    gmul(&t8, &w, &mut t16); // t16 = x^(2^16-1)

    w = t16;
    for _ in 0..8 {
        // shift exponent left 16 bits
        gsqr(&w, &mut t1);
        gsqr(&t1, &mut w);
    }
    gmul(&t16, &w, &mut t32); // t32 = x^(2^32-1)

    w = t32;
    for _ in 0..16 {
        // shift exponent left 32 bits
        gsqr(&w, &mut t1);
        gsqr(&t1, &mut w);
    }
    gmul(&t32, &w, &mut t64); // t64 = x^(2^64-1)

    w = t64;
    for _ in 0..32 {
        // shift exponent left 64 bits
        gsqr(&w, &mut t1);
        gsqr(&t1, &mut w);
    }
    gmul(&t64, &w, &mut t1); // t1 = x^(2^128-1)

    for _ in 0..32 {
        // shift exponent left 64 bits
        gsqr(&t1, &mut w);
        gsqr(&w, &mut t1);
    }
    gmul(&t64, &t1, &mut w); // w = x^(2^192-1)

    for _ in 0..16 {
        // shift exponent left 32 bits
        gsqr(&w, &mut t1);
        gsqr(&t1, &mut w);
    }
    gmul(&t32, &w, &mut t1); // t1 = x^(2^224-1)

    for _ in 0..8 {
        // shift exponent left 16 bits
        gsqr(&t1, &mut w);
        gsqr(&w, &mut t1);
    }
    gmul(&t16, &t1, &mut w); // w = x^(2^240-1)

    for _ in 0..4 {
        // shift exponent left 8 bits
        gsqr(&w, &mut t1);
        gsqr(&t1, &mut w);
    }
    gmul(&t8, &w, &mut t1); // t1 = x^(2^248-1)

    // shift exponent left 1 bit
    gsqr(&t1, &mut w); // w = x^(2*(248-1)) =? sqrt(x)

    gsqr(&w, &mut t1); // t1 = sqrt(x)^2 =? x

    if t1 == *x {
        Ok(w)
    } else {
        Err(CryptoError::NotQuadraticResidue)
    }
}

// ----------------------------------------------------------
// convert consecutive (little-endian) 64-bit cells
// into 51-bit representation
pub fn bin_to_elt(y: &U256, x: &mut Fq51) {
    {
        let mut s = y.0[0] as u128;
        x.0[0] = (s as i64) & BOT_51_BITS;
        s >>= 51;
        s += (y.0[1] as u128) << (64 - 51);
        x.0[1] = (s as i64) & BOT_51_BITS;
        s >>= 51;
        s += (y.0[2] as u128) << (128 - 2 * 51);
        x.0[2] = (s as i64) & BOT_51_BITS;
        s >>= 51;
        s += (y.0[3] as u128) << (192 - 3 * 51);
        x.0[3] = (s as i64) & BOT_51_BITS;
        s >>= 51;
        x.0[4] = s as i64;
    }
    scr(x);
}

impl Fq51 {
    pub fn from_str(s: &str) -> Result<Fq51, CryptoError> {
        let bin = U256::try_from_hex(s)?;
        let mut e = Fq51::zero();
        bin_to_elt(&bin, &mut e);
        Ok(e)
    }
}

// -------------------------------------------------------------------------------
// Binary/frames conversions

// convert a frame Fq51 from 51-bit representation into
// into a collection of 64-bit cells
impl From<Fq51> for U256 {
    fn from(x: Fq51) -> U256 {
        let mut xx = x;
        scr(&mut xx);
        let mut y = U256::zero();
        clean_convert_Fq51_to_lev_u64(&xx, &mut y.0);
        y
    }
}

fn clean_convert_Fq51_to_lev_u64(x: &Fq51, y: &mut [u64; 4]) {
    // convert an Fq51 that has already been scr()
    // to [u64;4] vector
    let mut s = x.0[0] as u128;
    s += (x.0[1] as u128) << 51;
    y[0] = s as u64;
    s >>= 64;
    s += (x.0[2] as u128) << (2 * 51 - 64);
    y[1] = s as u64;
    s >>= 64;
    s += (x.0[3] as u128) << (3 * 51 - 128);
    y[2] = s as u64;
    s >>= 64;
    s += (x.0[4] as u128) << (4 * 51 - 192);
    y[3] = s as u64;
}

// ------------------------------------------------------------------

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn check_gsqrt() {
        for _ in 0..1000 {
            let x = Fq::random();
            let x51 = Fq51::from(x);
            let mut xsq51 = x51;
            gsqr(&x51, &mut xsq51);
            let mut xsq51a = x51;
            // check that squaring = self mul
            gmul(&x51, &x51, &mut xsq51a);
            assert!(xsq51a == xsq51);
            let xrt51 = gsqrt(&xsq51).expect("Valid root");
            assert!(x51 == xrt51 || x51 == -xrt51);
            /* */
            let xb = Fq::random();
            let xsq = xb * xb;
            let xsq51b = Fq51::from(xsq);
            let xrt51b = gsqrt(&xsq51b).expect("Valid root");
            let xrt = Fq::from(xrt51b);
            assert!(xrt == xb || -xrt == xb);
            /* */
        }
    }

    #[test]
    pub fn check_ginv() {
        for _ in 0..1000 {
            let x = Fq::random();
            let x51 = Fq51::from(x);
            let mut xinv51 = x51;
            ginv(&mut xinv51);
            let mut xprod51 = xinv51;
            gmul(&x51, &xinv51, &mut xprod51);
            assert!(FQ51_1 == xprod51);
        }
    }
}
