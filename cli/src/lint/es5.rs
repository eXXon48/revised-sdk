//! ES5 check for HMI code (2012 Opera/Presto). A tokenizer, not a parser: it only has to
//! tell code from strings, comments and regex literals. Positions are char indexes.

fn id_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Index just past the string literal starting at `i` (stops at an unescaped newline).
fn skip_str(c: &[char], i: usize) -> usize {
    let q = c[i];
    let mut j = i + 1;
    while j < c.len() {
        match c[j] {
            '\\' => j += 2,
            '\n' => return j,
            x if x == q => return j + 1,
            _ => j += 1,
        }
    }
    c.len()
}

/// Index just past the regex literal starting at `i` (bails at a newline).
fn skip_regex(c: &[char], i: usize) -> usize {
    let (mut j, mut class) = (i + 1, false);
    while j < c.len() {
        match c[j] {
            '\\' => j += 2,
            '[' => {
                class = true;
                j += 1;
            }
            ']' => {
                class = false;
                j += 1;
            }
            '/' if !class => return j + 1,
            '\n' => return j,
            _ => j += 1,
        }
    }
    c.len()
}

fn next_sig(c: &[char], mut i: usize) -> char {
    while i < c.len() && c[i].is_whitespace() {
        i += 1;
    }
    c.get(i).copied().unwrap_or('\0')
}

pub fn scan(c: &[char]) -> Vec<(usize, &'static str)> {
    let n = c.len();
    let mut out = Vec::new();
    let (mut i, mut prev, mut prev_word) = (0, '\0', String::new());
    let (mut depth, mut for_parens, mut pending_for) = (0usize, Vec::<usize>::new(), false);
    while i < n {
        let ch = c[i];
        if ch.is_whitespace() || ch == '\u{feff}' {
            i += 1;
            continue;
        }
        let was_for = std::mem::replace(&mut pending_for, false);
        let at = |p: usize| c.get(p).copied().unwrap_or('\0');
        if ch == '/' && at(i + 1) == '/' {
            while i < n && c[i] != '\n' {
                i += 1;
            }
        } else if ch == '/' && at(i + 1) == '*' {
            i += 2;
            while i < n && !(c[i] == '*' && at(i + 1) == '/') {
                i += 1;
            }
            i += 2;
        } else if ch == '"' || ch == '\'' {
            i = skip_str(c, i);
            (prev, prev_word) = ('"', String::new());
        } else if ch == '`' {
            out.push((i, "template literal is not ES5: use string concatenation"));
            i += 1;
            while i < n && c[i] != '`' {
                i += if c[i] == '\\' { 2 } else { 1 };
            }
            i += 1;
            (prev, prev_word) = ('`', String::new());
        } else if ch == '/' {
            let regex = prev_word == "return" || (prev_word.is_empty() && "\0(,=:[!&|?{};+-*%<>~^".contains(prev));
            if regex {
                i = skip_regex(c, i);
                (prev, prev_word) = ('0', String::new());
            } else {
                (prev, prev_word, i) = ('/', String::new(), i + 1);
            }
        } else if id_char(ch) {
            let s = i;
            while i < n && id_char(c[i]) {
                i += 1;
            }
            let word: String = c[s..i].iter().collect();
            let nx = next_sig(c, i);
            let prop = prev == '.';
            let msg = match word.as_str() {
                "let" if !prop && nx != ':' => Some("`let` is not ES5: use `var`"),
                "const" if !prop && nx != ':' => Some("`const` is not ES5: use `var`"),
                "class" if !prop && nx != ':' => Some("`class` is not ES5: use a constructor function and prototype"),
                "await" if !prop && nx != ':' => Some("`await` is not ES5: use callbacks"),
                "async" if !prop && id_char(nx) => Some("`async` is not ES5: use callbacks"),
                // note: only `.reload(`; `location.href = ...` also navigates, add if it shows up
                "reload" if prop && nx == '(' => {
                    Some("`.reload()` of the HMI page reboots the unit ~1 min later: never reload; SafeReboot instead")
                }
                "Promise" if !prop => Some("`Promise` does not exist in the HMI browser: use callbacks"),
                "fetch" if !prop && nx == '(' => Some("`fetch()` does not exist in the HMI browser: use XMLHttpRequest"),
                "of" if !prop && for_parens.last() == Some(&depth) => {
                    Some("`for (... of ...)` is not ES5: use an indexed `for` loop")
                }
                _ => None,
            };
            if let Some(m) = msg {
                out.push((s, m));
            }
            pending_for = word == "for";
            (prev, prev_word) = ('a', word);
        } else if ch == '.' && at(i + 1) == '.' && at(i + 2) == '.' {
            out.push((i, "spread/rest `...` is not ES5: use `apply` / `arguments`"));
            (prev, prev_word, i) = ('.', String::new(), i + 3);
        } else if ch == '=' && at(i + 1) == '>' {
            out.push((i, "arrow function is not ES5: use `function () {}`"));
            (prev, prev_word, i) = ('>', String::new(), i + 2);
        } else {
            match ch {
                '(' => {
                    depth += 1;
                    if was_for {
                        for_parens.push(depth);
                    }
                }
                ')' => {
                    if for_parens.last() == Some(&depth) {
                        for_parens.pop();
                    }
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            (prev, prev_word, i) = (ch, String::new(), i + 1);
        }
    }
    out
}
