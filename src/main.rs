use clap::Parser;
use std::env;
use serde_json::{json, Value};
use std::io::Write;
use tokio::io::{self as async_io, AsyncReadExt};
use tokio_stream::{self, StreamExt};

/*
    repeat_last_n = 64, repeat_penalty = 1.100, frequency_penalty = 0.000, presence_penalty = 0.000
    top_k = 40, tfs_z = 1.000, top_p = 0.950, min_p = 0.050, typical_p = 1.000, temp = 0.800
    mirostat = 0, mirostat_lr = 0.100, mirostat_ent = 5.000
*/

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// the model name
    #[clap(short, long, default_value = "default")]
    model: String,

    /// the endpoint, taken from the environment variable QLLM_ENDPOINT if not specified
    #[clap(short, long, required = false, default_value = "")]
    endpoint: String,

    /// the api key, which is taken from the environment variable QLLM_KEY if not specified
    #[clap(short, long, required = false, default_value = "")]
    key: String,

    /// the system prompt
    #[clap(short, long, required = false, default_value = "Help the user with their task.")]
    system: String,

    /// flag to say if we should read from stdin, use -c as the single character version
    #[clap(short = 'c', long)]
    stdin: bool,

    /// no instruction prompt, just continuation of input
    #[clap(short, long)]
    no_instruct: bool,

    /// the positional argument is the user prompt
    #[clap(name = "PROMPT", required = true)]
    prompt: Vec<String>,

    /// copy full prompt to the output, to make the output suitable for recursive use
    #[clap(short, long)]
    recurse: bool,

    /// context length
    #[clap(short = 'l', long, default_value = "-1")]
    max_tokens: i64,

    /// the temperature parameter for the model
    #[clap(short, long, default_value = "0.8")]
    temperature: f64,

    /// the top_p parameter for the model
    #[clap(long, default_value = "0.95")]
    top_p: f64,

    /// the min_p parameter for the model
    #[clap(long, default_value = "0.05")]
    min_p: f64,

    /// the top_k parameter for the model
    #[clap(long, default_value = "40")]
    top_k: usize,

    /// the repetition penalty for the model
    #[clap(long, default_value = "1.1")]
    repetition_penalty: f64,

    /// the token set to consider for repetition penalty
    #[clap(long, default_value = "64")]
    repetition_penalty_last: usize,

    /// the presence penalty for the model
    #[clap(long, default_value = "0.0")]
    presence_penalty: f64,

    /// the frequency penalty for the model
    #[clap(long, default_value = "0.0")]
    frequency_penalty: f64,

    /// the typical p parameter for the model
    #[clap(long, default_value = "1.0")]
    typical_p: f64,

    /// the mirostat mode for the model
    #[clap(long, default_value = "0")]
    mirostat_mode: u8,

    /// the mirostat tau parameter for the model
    #[clap(long, default_value = "5.0")]
    mirostat_tau: f64,

    /// the mirostat eta parameter for the model
    #[clap(long, default_value = "0.1")]
    mirostat_eta: f64
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let endpoint = if !args.endpoint.is_empty() {
        args.endpoint.clone()
    } else if env::var_os("QLLM_ENDPOINT").is_some() {
        std::env::var("QLLM_ENDPOINT")?
    } else {
        return Err("No endpoint specified. One must be given on the command line via -e or via the environmental variable QLLM_ENDPOINT.".into());
    };

    // set a key if we have one in the environment under QLLM_KEY
    let key = if !args.key.is_empty() {
        Some(args.key.clone())
    } else if env::var_os("QLLM_KEY").is_some() {
        Some(std::env::var("QLLM_KEY")?)
    } else {
        None
    };

    // Check for stdin data using select
    let mut stdin = async_io::stdin();
    let mut input = String::new();
    // if we read from stdin
    if args.stdin {
        // read from stdin
        stdin.read_to_string(&mut input).await?;
    }

    let mut user_prompt = args.prompt.join(" ");
    if !input.is_empty() {
        user_prompt = format!("{}\n{}", input, user_prompt);
    } else {
        user_prompt = user_prompt.to_string();
    }

    let client = reqwest::Client::new();
    let models = json!({
        "messages": [
            { "role": "system", "content": args.system },
            { "role": "user", "content": user_prompt },
        ],
        "max_tokens": args.max_tokens,
        "temperature": args.temperature,
        "top_p": args.top_p,
        "top_k": args.top_k,
        "min_p": args.min_p,
        "repetition_penalty": args.repetition_penalty,
        "repetition_penalty_last": args.repetition_penalty_last,
        "presence_penalty": args.presence_penalty,
        "frequency_penalty": args.frequency_penalty,
        "typical_p": args.typical_p,
        "mirostat_mode": args.mirostat_mode,
        "mirostat_tau": args.mirostat_tau,
        "mirostat_eta": args.mirostat_eta,
        "stream": true
    });

    let response = client.post(&endpoint)
        .header("Content-Type", "application/json")
        .bearer_auth(key.unwrap_or_default())
        .json(&models)
        .send()
        .await?;

    let mut stream = response.bytes_stream();
    let mut first = true;

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                let line = String::from_utf8_lossy(&bytes).trim().to_string();
                if line == "data: [DONE]" {
                    break;
                }
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                        if let Some(text) = parsed["choices"][0]["delta"]["content"].as_str() {
                            let mut text = text;
                            if first {
                                // trim the leading space from the first response
                                text = text.trim_start();
                                first = false;
                            }
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
