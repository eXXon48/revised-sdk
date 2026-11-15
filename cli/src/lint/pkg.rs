//! Package-level checks: ws literal ports, HMI register key.
use crate::report::Finding;

/// Char indexes of every occurrence of `needle`.
pub fn occurrences(c: &[char], needle: &str) -> Vec<usize> {
    let n: Vec<char> = needle.chars().collect();
    (0..c.len()).filter(|&i| c[i..].starts_with(&n)).collect()
}

/// `ws://127.0.0.1:<digits>` or `ws://localhost:<digits>`.
pub fn ws_literals(c: &[char]) -> Vec<usize> {
    let mut v: Vec<usize> = ["ws://127.0.0.1:", "ws://localhost:"]
        .iter()
        .flat_map(|p| occurrences(c, p).into_iter().map(move |i| (i, p.len())))
        .filter(|(i, l)| c.get(i + l).is_some_and(|x| x.is_ascii_digit()))
        .map(|(i, _)| i)
        .collect();
    v.sort();
    v
}

/// True when the source has `RevisedHMI.register("<name>/`.
pub fn registers(c: &[char], name: &str) -> bool {
    let key: Vec<char> = name.chars().chain(['/']).collect();
    occurrences(c, "RevisedHMI.register(").into_iter().any(|i| {
        let mut j = i + "RevisedHMI.register(".len();
        while c.get(j).is_some_and(|x| x.is_whitespace()) {
            j += 1;
        }
        matches!(c.get(j), Some('"' | '\'')) && c[j + 1..].starts_with(&key)
    })
}

pub fn no_register(name: &str) -> Finding {
    Finding::error(
        "register-key",
        &format!("no `RevisedHMI.register(\"{name}/...\")` call in ui/**/*.js: the register key must start with \"{name}/\""),
    )
}
