//! `&`-coded messages (`&c{player} has combat logged!`) and MiniMessage colour tags
//! (`<red>Player not found`) as coloured segments.

const COLOURS: [(char, u32); 16] = [
    ('0', 0x000000),
    ('1', 0x0000AA),
    ('2', 0x00AA00),
    ('3', 0x00AAAA),
    ('4', 0xAA0000),
    ('5', 0xAA00AA),
    ('6', 0xFFAA00),
    ('7', 0xAAAAAA),
    ('8', 0x555555),
    ('9', 0x5555FF),
    ('a', 0x55FF55),
    ('b', 0x55FFFF),
    ('c', 0xFF5555),
    ('d', 0xFF55FF),
    ('e', 0xFFFF55),
    ('f', 0xFFFFFF),
];

/// MiniMessage's named colours, as the `&` codes above.
const NAMED: [(&str, char); 16] = [
    ("black", '0'),
    ("dark_blue", '1'),
    ("dark_green", '2'),
    ("dark_aqua", '3'),
    ("dark_red", '4'),
    ("dark_purple", '5'),
    ("gold", '6'),
    ("gray", '7'),
    ("dark_gray", '8'),
    ("blue", '9'),
    ("green", 'a'),
    ("aqua", 'b'),
    ("red", 'c'),
    ("light_purple", 'd'),
    ("yellow", 'e'),
    ("white", 'f'),
];

/// The colour a `<tag>` at the start of `rest` sets, and its length: a named colour, or white
/// for `<reset>` and any closing tag of a named colour. None for text that only looks like a
/// tag (`<player>` stays as it is).
fn tag(rest: &str) -> Option<(u32, usize)> {
    let end = rest.find('>')?;
    let name = rest[1..end].to_ascii_lowercase();
    let len = end + 1;
    if name == "reset" {
        return Some((0xFFFFFF, len));
    }
    let (closing, name) = name
        .strip_prefix('/')
        .map_or((false, name.as_str()), |n| (true, n));
    let (_, code) = NAMED.iter().find(|(n, _)| *n == name)?;
    if closing {
        return Some((0xFFFFFF, len));
    }
    COLOURS
        .iter()
        .find(|(k, _)| k == code)
        .map(|(_, rgb)| (*rgb, len))
}

/// Splits `template` at its colour codes into `(text, colour)` segments; `&r` and any other
/// code resets to white. Formatting codes (`&l`...) are dropped, they are not ported.
pub fn segments(template: &str) -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = Vec::new();
    let mut colour = 0xFFFFFF;
    let mut text = String::new();
    let mut chars = template.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if c == '<'
            && let Some((rgb, len)) = tag(&template[at..])
        {
            if !text.is_empty() {
                out.push((std::mem::take(&mut text), colour));
            }
            colour = rgb;
            while chars.peek().is_some_and(|(i, _)| *i < at + len) {
                chars.next();
            }
            continue;
        }
        let code = (c == '&').then(|| chars.peek().map(|(_, k)| *k)).flatten();
        let Some(code) = code
            .filter(|k| matches!(k.to_ascii_lowercase(), '0'..='9' | 'a'..='f' | 'k'..='o' | 'r'))
        else {
            text.push(c);
            continue;
        };
        chars.next();
        if !text.is_empty() {
            out.push((std::mem::take(&mut text), colour));
        }
        colour = COLOURS
            .iter()
            .find(|(k, _)| *k == code.to_ascii_lowercase())
            .map_or(0xFFFFFF, |(_, rgb)| *rgb);
    }
    if !text.is_empty() {
        out.push((text, colour));
    }
    out
}

/// `template` with `{key}` replaced by each `(key, value)`.
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = template.to_owned();
    for (key, value) in values {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_code_colours_the_line() {
        assert_eq!(segments("&c14/15"), vec![("14/15".to_string(), 0xFF5555)]);
    }

    #[test]
    fn codes_split_and_reset() {
        assert_eq!(
            segments("a&6b&rc"),
            vec![
                ("a".to_string(), 0xFFFFFF),
                ("b".to_string(), 0xFFAA00),
                ("c".to_string(), 0xFFFFFF)
            ]
        );
    }

    #[test]
    fn a_lone_ampersand_stays() {
        assert_eq!(
            segments("R&X & more"),
            vec![("R&X & more".to_string(), 0xFFFFFF)]
        );
    }

    #[test]
    fn minimessage_colour_tags_colour_and_unknown_tags_stay() {
        assert_eq!(
            segments("<red>Player not found: <white>{player}"),
            vec![
                ("Player not found: ".to_string(), 0xFF5555),
                ("{player}".to_string(), 0xFFFFFF)
            ]
        );
        assert_eq!(
            segments("<gray>/x status <player>"),
            vec![("/x status <player>".to_string(), 0xAAAAAA)]
        );
        assert_eq!(
            segments("<GOLD>a</gold>b"),
            vec![("a".to_string(), 0xFFAA00), ("b".to_string(), 0xFFFFFF)]
        );
    }

    #[test]
    fn placeholders_are_replaced() {
        assert_eq!(
            fill(
                "{timeLeft}/{maxTime}",
                &[("timeLeft", "9"), ("maxTime", "15")]
            ),
            "9/15"
        );
    }
}
