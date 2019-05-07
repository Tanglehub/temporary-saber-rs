#![allow(unused)]

use core::ops::{Add, Mul, Shr, Sub};

use sha3::digest::{ExtendableOutput, Input, XofReader};

use crate::params::*;
use crate::poly::Poly;

/// Also known as `l`
pub const K: usize = 3;

pub const MU: usize = 8;
pub const DELTA: usize = 3;
pub const POLYVECCOMPRESSEDBYTES: usize = K * (N * 10) / 8;
pub const CIPHERTEXTBYTES: usize = POLYVECCOMPRESSEDBYTES;
pub const MESSAGEBYTES: usize = 32;
pub const RECONBYTES: usize = DELTA * N / 8;
pub const RECONBYTES_KEM: usize = (DELTA + 1) * N / 8;
pub const INDCPA_PUBKEYBYTES: usize = 992;
pub const INDCPA_SECRETKEYBYTES: usize = 1248;
pub const PUBLICKEYBYTES: usize = 992;
pub const SECRETKEYBYTES: usize = 2304;
pub const BYTES_CCA_DEC: usize = 1088;
pub const MSG2POL_CONST: u8 = 9;

mod ffi {
    use super::*;

    extern "C" {
        pub fn indcpa_kem_keypair(pk: *mut PublicKey, sk: *mut SecretKey);
        pub fn indcpa_kem_enc(
            message_received: *mut u8,
            noiseseed: *mut u8,
            pk: *const PublicKey,
            ciphertext: *mut u8,
        );
        pub fn indcpa_kem_dec(sk: *const SecretKey, ciphertext: *const u8, message_dec: *mut u8);
        pub fn randombytes(output: *mut u8, len: u64);
        pub fn shake128(output: *mut u8, outlen: u64, input: *const u8, inlen: u64);

        // GenMatrix(polyvec *a, const unsigned char *seed)
        pub fn GenMatrix(a: *mut Matrix, seed: *const u8);

        // GenSecret(uint16_t r[SABER_K][SABER_N],const unsigned char *seed)
        pub fn GenSecret(s: *mut Vector, seed: *const u8);

        // void MatrixVectorMul(polyvec *a, uint16_t skpv[SABER_K][SABER_N], uint16_t res[SABER_K][SABER_N], uint16_t mod, int16_t transpose);
        pub fn MatrixVectorMul(
            a: *mut Matrix,
            skpv: *mut Vector,
            result: *mut Vector,
            modulus: u16,
            transpose: i16,
        );

        // void POLVECq2BS(uint8_t *sk,  uint16_t skpv[SABER_K][SABER_N]);
        pub fn POLVECq2BS(sk: *mut u8, skpv: *mut Vector);

        // void POLVECp2BS(uint8_t *pk,  uint16_t skpv[SABER_K][SABER_N]);
        pub fn POLVECp2BS(pk: *mut u8, skpv: *mut Vector);

        // void BS2POLVECp(const unsigned char *pk, uint16_t data[SABER_K][SABER_N]);
        pub fn BS2POLVECp(pk: *const u8, data: *mut Vector);

        // void BS2POLVECq(const unsigned char *sk,  uint16_t skpv[SABER_K][SABER_N]);
        pub fn BS2POLVECq(sk: *const u8, skpv: *mut Vector);

        // void POL2MSG(uint16_t *message_dec_unpacked, unsigned char *message_dec);
        pub fn POL2MSG(message_dec_unpacked: *mut Poly, message_dec: *mut u8);

        // void ReconDataGen(uint16_t *vprime, unsigned char *rec_c);
        pub fn ReconDataGen(vprime: *mut Poly, rec_c: *mut u8);

        // void Recon(uint16_t *recon_data,unsigned char *recon_ar,uint16_t *message_dec_unpacked);
        pub fn Recon(recon_data: *mut Poly, recon_ar: *mut u8, message_dec_unpacked: *mut Poly);

        // void BS2POLq(const unsigned char *bytes, uint16_t data[SABER_N]);
        pub fn BS2POLq(bytes: *const u8, data: *mut Poly);
    }
}

/// Vector is equivalent to the reference implementation's `polyvec` type.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vector {
    polys: [Poly; K],
}

impl Add<Vector> for Vector {
    type Output = Vector;

    fn add(self, rhs: Self) -> Vector {
        let Vector { mut polys } = self;
        for i in 0..K {
            polys[i] = polys[i] + rhs.polys[i];
        }
        Vector { polys }
    }
}

