use secp256k1::recovery::{RecoverableSignature, RecoveryId};
use secp256k1::{key::SecretKey, Message, PublicKey, Secp256k1};
use std::fmt;
use tiny_keccak::{Hasher, Keccak};
use rand::{RngCore, thread_rng};

pub struct EcdsaSig {
    v: u64,
    r: Vec<u8>,
    s: Vec<u8>,
}

impl fmt::Debug for EcdsaSig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "r:{}, s:{}, b: {}", b2h(&self.r), b2h(&self.s), &self.v)
    }
}

impl fmt::Display for EcdsaSig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "r:{}, s:{}, b: {}", b2h(&self.r), b2h(&self.s), &self.v)
    }
}

impl EcdsaSig {
    pub fn from(b: &[u8]) -> Result<Self, secp256k1::Error> {
        if b.len() != 65 {
            println!("EcdsaSig.from: wrong length: {}", b.len());
            return Err(secp256k1::Error::IncorrectSignature);
        }
        let mut s = [0 as u8; 65];
        s.copy_from_slice(b);
        Ok(EcdsaSig {
            r: Vec::from(&s[..32]),
            s: Vec::from(&s[32..64]),
            v: s[64] as u64,
        })
    }

    pub fn to_u8(&self) -> Vec<u8> {
        let mut rv = [0 as u8; 65];
        // let mut v = Vec::from(&rv);
        rv[..32].clone_from_slice(&self.r);
        rv[32..64].clone_from_slice(&self.s);

        // self.r.clone_into(rv[0..]);
        // self.r.clone_into(rv[32..]);
        rv[64] = self.v as u8;
        Vec::from(rv)
    }

    #[allow(dead_code)]
    pub fn to_hex(&self) -> String {
        b2h(&self.to_u8())
    }
}

pub struct CryptoUtils {}

pub fn b2h(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

pub fn h2b(h: &String) -> Vec<u8> {
    if h.starts_with("0x") || h.starts_with("0X") {
        let mut trimmed = h.chars();
        trimmed.by_ref().nth(1);
        hex::decode(trimmed.as_str()).unwrap()
    } else {
        hex::decode(h).unwrap()
    }
}

pub fn keccak256_hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    let mut resp: [u8; 32] = Default::default();
    hasher.finalize(&mut resp);
    resp.iter().cloned().collect()
}

pub fn public_to_address(public: &[u8]) -> Vec<u8> {
    let hash = keccak256_hash(public);
    Vec::from(&hash[12..])
}

pub fn rand_hex(len: usize) -> String {
    // get some random data:
    let mut data: Vec<u8> = Vec::new();
    data.resize(len, 0);
    thread_rng().fill_bytes(&mut data);
    let rv = b2h(&data);
    assert_eq!(rv.len(), len * 2, "Unexpected random size");
    rv
}

pub fn rand_hex32() -> String {
    // get some random data:
    let mut data = [0u8; 32];
    thread_rng().fill_bytes(&mut data);
    let rv = b2h(&data);
    assert_eq!(rv.len(), 64, "Unexpected random size");
    rv
}

#[allow(dead_code)]
pub fn private_to_address(sk: &[u8]) -> Vec<u8> {
    let s = Secp256k1::new();
    let key = SecretKey::from_slice(sk).unwrap();
    let pub_key = PublicKey::from_secret_key(&s, &key);
    public_to_address(&pub_key.serialize_uncompressed()[1..])
}

fn ecdsa_sign(hash: &[u8], private_key: &[u8]) -> EcdsaSig {
    let s = Secp256k1::signing_only();
    let msg = Message::from_slice(hash).unwrap();
    let key = SecretKey::from_slice(private_key).unwrap();
    let (v, sig_bytes) = s.sign_recoverable(&msg, &key).serialize_compact();

    EcdsaSig {
        v: v.to_i32() as u64, // + chain_id * 2 + 35,
        r: sig_bytes[0..32].to_vec(),
        s: sig_bytes[32..64].to_vec(),
    }
}

pub fn ecdsa_recover(hash: &[u8], sig: &EcdsaSig) -> Result<Vec<u8>, secp256k1::Error> {
    let s = Secp256k1::new();
    let msg = Message::from_slice(hash).unwrap();
    let mut sig_compact: Vec<u8> = sig.r.clone();
    sig_compact.extend(&sig.s);
    let sig_v = RecoveryId::from_i32(sig.v.clone() as i32).unwrap();
    let rec_sig = RecoverableSignature::from_compact(&sig_compact, sig_v);
    match rec_sig {
        Ok(r) => {
            match s.recover(&msg, &r) {
                Ok(pub_key) => {
                    let pk_bytes_raw: [u8; 65] = pub_key.serialize_uncompressed();
                    Ok(public_to_address(&pk_bytes_raw[1..]))
                }
                Err(e) => return Err(e),
            }
        }
        Err(e) => return Err(e),
    }
}

impl CryptoUtils {
    pub fn new() -> Self {
        return CryptoUtils {};
    }

    pub fn sign(&self, hash: &[u8], private_key: &[u8]) -> Vec<u8> {
        let sig = ecdsa_sign(hash, private_key);
        sig.to_u8()
    }

    pub fn recover(&self, hash: &[u8], sig: &[u8]) -> Vec<u8> {
        let sig_o = EcdsaSig::from(sig).unwrap();
        ecdsa_recover(hash, &sig_o).unwrap()
    }
}