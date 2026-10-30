use zeroize::Zeroize;

const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{}:,.?";
const HEX: &[u8] = b"0123456789abcdef";
const URL_SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const AMBIGUOUS: &[u8] = b"O0Il1|";

pub const MIN_LENGTH: usize = 4;
pub const MAX_LENGTH: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Password,
    Pin,
    ApiKey,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ApiFormat {
    UrlSafe,
    Hex,
}

#[derive(Clone, Copy)]
pub struct Options {
    pub mode: Mode,
    pub length: usize,
    pub upper: bool,
    pub lower: bool,
    pub digits: bool,
    pub symbols: bool,
    pub avoid_ambiguous: bool,
    pub api_format: ApiFormat,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Password,
            length: 20,
            upper: true,
            lower: true,
            digits: true,
            symbols: true,
            avoid_ambiguous: false,
            api_format: ApiFormat::UrlSafe,
        }
    }
}

/// Buffered wrapper around the operating system's CSPRNG.
/// Fetches randomness in blocks instead of one system call per byte,
/// and wipes the buffer when dropped.
struct SecureRng {
    buf: [u8; 256],
    pos: usize,
}

impl SecureRng {
    fn new() -> Self {
        Self {
            buf: [0; 256],
            pos: 256,
        }
    }

    fn next_u32(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.buf.len() {
            getrandom::fill(&mut self.buf).map_err(|e| format!("Slumpkälla saknas: {e}"))?;
            self.pos = 0;
        }
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&self.buf[self.pos..self.pos + 4]);
        // Wipe consumed bytes so they can't be reused or read later.
        self.buf[self.pos..self.pos + 4].zeroize();
        self.pos += 4;
        let value = u32::from_le_bytes(bytes);
        bytes.zeroize();
        Ok(value)
    }

    /// Uniform index in `0..max` using rejection sampling (no modulo bias).
    /// Works for any `max` in `1..=u32::MAX`.
    fn index(&mut self, max: usize) -> Result<usize, String> {
        assert!(
            max > 0 && max as u64 <= u32::MAX as u64,
            "ogiltigt intervall"
        );
        let max = max as u64;
        let range = 1u64 << 32;
        let limit = range - (range % max);
        loop {
            let x = self.next_u32()? as u64;
            if x < limit {
                return Ok((x % max) as usize);
            }
        }
    }

    fn pick(&mut self, alphabet: &[u8]) -> Result<u8, String> {
        Ok(alphabet[self.index(alphabet.len())?])
    }
}

impl Drop for SecureRng {
    fn drop(&mut self) {
        self.buf.zeroize();
    }
}

fn password_groups(options: &Options) -> Vec<Vec<u8>> {
    let mut groups = Vec::new();
    for (enabled, alphabet) in [
        (options.upper, UPPER),
        (options.lower, LOWER),
        (options.digits, DIGITS),
        (options.symbols, SYMBOLS),
    ] {
        if enabled {
            let group: Vec<u8> = alphabet
                .iter()
                .copied()
                .filter(|c| !options.avoid_ambiguous || !AMBIGUOUS.contains(c))
                .collect();
            if !group.is_empty() {
                groups.push(group);
            }
        }
    }
    groups
}

pub fn generate(options: &Options) -> Result<String, String> {
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&options.length) {
        return Err(format!(
            "Längden måste vara mellan {MIN_LENGTH} och {MAX_LENGTH} tecken."
        ));
    }

    let groups = match options.mode {
        Mode::Password => password_groups(options),
        Mode::Pin => vec![DIGITS.to_vec()],
        Mode::ApiKey => vec![match options.api_format {
            ApiFormat::UrlSafe => URL_SAFE.to_vec(),
            ApiFormat::Hex => HEX.to_vec(),
        }],
    };
    if groups.is_empty() {
        return Err("Välj minst en teckengrupp.".into());
    }
    if options.length < groups.len() {
        return Err("Längden är kortare än antalet valda teckengrupper.".into());
    }

    let mut rng = SecureRng::new();
    let pool: Vec<u8> = groups.iter().flatten().copied().collect();
    let mut value = Vec::with_capacity(options.length);
    if options.mode == Mode::Password {
        for group in &groups {
            value.push(rng.pick(group)?);
        }
    }
    while value.len() < options.length {
        value.push(rng.pick(&pool)?);
    }
    // Fisher–Yates shuffle so the guaranteed characters end up in random positions.
    for i in (1..value.len()).rev() {
        let j = rng.index(i + 1)?;
        value.swap(i, j);
    }
    // All alphabets are ASCII, so this never fails; from_utf8 reuses the buffer.
    String::from_utf8(value).map_err(|e| {
        let mut bytes = e.into_bytes();
        bytes.zeroize();
        "Internt fel: ogiltiga tecken.".to_string()
    })
}

