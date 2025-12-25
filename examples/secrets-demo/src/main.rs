use axum::{routing::get, Router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // 1. Try to load from .env (if present and decrypted)
    let _ = dotenvy::dotenv();

    println!("🔒 Secrets Demo Starting...");

    // 2. Collect all secrets
    let secrets = vec![
        ("API_KEY", std::env::var("API_KEY").ok()),
        ("STRIPE_SECRET_KEY", std::env::var("STRIPE_SECRET_KEY").ok()),
        ("OPENAI_API_KEY", std::env::var("OPENAI_API_KEY").ok()),
        ("DATABASE_URL", std::env::var("DATABASE_URL").ok()),
        ("REDIS_URL", std::env::var("REDIS_URL").ok()),
        ("AWS_ACCESS_KEY_ID", std::env::var("AWS_ACCESS_KEY_ID").ok()),
        (
            "AWS_SECRET_ACCESS_KEY",
            std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
        ),
        ("JWT_SECRET", std::env::var("JWT_SECRET").ok()),
        ("GITHUB_TOKEN", std::env::var("GITHUB_TOKEN").ok()),
        ("SENDGRID_API_KEY", std::env::var("SENDGRID_API_KEY").ok()),
        ("TWILIO_AUTH_TOKEN", std::env::var("TWILIO_AUTH_TOKEN").ok()),
        ("ADMIN_PASSWORD", std::env::var("ADMIN_PASSWORD").ok()),
    ];

    let loaded_count = secrets.iter().filter(|(_, v)| v.is_some()).count();
    println!("✅ Loaded {}/{} secrets", loaded_count, secrets.len());

    // 3. Start Server
    let app = Router::new().route(
        "/",
        get(move || {
            let secrets_clone = secrets.clone();
            async move {
                let mut output = String::from("🔐 Arcane Secrets Demo\n");
                output.push_str("=".repeat(50).as_str());
                output.push_str("\n\n");

                let loaded = secrets_clone.iter().filter(|(_, v)| v.is_some()).count();
                let total = secrets_clone.len();
                output.push_str(&format!(
                    "📊 Status: {}/{} secrets loaded\n\n",
                    loaded, total
                ));

                for (name, value) in &secrets_clone {
                    let status = if value.is_some() { "✅" } else { "❌" };
                    let display_val = value
                        .as_ref()
                        .map(|v| {
                            // Mask middle of secret for security display
                            if v.len() > 10 {
                                format!("{}...{}", &v[..5], &v[v.len() - 5..])
                            } else {
                                v.clone()
                            }
                        })
                        .unwrap_or_else(|| "NOT_FOUND".to_string());
                    output.push_str(&format!("{} {}: {}\n", status, name, display_val));
                }

                output.push_str("\n");
                output.push_str("=".repeat(50).as_str());
                output.push_str("\n\nIf you see ✅ with masked values, decryption worked!\n");
                output.push_str("All secrets loaded from encrypted .env via arcane run.\n");

                output
            }
        }),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], 5122));
    println!("🚀 Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
