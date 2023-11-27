use clap::Parser;
use reqwest;
use serde_json::{json, Value};
use tokio;
use std::io::Write;
use tokio::io::{self as async_io, AsyncReadExt};
use tokio_stream::{self, StreamExt};

/// Your CLI description here.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets the model to use
    #[clap(short, long, default_value = "brucethemoose/Capybara-Tess-Yi-34B-200K-DARE-Ties")]
    model: String,

    /// Sets the API endpoint
    #[clap(short, long, default_value = "http://localhost:7000/v1/completions")]
    endpoint: String,

    /// Sets the system prompt
    #[clap(short, long, required = false, default_value = "Help the user with their task.")]
    system: String,

    /// Flag to say if we should read from stdin, use -c as the single character version
    #[clap(short = 'c', long)]
    stdin: bool,

    /// The positional argument is the user prompt
    #[clap(name = "PROMPT", required = true)]
    prompt: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Check for stdin data using select
    let mut stdin = async_io::stdin();
    let mut input = String::new();
    // if we read from stdin
    if args.stdin {
        // read from stdin
        stdin.read_to_string(&mut input).await?;
    }

    // format should be like this 
    //"SYSTEM: You are an assistant. Do exactly as the user asks. Write very long replies. They must be at least 1000 words, even to trivial requests. USER: Hello! How are you today? Write me an essay. ASSISTANT:"
    let system_prompt = format!("SYSTEM: {}", args.system);
    let user_prompt_prefix = "USER:";
    let mut user_prompt = args.prompt.join(" ");
    if !input.is_empty() {
        user_prompt = format!("{} {}\n{}", user_prompt_prefix, user_prompt, input);
    }
    let assistant_prompt_prefix = "ASSISTANT:";
    let prompt = format!(
        "{}\n{}\n{}",
        system_prompt,
        user_prompt,
        assistant_prompt_prefix
    );
    println!("Prompt: {}", prompt);
    
    let client = reqwest::Client::new();
    let payload = json!({
        "model": args.model,
        "prompt": prompt,
        "max_tokens": 10000,
        "stream": true
    });

    let response = client.post(&args.endpoint)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                let line = String::from_utf8_lossy(&bytes).trim().to_string();
                if line == "data: [DONE]" {
                    break;
                }
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                        if let Some(text) = parsed["choices"][0]["text"].as_str() {
                            print!("{}", text);
                            // flush stdout to make sure the text is visible immediately
                            std::io::stdout().flush().unwrap();
                        }
                    }
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

