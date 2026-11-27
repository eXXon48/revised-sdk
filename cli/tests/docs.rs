//! Docs: `docs/agents/*.md` is the single source of the human guide and of the `AGENTS.md` template.
//! After editing a source file run: `cli/t.sh env REVISED_DOCS_UPDATE=1 cargo test --test docs`.
use revised_sdk::new::new_project;
use std::path::{Path, PathBuf};

const PARTS: [&str; 5] = ["rules", "api", "loop", "baseline", "skins"];
const AGENTS_HEAD: &str = "# AGENTS.md\n\nRules for coding agents that work on this Revised HMI app. Read every section before you change code.\n\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn parts() -> String {
    PARTS
        .iter()
        .map(|p| read(&format!("docs/agents/{p}.md")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn expected() -> [(&'static str, String); 2] {
    [
        (
            "templates/app/AGENTS.md",
            format!("{AGENTS_HEAD}{}", parts()),
        ),
        (
            "docs/guide.md",
            format!("{}\n{}", read("docs/guide-intro.md"), parts()),
        ),
    ]
}

#[test]
fn generated_docs_in_sync() {
    for (rel, want) in expected() {
        if std::env::var_os("REVISED_DOCS_UPDATE").is_some() {
            std::fs::write(root().join(rel), &want).unwrap();
        }
        assert!(
            read(rel) == want,
            "{rel} is stale: run cli/t.sh env REVISED_DOCS_UPDATE=1 cargo test --test docs"
        );
    }
}

#[test]
fn agents_md_has_sections() {
    let a = read("templates/app/AGENTS.md");
    for h in ["## Rules", "## API", "## Loop", "## Stock baseline"] {
        assert!(a.lines().any(|l| l == h), "missing heading {h}");
    }
    assert_eq!(read("templates/app/CLAUDE.md").trim(), "Read AGENTS.md.");
}

/// Every rule id the CLI emits (`cli/src`, lint included) is documented in `docs/agents/rules.md`.
#[test]
fn emitted_rules_documented() {
    let rules = read("docs/agents/rules.md");
    let mut ids = Vec::new();
    let mut files = Vec::new();
    for d in ["cli/src", "cli/src/lint"] {
        for e in std::fs::read_dir(root().join(d)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "rs") {
                files.push(p);
            }
        }
    }
    for f in files {
        let src = std::fs::read_to_string(f).unwrap();
        // the constructors, plus build's `err(` and new's `fail(` helpers
        for pat in ["Finding::error(", "Finding::warning(", "Finding {", "err(", "fail("] {
            for (i, _) in src.match_indices(pat) {
                let rest = src[i + pat.len()..].trim_start();
                if let Some(r) = rest.strip_prefix('"') {
                    ids.push(r.split('"').next().unwrap().to_string());
                }
            }
        }
        // ids that sit in tuples or on a line of their own
        for id in ["register-key", "baseline-missing", "baseline-mismatch"] {
            if src.contains(&format!("\"{id}\"")) {
                ids.push(id.to_string());
            }
        }
    }
    ids.sort();
    ids.dedup();
    assert!(ids.len() >= 20, "id scan found too little: {ids:?}");
    for id in ids {
        assert!(
            rules.contains(&format!("`{id}`")),
            "rule {id} is not documented in docs/agents/rules.md"
        );
    }
}

/// The docs name `CMU_UP_PASS` and never contain its value. The value is never printed.
#[test]
fn no_password_literal() {
    let mut text = parts()
        + &read("docs/guide-intro.md")
        + &read("docs/developer-sdk.md")
        + &read("README.md");
    for f in ["README.md", "mod.yaml"] {
        text += &read(&format!("templates/app/{f}"));
    }
    assert!(text.contains("CMU_UP_PASS"));
    if let Ok(pw) = std::env::var("CMU_UP_PASS") {
        if pw.len() >= 8 {
            let mut all = text;
            for (rel, _) in expected() {
                all += &read(rel);
            }
            assert!(!all.contains(&pw), "a doc contains the CMU_UP_PASS value");
            assert!(
                !pw.get(..8).is_some_and(|p| all.contains(p)),
                "a doc contains a prefix of the CMU_UP_PASS value"
            );
        }
    }
}

#[test]
fn docs_have_no_local_paths_or_ips() {
    let (a, g) = (read("templates/app/AGENTS.md"), read("docs/guide.md"));
    for t in [a, g, read("README.md")] {
        for bad in ["W:/", "C:/Users", r"C:\Users", "192.168.", "Claude"] {
            assert!(!t.contains(bad), "{bad:?} in docs");
        }
    }
}

#[test]
fn new_writes_agents_and_claude() {
    let base = std::env::temp_dir().join(format!("rsdk-docs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let p = new_project(&base, "app", "dd").unwrap();
    let a = std::fs::read_to_string(p.join("AGENTS.md")).unwrap();
    assert!(a.contains("## Stock baseline") && a.contains("CMU_UP_PASS"));
    assert_eq!(
        std::fs::read_to_string(p.join("CLAUDE.md")).unwrap().trim(),
        "Read AGENTS.md."
    );
}

/// Every `revised-sdk <command>` the guide uses has a row in its Commands table,
/// and the guide has a Skins section.
#[test]
fn guide_documents_every_command_it_uses() {
    const CMDS: [&str; 9] = ["setup", "opera", "new", "build", "lint", "baseline", "preview", "push", "unit"];
    const WITH_SUB: [&str; 4] = ["opera", "baseline", "preview", "unit"];
    let g = read("docs/guide.md");
    assert!(g.lines().any(|l| l == "## Skins"), "no ## Skins section");
    for (i, _) in g.match_indices("revised-sdk ") {
        let words: Vec<&str> = g[i + 12..]
            .split(|c: char| c.is_whitespace() || c == '`')
            .filter(|w| !w.is_empty() && !w.starts_with("--"))
            .take(2)
            .collect();
        let Some(&cmd) = words.first().filter(|w| CMDS.contains(w)) else { continue };
        let used = match words.get(1) {
            Some(sub) if WITH_SUB.contains(&cmd) => format!("{cmd} {sub}"),
            _ => cmd.to_string(),
        };
        assert!(g.contains(&format!("| `{used}")), "guide uses `revised-sdk {used}` but has no Commands row for it");
    }
}
