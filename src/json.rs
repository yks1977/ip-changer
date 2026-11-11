//! settings.json 과 PowerShell 출력을 읽고 쓰기 위한 작은 JSON 구현.
//! (외부 크레이트 없이 필요한 만큼만)

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    /// 키 순서를 유지한다.
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn parse(text: &str) -> Option<Json> {
        let text = text.trim_start_matches('\u{feff}');
        let mut parser = Parser {
            s: text.as_bytes(),
            i: 0,
        };
        parser.ws();
        let value = parser.value(0)?;
        parser.ws();
        (parser.i == parser.s.len()).then_some(value)
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// 문자열 값. 숫자는 문자열로 바꾸고, 없거나 null이면 빈 문자열.
    pub fn str_of(&self, key: &str) -> String {
        match self.get(key) {
            Some(Json::Str(s)) => s.clone(),
            Some(Json::Num(n)) => format_num(*n),
            _ => String::new(),
        }
    }

    pub fn bool_of(&self, key: &str, default: bool) -> bool {
        match self.get(key) {
            Some(Json::Bool(b)) => *b,
            _ => default,
        }
    }

    /// 배열 값. 값이 하나뿐이라 배열이 아닌 값으로 온 경우(PowerShell 5.1)도 배열로 본다.
    pub fn list_of(&self, key: &str) -> Vec<Json> {
        match self.get(key) {
            Some(Json::Arr(items)) => items.clone(),
            Some(Json::Null) | None => Vec::new(),
            Some(other) => vec![other.clone()],
        }
    }

    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => out.push_str(&format_num(*n)),
            Json::Str(s) => write_str(out, s),
            Json::Arr(items) if items.is_empty() => out.push_str("[]"),
            Json::Obj(items) if items.is_empty() => out.push_str("{}"),
            Json::Arr(items) => {
                out.push_str("[\n");
                for (index, item) in items.iter().enumerate() {
                    push_indent(out, indent + 2);
                    item.write(out, indent + 2);
                    out.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
                }
                push_indent(out, indent);
                out.push(']');
            }
            Json::Obj(items) => {
                out.push_str("{\n");
                for (index, (key, value)) in items.iter().enumerate() {
                    push_indent(out, indent + 2);
                    write_str(out, key);
                    out.push_str(": ");
                    value.write(out, indent + 2);
                    out.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
                }
                push_indent(out, indent);
                out.push('}');
            }
        }
    }
}

fn push_indent(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat(' ').take(indent));
}

fn format_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Option<Json> {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Some(value)
        } else {
            None
        }
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        if depth > 64 {
            return None;
        }
        match self.peek()? {
            b'n' => self.literal("null", Json::Null),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'"' => self.string().map(Json::Str),
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    return Some(Json::Arr(items));
                }
                loop {
                    self.ws();
                    items.push(self.value(depth + 1)?);
                    self.ws();
                    match self.peek()? {
                        b',' => self.i += 1,
                        b']' => {
                            self.i += 1;
                            return Some(Json::Arr(items));
                        }
                        _ => return None,
                    }
                }
            }
            b'{' => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    return Some(Json::Obj(items));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    self.ws();
                    if self.peek()? != b':' {
                        return None;
                    }
                    self.i += 1;
                    self.ws();
                    let value = self.value(depth + 1)?;
                    items.push((key, value));
                    self.ws();
                    match self.peek()? {
                        b',' => self.i += 1,
                        b'}' => {
                            self.i += 1;
                            return Some(Json::Obj(items));
                        }
                        _ => return None,
                    }
                }
            }
            _ => self.number(),
        }
    }

    fn number(&mut self) -> Option<Json> {
        let start = self.i;
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.i += 1;
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).ok()?;
        text.parse::<f64>().ok().map(Json::Num)
    }

    fn hex4(&mut self) -> Option<u32> {
        let text = std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
        self.i += 4;
        u32::from_str_radix(text, 16).ok()
    }

    fn string(&mut self) -> Option<String> {
        if self.peek()? != b'"' {
            return None;
        }
        self.i += 1;
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            let b = self.peek()?;
            self.i += 1;
            match b {
                b'"' => return String::from_utf8(bytes).ok(),
                b'\\' => {
                    let esc = self.peek()?;
                    self.i += 1;
                    let ch = match esc {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let mut code = self.hex4()?;
                            if (0xD800..0xDC00).contains(&code)
                                && self.s.get(self.i..self.i + 2) == Some(b"\\u")
                            {
                                self.i += 2;
                                let low = self.hex4()?;
                                code = 0x10000 + ((code - 0xD800) << 10) + (low.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            char::from_u32(code).unwrap_or('\u{fffd}')
                        }
                        _ => return None,
                    };
                    let mut buf = [0u8; 4];
                    bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                _ => bytes.push(b),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Json;

    #[test]
    fn parses_and_round_trips() {
        let text = r#"{"a": "한글 \"q\" A😀", "b": [1, 2.5, true, null], "c": {}, "d": []}"#;
        let value = Json::parse(text).unwrap();
        assert_eq!(value.str_of("a"), "한글 \"q\" A😀");
        assert_eq!(
            value.get("b"),
            Some(&Json::Arr(vec![
                Json::Num(1.0),
                Json::Num(2.5),
                Json::Bool(true),
                Json::Null
            ]))
        );
        let again = Json::parse(&value.to_pretty()).unwrap();
        assert_eq!(again, value);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Json::parse("").is_none());
        assert!(Json::parse("not json").is_none());
        assert!(Json::parse("{\"a\":1").is_none());
        assert!(Json::parse("[1,]").is_none());
        assert!(Json::parse("{} trailing").is_none());
    }

    #[test]
    fn single_value_is_treated_as_list() {
        let value = Json::parse(r#"{"one": "8.8.8.8", "many": ["1.1.1.1", "8.8.8.8"], "none": null}"#)
            .unwrap();
        assert_eq!(value.list_of("one"), vec![Json::Str("8.8.8.8".into())]);
        assert_eq!(value.list_of("many").len(), 2);
        assert!(value.list_of("none").is_empty());
        assert!(value.list_of("missing").is_empty());
    }

    #[test]
    fn pretty_output_is_stable() {
        let value = Json::Obj(vec![
            ("name".into(), Json::Str("a\\b".into())),
            ("list".into(), Json::Arr(vec![Json::Num(3.0)])),
        ]);
        assert_eq!(
            value.to_pretty(),
            "{\n  \"name\": \"a\\\\b\",\n  \"list\": [\n    3\n  ]\n}"
        );
    }
}
