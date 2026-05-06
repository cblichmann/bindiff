use std::sync::OnceLock;

static PRIMES: OnceLock<Vec<u32>> = OnceLock::new();

fn get_primes() -> &'static [u32] {
    PRIMES.get_or_init(|| generate_primes(256))
}

fn generate_primes(n: usize) -> Vec<u32> {
    let mut primes = Vec::with_capacity(n);
    let mut candidate = 2;
    while primes.len() < n {
        if is_prime(candidate) {
            primes.push(candidate);
        }
        candidate += 1;
    }
    primes
}

fn is_prime(n: u32) -> bool {
    if n <= 1 {
        return false;
    }
    for i in 2..=((n as f64).sqrt() as u32) {
        if n % i == 0 {
            return false;
        }
    }
    true
}

pub fn ipow32(mut base: u32, mut exp: u32) -> u32 {
    let mut res: u32 = 1;
    while exp > 0 {
        if exp % 2 == 1 {
            res = res.wrapping_mul(base);
        }
        base = base.wrapping_mul(base);
        exp /= 2;
    }
    res
}

pub fn get_prime(mnemonic: &str) -> u32 {
    const ASCII_SPACE: u8 = 32;
    let mut id: u32 = 1;
    let primes = get_primes();

    for (i, c) in mnemonic.chars().enumerate() {
        let c_val = c as u32;
        if c_val <= ASCII_SPACE as u32 {
            continue;
        }
        let idx = (c_val - ASCII_SPACE as u32) as usize;
        if idx < primes.len() {
            id = id.wrapping_mul(ipow32(primes[idx], (i + 1) as u32));
        } else {
            // Fallback for out-of-range characters, though unlikely for standard mnemonics
            id = id.wrapping_mul(ipow32(2, (i + 1) as u32));
        }
    }
    id
}
