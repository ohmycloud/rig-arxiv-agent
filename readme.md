# arxiv agent

How to run?

```bash
RUSTLOG=info cargo run
```

please refer to [arxiv-rig-rust](https://www.shuttle.dev/blog/2025/01/08/arxiv-rig-rust)

note: You need a OpenAI API key.

```bash
export OPENAI_BASE_URL="https://api.apiyi.com/v1"
export OPENAI_API_KEY="xxxxxxxxxxxxxxxxxxxxxxxxx"
```

And, the prompt must tell the LLM to return a raw JSON string without any markdown label.
