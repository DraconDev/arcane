use regex::Regex;

fn main() {
    let patterns = vec![
        (
            "Private Key Variable (Unquoted)",
            r#"(?i)[A-Z0-9_]*PRIVATE_KEY[A-Z0-9_]*=[A-Za-z0-9_-]{20,}"#,
        ),
        (
            "Generic Secret (Unquoted)",
            r#"(?i)(?:secret|token|password|passwd|pwd|credential).{0,10}=[^\s"']{16,}"#,
        ),
    ];

    let input = "-   STRIPE_PRIVATE_KEY=ANSKDFN13N141212311123123asdasdBA";
    // Also test without the diff markers just in case
    let input_clean = "STRIPE_PRIVATE_KEY=ANSKDFN13N141212311123123asdasdBA";

    println!("Testing input: '{}'", input);

    for (name, pattern) in &patterns {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(input) {
            println!("✅ Matches '{}'", name);
        } else {
            println!("❌ Failed '{}'", name);
        }

        if re.is_match(input_clean) {
            println!("✅ Matches clean '{}'", name);
        }
    }
}