/// Estimated entropy in bits for the given options (approximate for passwords,
/// since forcing one character per group slightly reduces it).
pub fn entropy_bits(options: &Options) -> Option<f64> {
    let alphabet = match options.mode {
        Mode::Password => password_groups(options).iter().map(Vec::len).sum(),
        Mode::Pin => DIGITS.len(),
        Mode::ApiKey => match options.api_format {
            ApiFormat::UrlSafe => URL_SAFE.len(),
            ApiFormat::Hex => HEX.len(),
        },
    };
    if alphabet < 2 || !(MIN_LENGTH..=MAX_LENGTH).contains(&options.length) {
        return None;
    }
    Some(options.length as f64 * (alphabet as f64).log2())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_contains_every_selected_group() {
        let opts = Options::default();
        for _ in 0..50 {
            let result = generate(&opts).unwrap();
            assert_eq!(result.len(), 20);
            assert!(result.bytes().any(|c| UPPER.contains(&c)));
            assert!(result.bytes().any(|c| LOWER.contains(&c)));
            assert!(result.bytes().any(|c| DIGITS.contains(&c)));
            assert!(result.bytes().any(|c| SYMBOLS.contains(&c)));
        }
    }

    #[test]
    fn single_group_and_ambiguous_filter() {
        let opts = Options {
            upper: true,
            lower: false,
            digits: false,
            symbols: false,
            avoid_ambiguous: true,
            ..Options::default()
        };
        let result = generate(&opts).unwrap();
        assert!(
            result
                .bytes()
                .all(|c| UPPER.contains(&c) && !AMBIGUOUS.contains(&c))
        );
    }

    #[test]
    fn ambiguous_filter_with_all_groups() {
        let opts = Options {
            avoid_ambiguous: true,
            length: MAX_LENGTH,
            ..Options::default()
        };
        for _ in 0..20 {
            let result = generate(&opts).unwrap();
            assert!(result.bytes().all(|c| !AMBIGUOUS.contains(&c)));
        }
    }

    #[test]
    fn pin_and_api_formats() {
        let mut opts = Options {
            mode: Mode::Pin,
            length: 32,
            ..Options::default()
        };
        assert!(
            generate(&opts)
                .unwrap()
                .bytes()
                .all(|c| DIGITS.contains(&c))
        );
        opts.mode = Mode::ApiKey;
        for format in [ApiFormat::Hex, ApiFormat::UrlSafe] {
            opts.api_format = format;
            let result = generate(&opts).unwrap();
            let alphabet = if format == ApiFormat::Hex {
                HEX
            } else {
                URL_SAFE
            };
            assert_eq!(result.len(), 32);
            assert!(result.bytes().all(|c| alphabet.contains(&c)));
        }
    }

    #[test]
    fn rejects_empty_selection() {
        let opts = Options {
            upper: false,
            lower: false,
            digits: false,
            symbols: false,
            ..Options::default()
        };
        assert!(generate(&opts).is_err());
    }

    #[test]
    fn length_boundaries_for_all_modes() {
        for mode in [Mode::Password, Mode::Pin, Mode::ApiKey] {
            for length in [MIN_LENGTH, MAX_LENGTH] {
                let opts = Options {
                    mode,
                    length,
                    ..Options::default()
                };
                assert_eq!(generate(&opts).unwrap().len(), length);
            }
            for length in [0, MIN_LENGTH - 1, MAX_LENGTH + 1] {
                let opts = Options {
                    mode,
                    length,
                    ..Options::default()
                };
                assert!(generate(&opts).is_err());
            }
        }
    }

    #[test]
    fn index_handles_large_ranges() {
        // Previously, ranges above 256 caused an infinite loop.
        let mut rng = SecureRng::new();
        for max in [1, 2, 255, 256, 257, 1000, 65_537, u32::MAX as usize] {
            for _ in 0..100 {
                assert!(rng.index(max).unwrap() < max);
            }
        }
    }

    #[test]
    fn distribution_is_roughly_uniform() {
        let mut rng = SecureRng::new();
        let buckets = 10;
        let samples = 100_000;
        let mut counts = vec![0usize; buckets];
        for _ in 0..samples {
            counts[rng.index(buckets).unwrap()] += 1;
        }
        let expected = samples / buckets;
        for count in counts {
            // Allow ±5 %, far beyond normal statistical variation (~1 %).
            assert!(count.abs_diff(expected) < expected / 20, "count={count}");
        }
    }

    #[test]
    fn entropy_estimate() {
        let opts = Options {
            mode: Mode::ApiKey,
            api_format: ApiFormat::Hex,
            length: 32,
            ..Options::default()
        };
        assert_eq!(entropy_bits(&opts), Some(128.0));
    }
}
