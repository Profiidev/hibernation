/// https://github.com/kolloch/nix-base32/blob/master/src/lib.rs
const BASE32_CHARS: &[u8] = b"0123456789abcdfghijklmnpqrsvwxyz";

pub fn to_nix_base32(bytes: &[u8]) -> String {
  let len = (bytes.len() * 8 - 1) / 5 + 1;

  (0..len)
    .rev()
    .map(|n| {
      let b: usize = n * 5;
      let i: usize = b / 8;
      let j: usize = b % 8;
      // bits from the lower byte
      let v1 = bytes[i].checked_shr(j as u32).unwrap_or(0);
      // bits from the upper byte
      let v2 = if i >= bytes.len() - 1 {
        0
      } else {
        bytes[i + 1].checked_shl(8 - j as u32).unwrap_or(0)
      };
      let v: usize = (v1 | v2) as usize;
      char::from(BASE32_CHARS[v % BASE32_CHARS.len()])
    })
    .collect()
}
