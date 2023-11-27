# qllm

`qllm` is a command-line tool for interacting with local Large Language Models (LLMs) using a simple and user-friendly interface.
It is designed to work with local LLMs running on platforms like [vllm](https://github.com/vllm-project/vllm).

## Installation

To install `qllm`, clone the repository and run:

```bash
cargo install --path .
```

## Usage

```bash
qllm [args]
```

### Arguments

- `-h`, `--help`: Display help information.
- `-v`, `--version`: Display the version number.
- `-a`, `--author`: Display the author of the program.
- `-m`, `--model`: Set the model to use, e.g., `brucethemoose/Capybara-Tess-Yi-34B-200K-DARE-Ties`.
- `-e`, `--endpoint`: Set the API endpoint, e.g., `http://localhost:7000/v1/completions`.
- `-s`, `--system`: Set the system prompt, e.g., "Help the user with their task.".
- `-d`, `--debug`: Display debug information.
- `-c`, `--stdin`: Read from stdin.
- `PROMPT`: The positional argument is the user prompt.

## Example

```bash
qllm -m brucethemoose/Capybara-Tess-Yi-34B-200K-DARE-Ties -e http://localhost:7000/v1/completions -s "Please type in your task." "What is the sum of 10 and 20?"
```

This will send the user-provided prompt "What is the sum of 10 and 20?" to the specified model endpoint, and display the model's response on stdout.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.</s>
