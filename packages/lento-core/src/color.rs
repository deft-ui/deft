use skia_safe::Color;

pub fn parse_hex_color(value: &str) -> Option<Color> {
    let mut chars = value.chars();
    let mut hex_chars = ['F'; 8];
    if value.len() == 3 || value.len() == 4 {
        for i in 0..value.len() {
            let c = chars.next().unwrap();
            hex_chars[i * 2] = c;
            hex_chars[i * 2 + 1] = c;
        }
    } else if value.len() == 6 || value.len() == 8 {
        for i in 0..value.len() {
            hex_chars[i] = chars.next().unwrap();
        }
    } else {
        return None;
    }
    let r = hex2u8([hex_chars[0], hex_chars[1]])?;
    let g = hex2u8([hex_chars[2], hex_chars[3]])?;
    let b = hex2u8([hex_chars[4], hex_chars[5]])?;
    let a = hex2u8([hex_chars[6], hex_chars[7]])?;
    Some(Color::from_argb(a, r, g, b))
}

fn hex2u8(hex: [char; 2]) -> Option<u8> {
    let hi = hex[0].to_digit(16)?;
    let lo = hex[1].to_digit(16)?;
    Some((hi * 16 + lo) as u8)
}