impl Add<u16> for Vector {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u16) -> Vector {
        let Vector { mut polys } = self;
        for i in 0..K {
            polys[i] = polys[i] + rhs;
        }
        Vector { polys }
    }
}

impl Mul<Vector> for Vector {
    type Output = Poly;

    /// As implemented by Algorithm 17
    fn mul(self, rhs: Self) -> Poly {
        let mut acc = Poly::default();
        for i in 0..K {
            acc = acc + (self.polys[i] * rhs.polys[i]);
        }
        acc
    }
}

impl Shr<u8> for Vector {
    type Output = Self;

    #[inline]
    fn shr(self, rhs: u8) -> Self {
        let Vector { mut polys } = self;
        for i in 0..K {
            polys[i] = polys[i] >> rhs;
        }
        Vector { polys }
    }
}

impl Default for Vector {
    fn default() -> Self {
        Vector {
            polys: [Poly::default(); K],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Matrix {
    vecs: [Vector; K],
}

impl Add<u16> for Matrix {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u16) -> Matrix {
        let Matrix { mut vecs } = self;
        for i in 0..K {
            vecs[i] = vecs[i] + rhs;
        }
        Matrix { vecs }
    }
}

impl Default for Matrix {
    #[inline]
    fn default() -> Self {
        Matrix {
            vecs: [Vector::default(); K],
        }
    }
}

impl Matrix {
    /// As implemented by Algorithm 16
    #[inline]
    fn mul(self, rhs: Vector) -> Vector {
        let mut result = Vector::default();
        for i in 0..K {
            result.polys[i] = self.vecs[i] * rhs;
        }
        result
    }

    /// As implemented by Algorithm 16
    #[inline]
    fn mul_transpose(self, rhs: Vector) -> Vector {
        let mut result = Vector::default();
        for i in 0..K {
            for j in 0..K {
                result.polys[i] = result.polys[i] + self.vecs[j].polys[i] * rhs.polys[j];
            }
        }
        result
    }
}

impl Shr<u8> for Matrix {
    type Output = Self;

    #[inline]
    fn shr(self, rhs: u8) -> Self {
        let Matrix { mut vecs } = self;
        for i in 0..N {
            vecs[i] = vecs[i] >> rhs;
        }
        Matrix { vecs }
    }
}

#[repr(C)]
pub struct PublicKey([u8; PUBLICKEYBYTES]);

#[repr(C)]
pub struct SecretKey([u8; SECRETKEYBYTES]);

fn gen_matrix(seed: &[u8]) -> Matrix {
    debug_assert_eq!(seed.len(), SEEDBYTES);

    let mut hasher = sha3::Shake128::default();
    hasher.input(seed);
    let mut xof = hasher.xof_result();

    let mut matrix = Matrix::default();
    let mut buf = [0; 13 * N / 8];
    for idx in 0..K {
        for idx2 in 0..K {
            xof.read(&mut buf);
            matrix.vecs[idx].polys[idx2] = Poly::from(&buf);
        }
    }
    matrix
}

fn gen_secret(seed: &[u8; NOISE_SEEDBYTES]) -> Vector {
    let mut hasher = sha3::Shake128::default();
    hasher.input(seed);
    let mut xof = hasher.xof_result();

    let mut secret = Vector::default();
    let mut buf = [0; 4];
    for idx in 0..K {
        for idx2 in (0..N).step_by(4) {
            xof.read(&mut buf);

            let t = load_littleendian(&buf);
            let mut d = 0;
            for idx3 in 0..4 {
                d += (t >> idx3) & 0x11111111;
            }

            let mut a = [0; 4];
            let mut b = [0; 4];
            a[0] = (d & 0xF) as u16;
            b[0] = ((d >> 4) & 0xF) as u16;
            a[1] = ((d >> 8) & 0xF) as u16;
            b[1] = ((d >> 12) & 0xF) as u16;
            a[2] = ((d >> 16) & 0xF) as u16;
            b[2] = ((d >> 20) & 0xF) as u16;
            a[3] = ((d >> 24) & 0xF) as u16;
            b[3] = (d >> 28) as u16;

            secret.polys[idx].coeffs[idx2] = (a[0]).wrapping_sub(b[0]);
            secret.polys[idx].coeffs[idx2 + 1] = (a[1]).wrapping_sub(b[1]);
            secret.polys[idx].coeffs[idx2 + 2] = (a[2]).wrapping_sub(b[2]);
            secret.polys[idx].coeffs[idx2 + 3] = (a[3]).wrapping_sub(b[3]);
        }
    }
    secret
}

fn load_littleendian(bytes: &[u8; 4]) -> u64 {
    let mut r = 0;
    for idx in 0..bytes.len() {
        r |= u64::from(bytes[idx]) << (8 * idx);
    }
    r
}

/// Returns a tuple (public_key, secret_key), of PublicKey, SecretKey objects
// C type in reference: void indcpa_kem_keypair(unsigned char *pk, unsigned char *sk);
fn indcpa_kem_keypair() -> (PublicKey, SecretKey) {
    let mut a = Matrix::default();
    let mut sk_vec = Vector::default();
    let mut pk_vec;
    let mut sk = SecretKey([0; SECRETKEYBYTES]);
    let mut pk = PublicKey([0; PUBLICKEYBYTES]);
    let mut seed: [u8; SEEDBYTES] = rand::random();
    let mut noiseseed: [u8; COINBYTES] = rand::random();

    unsafe {
        ffi::GenMatrix(&mut a as *mut Matrix, seed.as_ptr());
        ffi::GenSecret(&mut sk_vec as *mut Vector, noiseseed.as_ptr());

        // Compute b (called `res` in reference implementation)
        pk_vec = a.mul(sk_vec);

        // Rounding of b
        pk_vec = (pk_vec + 4) >> 3;

        // Save the secret and public vectors
        ffi::POLVECq2BS(sk.0.as_mut_ptr(), &mut sk_vec as *mut Vector);
        ffi::POLVECp2BS(pk.0.as_mut_ptr(), &mut pk_vec as *mut Vector);
        (&mut pk.0[POLYVECCOMPRESSEDBYTES..]).copy_from_slice(&seed[..]);
    }
    (pk, sk)
}

// C type in reference: void indcpa_kem_enc(unsigned char *message_received, unsigned char *noiseseed, const unsigned char *pk, unsigned char *ciphertext)
fn indcpa_kem_enc(
    message_received: &[u8; KEYBYTES],
    noiseseed: &[u8; NOISE_SEEDBYTES],
    pk: &PublicKey,
) -> [u8; BYTES_CCA_DEC] {
    let mut a = Matrix::default();
    let mut sk_vec = Vector::default();
    let mut pk_vec: Vector;
    let mut ciphertext = [0; BYTES_CCA_DEC];
    let mut public_key = PublicKey([0; PUBLICKEYBYTES]);
    let mut v1_vec: Vector = Vector::default();
    let pol_p: Poly;
    let mut m_p = Poly::default();
    let mut rec = [0; RECONBYTES_KEM];

    let (pk, seed) = pk.0.split_at(POLYVECCOMPRESSEDBYTES);

    a = gen_matrix(seed);
    sk_vec = gen_secret(&noiseseed);

    // Compute b' (called `res` in reference implementation)
    pk_vec = a.mul_transpose(sk_vec);

    // Rounding of b' into v_p
    pk_vec = (pk_vec + 4) >> 3;

    unsafe {
        // ct = POLVECp2BS(v_p)
        ffi::POLVECp2BS(ciphertext.as_mut_ptr(), &mut pk_vec as *mut Vector);

        // v' = BS2POLVECp(pk)
        ffi::BS2POLVECp(pk.as_ptr(), &mut v1_vec as *mut Vector);
    }

    // pol_p = VectorMul(v', s', p)
    pol_p = v1_vec * sk_vec;

    // m_p = MSG2POL(m)
    for idx in 0..KEYBYTES {
        for idx2 in 0..8 {
            m_p.coeffs[8 * idx + idx2] = u16::from((message_received[idx] >> idx2) & 0x01);
        }
    }
    m_p = m_p << MSG2POL_CONST;

    // m_p = m_p + pol_p mod p
    m_p = m_p + pol_p;

    unsafe {
        // rec = ReconDataGen(m_p)
        ffi::ReconDataGen(&mut m_p as *mut Poly, rec.as_mut_ptr());
    }

    // CipherText_cpa = (rec || ct)
    ciphertext[POLYVECCOMPRESSEDBYTES..POLYVECCOMPRESSEDBYTES + RECONBYTES_KEM]
        .copy_from_slice(&rec);
    ciphertext
}

// C type in reference: void indcpa_kem_dec(const unsigned char *sk, const unsigned char *ciphertext, unsigned char message_dec[])
fn indcpa_kem_dec(sk: &SecretKey, ciphertext: &[u8; BYTES_CCA_DEC]) -> [u8; MESSAGEBYTES] {
    let mut sk_vec = Vector::default();
    let mut b_vec = Vector::default();
    let mut message_dec_unpacked = Poly::default();
    let mut message_dec = [0; MESSAGEBYTES];

    // Extract (rec || ct) = CipherText
    let (ct, _) = ciphertext.split_at(POLYVECCOMPRESSEDBYTES);
    let mut rec = [0; RECONBYTES_KEM];
    rec.copy_from_slice(&ciphertext[POLYVECCOMPRESSEDBYTES..]);

    unsafe {
        // s = BS2POLVECq(SecretKey_cpa)
        ffi::BS2POLVECq(sk.0.as_ptr(), &mut sk_vec as *mut Vector);

        // b = BS2BOLVECp(ct)
        ffi::BS2POLVECp(ct.as_ptr(), &mut b_vec as *mut Vector);

        // v' = VectorMul(b, s, p)
        let mut v1 = b_vec * sk_vec;

        // m' = Recon(rec, v')
        ffi::Recon(
            &mut v1 as *mut Poly,
            rec.as_mut_ptr(),
            &mut message_dec_unpacked as *mut Poly,
        );

        // m = POL2MSG(m')
        ffi::POL2MSG(
            &mut message_dec_unpacked as *mut Poly,
            message_dec.as_mut_ptr(),
        );
    }
    message_dec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indcpa_impl() {
        let (pk, sk) = indcpa_kem_keypair();
        for _ in 0..100 {
            let mut noiseseed = rand::random::<[u8; NOISE_SEEDBYTES]>();
            let mut message_received = rand::random::<[u8; 32]>();
            let ciphertext = indcpa_kem_enc(&message_received, &noiseseed, &pk);
            let message_dec = indcpa_kem_dec(&sk, &ciphertext);
            assert_eq!(&message_dec[..], &message_received[..]);
        }
    }

    #[test]
    fn indcpa_reference() {
        let mut pk = PublicKey([0; PUBLICKEYBYTES]);
        let mut sk = SecretKey([0; SECRETKEYBYTES]);

        for _ in 0..100 {
            let mut noiseseed = rand::random::<[u8; NOISE_SEEDBYTES]>();
            let mut message_received = rand::random::<[u8; MESSAGEBYTES]>();
            let mut ciphertext = [0; BYTES_CCA_DEC];
            let mut message_dec = [0; MESSAGEBYTES];
            unsafe {
                ffi::indcpa_kem_keypair(&mut pk as *mut PublicKey, &mut sk as *mut SecretKey);
                ffi::indcpa_kem_enc(
                    message_received.as_mut_ptr(),
                    noiseseed.as_mut_ptr(),
                    &pk as *const PublicKey,
                    ciphertext.as_mut_ptr(),
                );
                ffi::indcpa_kem_dec(
                    &mut sk as *mut SecretKey,
                    ciphertext.as_ptr(),
                    message_dec.as_mut_ptr(),
                );
            }
            assert_eq!(&message_dec[..], &message_received[..]);
        }
    }

    #[test]
    fn indcpa_keypair() {
        let mut noiseseed = rand::random::<[u8; NOISE_SEEDBYTES]>();
        let mut message_received = [b'A'; 32];
        let mut ciphertext = [b'B'; BYTES_CCA_DEC];
        let mut message_dec = [b'C'; 32];

        let (mut pk, mut sk) = indcpa_kem_keypair();
        unsafe {
            ffi::indcpa_kem_enc(
                message_received.as_mut_ptr(),
                noiseseed.as_mut_ptr(),
                &pk as *const PublicKey,
                ciphertext.as_mut_ptr(),
            );
            ffi::indcpa_kem_dec(
                &sk as *const SecretKey,
                ciphertext.as_ptr(),
                message_dec.as_mut_ptr(),
            );
        }
        assert_eq!(&message_dec[..], &message_received[..]);
    }

    #[test]
    fn polyveccompressedbytes_value() {
        assert_eq!(POLYVECCOMPRESSEDBYTES + SEEDBYTES, PUBLICKEYBYTES);
    }

    #[test]
    fn bytes_cca_dec_value() {
        assert_eq!(CIPHERTEXTBYTES + RECONBYTES_KEM, BYTES_CCA_DEC);
    }
}
