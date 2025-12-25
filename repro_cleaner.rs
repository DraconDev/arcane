fn main() {
    let raw = r#"Here's a concise and descriptive commit message for this change:

```
Update test commit message with emphasis

- Added emphasis to test commit message with "!!!"
- No functional changes, purely cosmetic update
```"#;

    let cleaned = clean_response(raw);
    println!("--- CLEANED ---");
    println!("{}", cleaned);
    println!("---------------");

    if cleaned.starts_with("Here") {
        println!("FAIL: Preamble not stripped");
    } else {
        println!("PASS: Preamble stripped");
    }
}

fn clean_response(raw: &str) -> String {
    let mut text = raw.trim().to_string();

    // 1. Hunt for Explicit Delimiters (Highest Priority)
    let delimiters = ["ARCANE_COMMIT:", "SECURITY_ALERT:"];
    for d in delimiters {
        if let Some(idx) = text.to_lowercase().find(&d.to_lowercase()) {
            let after_tag = &text[idx + d.len()..];
            return after_tag.trim().to_string();
        }
    }

    // 2. Remove Markdown code blocks if present
    if let Some(start) = text.find("```") {
        if let Some(end) = text[start + 3..].find("```") {
            let content = &text[start + 3..start + 3 + end];
            let lines: Vec<&str> = content.lines().collect();
            // If the first line is exactly a language name (no spaces), skip it
            if lines.len() > 1 && !lines[0].contains(' ') {
                text = lines[1..].join("\n");
            } else {
                text = content.to_string();
            }
        } else {
            text = text.replace("```", "");
        }
    }

    // 3. Strip legacy prefixes
    let protocols = ["COMMIT_MESSAGE:", "MESSAGE:"];
    for p in protocols {
        if let Some(idx) = text.to_lowercase().find(&p.to_lowercase()) {
            text = text[idx + p.len()..].trim().to_string();
            break;
        }
    }

    // 4. Conventional Commit Header Scan
    let common_types = [
        "feat", "fix", "docs", "style", "refactor", "perf", "test", "chore", "build", "ci",
        "revert",
    ];
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let lower = line.trim().to_lowercase();
        for t in common_types {
            if lower.starts_with(&format!("{}:", t)) || lower.starts_with(&format!("{}(", t)) {
                return lines[i..].join("\n").trim().to_string();
            }
        }
    }

    // 5. Final fallback
    text.trim().trim_matches('"').trim_matches('\'').to_string()
}
