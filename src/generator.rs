const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{}:,.?";
const HEX: &[u8] = b"0123456789abcdef";
const URL_SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

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

fn random_index(max: usize) -> Result<usize, String> {
    // Rejection sampling prevents modulo bias for every supported alphabet.
    let limit = 256 - (256 % max);
    loop {
        let mut byte = [0u8; 1];
        getrandom::fill(&mut byte).map_err(|e| format!("Slumpkälla saknas: {e}"))?;
        if (byte[0] as usize) < limit {
            return Ok((byte[0] as usize) % max);
        }
    }
}

fn pick(alphabet: &[u8]) -> Result<u8, String> {
    Ok(alphabet[random_index(alphabet.len())?])
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
                .filter(|c| !options.avoid_ambiguous || !b"O0Il1|".contains(c))
                .collect();
            groups.push(group);
        }
    }
    groups
}

pub fn generate(options: &Options) -> Result<String, String> {
    if !(4..=256).contains(&options.length) {
        return Err("Längden måste vara mellan 4 och 256 tecken.".into());
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

    let pool: Vec<u8> = groups.iter().flatten().copied().collect();
    let mut value = Vec::with_capacity(options.length);
    if options.mode == Mode::Password {
        for group in &groups {
            value.push(pick(group)?);
        }
    }
    while value.len() < options.length {
        value.push(pick(&pool)?);
    }
    for i in (1..value.len()).rev() {
        let j = random_index(i + 1)?;
        value.swap(i, j);
    }
    String::from_utf8(value).map_err(|e| e.to_string())
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
                .all(|c| UPPER.contains(&c) && !b"O0Il1|".contains(&c))
        );
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
}
