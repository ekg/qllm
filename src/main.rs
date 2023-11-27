use reqwest;
use serde_json::{json, Value};
use tokio;
use tokio_stream::{self, StreamExt};

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let api_endpoint = "http://localhost:7000/v1/completions";
    let model = "brucethemoose/Capybara-Tess-Yi-34B-200K-DARE-Ties";
    let prompt = "SYSTEM: You are an assistant. USER: Hello! Write me a long form poem. ASSISTANT:";

    let client = reqwest::Client::new();
    let payload = json!({
        "model": model,
        "prompt": prompt,
        "max_tokens": 10000,
        "stream": true
    });

    let response = client.post(api_endpoint)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                let line = String::from_utf8_lossy(&bytes);
                if line.trim() == "data: [DONE]" {
                    break;
                }

                if let Some(json_str) = line.trim().strip_prefix("data: ") {
                    if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                        if let Some(text) = parsed["choices"][0]["text"].as_str() {
                            print!("{}", text);
                        }
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
