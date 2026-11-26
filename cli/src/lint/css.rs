//! Presto CSS check: text scan over comment-stripped, lowercased CSS. Positions are char indexes.

fn ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn starts(c: &[char], i: usize, s: &str) -> bool {
    s.chars().enumerate().all(|(k, x)| c.get(i + k) == Some(&x))
}

fn skip_ws(c: &[char], mut j: usize) -> usize {
    while j < c.len() && c[j].is_whitespace() {
        j += 1;
    }
    j
}

pub fn scan(src: &[char]) -> Vec<(usize, String)> {
    // lowercase + blank comments, same length so indexes stay valid
    let mut c: Vec<char> = src.iter().map(|x| x.to_ascii_lowercase()).collect();
    let mut i = 0;
    while i < c.len() {
        if starts(&c, i, "/*") {
            while i < c.len() && !starts(&c, i, "*/") {
                if c[i] != '\n' {
                    c[i] = ' ';
                }
                i += 1;
            }
            for k in i..(i + 2).min(c.len()) {
                c[k] = ' ';
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    let mut out = Vec::new();
    for i in 0..c.len() {
        if i > 0 && ident(c[i - 1]) {
            continue;
        }
        for (pat, msg) in [
            ("var(", "`var()` is not supported by Presto: write the value out"),
            ("calc(", "`calc()` is not supported by Presto: use fixed values"),
            ("clip-path", "`clip-path` is not supported by Presto"),
            ("-webkit-", "`-webkit-` properties are not supported by Presto"),
        ] {
            if starts(&c, i, pat) {
                out.push((i, msg.to_string()));
            }
        }
        if starts(&c, i, "display") {
            let j = skip_ws(&c, i + 7);
            if c.get(j) == Some(&':') {
                let j = skip_ws(&c, j + 1);
                let v: String = c[j..].iter().take_while(|x| ident(**x)).collect();
                if ["flex", "inline-flex", "grid", "inline-grid"].contains(&v.as_str()) {
                    out.push((i, format!("`display: {v}` is not supported by Presto: use block, inline-block or float")));
                } else if v.starts_with("table") || v == "inline-table" {
                    // Presto lays these out, but text in cells built after load is not repainted
                    out.push((i, format!("`display: {v}` does not repaint reliably in the HMI: use inline-block with vertical-align")));
                }
            }
        }
    }
    out
}

/// `style="..."` attribute values in HTML: (start, end) char ranges of the value.
pub fn style_attrs(c: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let low: Vec<char> = c.iter().map(|x| x.to_ascii_lowercase()).collect();
    for i in 1..c.len() {
        if !starts(&low, i, "style") || !c[i - 1].is_whitespace() {
            continue;
        }
        let j = skip_ws(c, i + 5);
        if c.get(j) != Some(&'=') {
            continue;
        }
        let j = skip_ws(c, j + 1);
        let Some(&q) = c.get(j).filter(|q| **q == '"' || **q == '\'') else { continue };
        let end = c[j + 1..].iter().position(|x| *x == q).map_or(c.len(), |p| j + 1 + p);
        out.push((j + 1, end));
    }
    out
}
