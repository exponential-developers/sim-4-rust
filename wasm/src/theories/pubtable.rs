/*!
 * Publication tables: sim-3's coded tables (`ptDecodeFormat1` / `ptDecodeFormat2` in
 * `src/Utils/ptDecode.ts`), copied unchanged from sim-3 into `data/`.
 *
 * A coded table is a base64 string of zlib-compressed big-endian numbers, each stored as the
 * difference to the previous one.
 */

use std::collections::HashMap;

#[derive(serde::Deserialize)]
pub struct CodedTable {
    /** base64 of the compressed numbers */
    t: String,
    /** Table step (format 1) or first key (format 2) */
    s: f64,
    /** Bytes per number */
    sz: usize,
    /** Added to every number */
    d: i64,
    /** Initial value */
    i: i64,
    /** Format 1: key at which the step changes */
    #[serde(default)]
    b: Option<String>,
    /** Format 1: the new step */
    #[serde(default)]
    s2: Option<f64>,
}

impl CodedTable {
    pub fn parse(json: &str) -> CodedTable {
        serde_json::from_str(json).expect("coded pub table")
    }
}

pub fn base64_decode(s: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for &c in s.as_bytes().iter().take_while(|&&c| c != b'=') {
        let v = ALPHABET
            .iter()
            .position(|&a| a == c)
            .expect("base64 character") as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/** The decoded numbers: `data[i]` is the sum of the first i + 1 differences (plus `d` each) */
pub fn decode_values(coded: &CodedTable) -> Vec<i64> {
    let bytes = miniz_oxide::inflate::decompress_to_vec_zlib(&base64_decode(&coded.t))
        .expect("pub table data");
    let mut prev = coded.i;
    bytes
        .chunks(coded.sz)
        .map(|chunk| {
            // JavaScript builds the number with 32-bit shifts: `res = (res << 8) | byte`
            let n = chunk
                .iter()
                .fold(0i32, |acc, &b| acc.wrapping_shl(8) | b as i32);
            prev += n as i64 + coded.d;
            prev
        })
        .collect()
}

/**
 * JavaScript `Number.prototype.toFixed`: rounds the exact decimal value half up. The next digit
 * after the ones kept decides (5 or more rounds up, as the value is then at or above the half).
 */
pub fn js_to_fixed(x: f64, digits: usize) -> String {
    // exact: doubles of this size have at most about 75 fractional digits
    let exact = format!("{:.100}", x.abs());
    let (int_part, frac) = exact.split_once('.').expect("decimal point");
    let mut kept: Vec<u8> = int_part.bytes().chain(frac.bytes().take(digits)).collect();
    if frac.as_bytes()[digits] >= b'5' {
        // add one unit in the last place
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    let int_len = kept.len() - digits;
    let (int_digits, frac_digits) = kept.split_at(int_len);
    let mut out = String::new();
    // JavaScript prints the sign unless the rounded result is zero
    if x < 0. && kept.iter().any(|&b| b != b'0') {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(int_digits).unwrap());
    if digits > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(frac_digits).unwrap());
    }
    out
}

/** JavaScript `Math.round`: rounds halves up */
pub fn js_round(x: f64) -> f64 {
    let floor = x.floor();
    if x - floor >= 0.5 { floor + 1. } else { floor }
}

/**
 * A format 1 table (sim-3 `ptDecodeFormat1` followed by `prepareTable`): from the last pub rho
 * to the rho to publish at.
 */
pub struct PubTable {
    /** Keys are written with 4 decimals, like the keys sim-3 looks up with `prepareTable` */
    entries: HashMap<String, f64>,
    /** Decimals of the keys sim-3 decodes (before `prepareTable` adds zeros) */
    digits: usize,
}

impl PubTable {
    pub fn decode_format1(coded: &CodedTable) -> PubTable {
        Self::decode(coded, None)
    }

    /**
     * A format 1 table prepared with a suffix other than zeros (sim-3 `prepareTable(table,
     * suffix)`): the suffix is appended to both the keys and the values, so it shifts the target.
     */
    pub fn decode_format1_with_suffix(coded: &CodedTable, suffix: &str) -> PubTable {
        Self::decode(coded, Some(suffix))
    }

    fn decode(coded: &CodedTable, suffix: Option<&str>) -> PubTable {
        let data = decode_values(coded);
        let mut step = coded.s;
        // Align the results to step
        let digits = if step >= 0.1
            && js_to_fixed(step, 1).parse::<f64>() == js_to_fixed(step, 2).parse::<f64>()
        {
            1
        } else {
            2
        };

        let mut rev_keys: Vec<String> = Vec::with_capacity(data.len());
        let mut pre_table: Vec<(String, i64)> = Vec::with_capacity(data.len());
        let mut cur_key = 0.0f64;
        for (i, &value) in data.iter().enumerate() {
            let key = js_to_fixed(cur_key, digits);
            rev_keys.push(key.clone());
            // the last entry is set to 0 to match the original tables
            let stored = if i == data.len() - 1 { 0 } else { value };
            pre_table.push((key.clone(), stored));
            if coded.b.as_deref() == Some(key.as_str())
                && let Some(s2) = coded.s2
            {
                step = s2;
            }
            cur_key += step;
        }

        let pad = suffix.map_or_else(|| "0".repeat(4 - digits), str::to_owned);
        let mut entries = HashMap::with_capacity(pre_table.len());
        for (key, index) in pre_table {
            // rev_keys[index] is undefined (NaN) for an index outside the table
            let target = usize::try_from(index)
                .ok()
                .and_then(|i| rev_keys.get(i))
                .map_or(f64::NAN, |k| {
                    let value = match suffix {
                        Some(suffix) => format!("{k}{suffix}"),
                        None => k.clone(),
                    };
                    value.parse::<f64>().unwrap_or(f64::NAN)
                });
            entries.insert(format!("{key}{pad}"), target);
        }
        PubTable { entries, digits }
    }

    /** The entry for a key exactly as written, e.g. `"12.3404"` */
    pub fn get(&self, key: &str) -> Option<f64> {
        self.entries.get(key).copied()
    }

    /** `target`, but None when the table has no entry */
    pub fn lookup(&self, last_pub_rho: f64, mult: f64) -> Option<f64> {
        let key = js_to_fixed(js_round(last_pub_rho * mult) / mult, 4);
        self.entries.get(&key).copied()
    }

    /**
     * The entry for a key written like sim-3 reads the decoded table directly (no `prepareTable`),
     * e.g. `"123.25"`: the key must have the table's number of decimals. NaN if there is none.
     */
    pub fn get_exact(&self, key: &str) -> f64 {
        let decimals = key.split_once('.').map_or(0, |(_, frac)| frac.len());
        if decimals != self.digits {
            return f64::NAN;
        }
        let padded = format!("{key}{}", "0".repeat(4 - decimals));
        self.entries.get(&padded).copied().unwrap_or(f64::NAN)
    }

    /**
     * Rho to publish at for the given last pub rho, looked up like sim-3:
     * `table[(Math.round(rho * mult) / mult).toFixed(4)]`. NaN if there is no entry.
     */
    pub fn target(&self, last_pub_rho: f64, mult: f64) -> f64 {
        let key = js_to_fixed(js_round(last_pub_rho * mult) / mult, 4);
        self.entries.get(&key).copied().unwrap_or(f64::NAN)
    }
}

/** A format 2 table (sim-3 `ptDecodeFormat2`): numbers with consecutive integer keys */
pub struct IntTable {
    first_key: i64,
    values: Vec<i64>,
}

impl IntTable {
    pub fn decode_format2(coded: &CodedTable) -> IntTable {
        IntTable {
            first_key: coded.s as i64,
            values: decode_values(coded),
        }
    }

    pub fn get(&self, key: i64) -> Option<i64> {
        usize::try_from(key - self.first_key)
            .ok()
            .and_then(|i| self.values.get(i).copied())
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_rounds_half_up_on_the_exact_value() {
        assert_eq!(js_to_fixed(0., 1), "0.0");
        assert_eq!(js_to_fixed(0.3, 1), "0.3");
        assert_eq!(js_to_fixed(0.1 + 0.2, 1), "0.3");
        assert_eq!(js_to_fixed(123.4, 4), "123.4000");
        assert_eq!(js_to_fixed(2.5, 0), "3"); // JavaScript: (2.5).toFixed(0) is "3"
        assert_eq!(js_to_fixed(0.125, 2), "0.13");
        assert_eq!(js_to_fixed(1.005, 2), "1.00"); // the double is a bit below 1.005
        assert_eq!(js_to_fixed(9.96, 1), "10.0");
        assert_eq!(js_to_fixed(99.99, 1), "100.0");
        assert_eq!(js_to_fixed(-0.04, 1), "0.0");
        assert_eq!(js_to_fixed(-1.25, 1), "-1.3");
    }

    #[test]
    fn js_round_rounds_halves_up() {
        assert_eq!(js_round(2.5), 3.);
        assert_eq!(js_round(-2.5), -2.);
        assert_eq!(js_round(2.4), 2.);
    }

    #[test]
    fn base64_decodes() {
        assert_eq!(base64_decode("TWFu"), b"Man");
        assert_eq!(base64_decode("TWE="), b"Ma");
        assert_eq!(base64_decode("TQ=="), b"M");
    }

    #[test]
    fn int_table_looks_up_by_key() {
        let table = IntTable {
            first_key: 10,
            values: vec![5, 6, 7],
        };
        assert_eq!((table.get(10), table.get(12)), (Some(5), Some(7)));
        assert_eq!((table.get(9), table.get(13)), (None, None));
        assert_eq!(table.len(), 3);
        assert!(!table.is_empty());
    }
}
