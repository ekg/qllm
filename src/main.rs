use clap::Parser;
use std::env;
use serde_json::{json, Value};
use std::io::Write;
use tokio::io::{self as async_io, AsyncReadExt};
use tokio_stream::{self, StreamExt};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {

    /// the model name
    #[clap(short, long, default_value = "default")]
    model: String,

    /// the endpoint, taken from the environment variable QLLM_ENDPOINT if not specified
    #[clap(short, long, required = false, default_value = "")]
    endpoint: String,

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

    ///// the model context length
    //#[clap(short = 'x', long, default_value = "200000")]
    //max_tokens: usize,

    /// context length
    #[clap(short = 'l', long, default_value = "65536")]
    max_tokens: usize,

    /// the temperature parameter for the model
    #[clap(short, long, default_value = "0.7")]
    temperature: f64,

    /// the top_p parameter for the model
    #[clap(long, default_value = "0.9")]
    top_p: f64,

    /// the min_p parameter for the model
    #[clap(long, default_value = "0.0")]
    min_p: f64,

    /// the top_k parameter for the model
    #[clap(long, default_value = "20")]
    top_k: usize,

    /// the repetition penalty for the model
    #[clap(long, default_value = "1.15")]
    repetition_penalty: f64,

    /// the presence penalty for the model
    #[clap(long, default_value = "0.0")]
    presence_penalty: f64,

    /// the frequency penalty for the model
    #[clap(long, default_value = "0.0")]
    frequency_penalty: f64,

    /// the repetition penalty range for the model
    #[clap(long, default_value = "0.0")]
    repetition_penalty_range: f64,

    /// the typical p parameter for the model
    #[clap(long, default_value = "1.0")]
    typical_p: f64,

    /// the guidance scale for the model
    #[clap(long, default_value = "1.0")]
    guidance_scale: f64,

    /// the penalty alpha parameter for the model
    #[clap(long, default_value = "0.0")]
    penalty_alpha: f64,

    /// the mirostat mode for the model
    #[clap(long, default_value = "0")]
    mirostat_mode: u8,

    /// the mirostat tau parameter for the model
    #[clap(long, default_value = "5")]
    mirostat_tau: f64,

    /// the mirostat eta parameter for the model
    #[clap(long, default_value = "0.1")]
    mirostat_eta: f64,

    /// whether the temperature should be applied to the last token
    #[clap(long)]
    temperature_last: bool,

    /// whether the model should do sampling
    #[clap(long)]
    do_sample: bool
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

    // Check for stdin data using select
    let mut stdin = async_io::stdin();
    let mut input = String::new();
    // if we read from stdin
    if args.stdin {
        // read from stdin
        stdin.read_to_string(&mut input).await?;
    }

    // format should be like this 
    //"SYSTEM: You are ... USER: Do ... ASSISTANT:"
    let system_prompt = format!("SYSTEM: {}", args.system);
    let user_prompt_prefix = "USER: ";
    let mut user_prompt = args.prompt.join(" ");
    if !input.is_empty() {
        user_prompt = format!("{}\n{}", input, user_prompt);
    } else {
        user_prompt = format!("{}", user_prompt);
    }
    let assistant_prompt_prefix = "ASSISTANT: ";
    let prompt = if args.no_instruct {
        user_prompt
    } else {
        format!(
            "{}\n{}{}\n{}",
            system_prompt,
            user_prompt_prefix,
            user_prompt,
            assistant_prompt_prefix)
    };
    if args.recurse {
        print!("{}", prompt);
    }

    let client = reqwest::Client::new();
    let models = json!({
        "model": args.model,
        "prompt": prompt,
        "stream": true,
        // unclear why we have this limit
        "max_tokens": args.max_tokens,
        //"max_new_tokens": args.max_new_tokens,
        "temperature": args.temperature,
        "top_p": args.top_p,
        "top_k": args.top_k,
        "min_p": args.min_p,
        "repetition_penalty": args.repetition_penalty,
        "presence_penalty": args.presence_penalty,
        "frequency_penalty": args.frequency_penalty,
        "repetition_penalty_range": args.repetition_penalty_range,
        "typical_p": args.typical_p,
        "guidance_scale": args.guidance_scale,
        "penalty_alpha": args.penalty_alpha,
        "mirostat_mode": args.mirostat_mode,
        "mirostat_tau": args.mirostat_tau,
        "mirostat_eta": args.mirostat_eta,
        "temperature_last": args.temperature_last,
        "do_sample": args.do_sample
    });

    let response = client.post(&endpoint)
        .header("Content-Type", "application/json")
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
                        if let Some(text) = parsed["choices"][0]["text"].as_str() {
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
