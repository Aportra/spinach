# spinach

A local RAG (Retrieval-Augmented Generation) CLI built in Rust with a Python chat interface. The core embedding, similarity search, and retrieval logic is written in Rust and exposed to Python via [PyO3](https://github.com/PyO3/pyo3), with [Ollama](https://ollama.ai) powering local inference.

## Architecture

```
spinach/
├── src/
│   ├── main.rs       # CLI entry point (clap subcommands)
│   ├── lib.rs        # Rust core — embeddings, RAG, news, search (PyO3)
│   └── util.rs       # Generic vector math trait (argmax, cosine sim, normalize)
└── model.py          # Python chat loop — calls Rust functions via spinach module
```

The Rust library compiles to a native Python extension (`spinach`). The Python layer handles the conversational loop and command parsing; Rust handles everything performance-sensitive — file chunking, embedding via [fastembed](https://github.com/Anush008/fastembed-rs), cosine similarity, and HTTP requests.

## Features

- **Local RAG over files** — chunk and embed any file or directory, retrieve semantically relevant context at query time
- **Dynamic file references** — register file paths by alias for quick lookup
- **News retrieval** — pull top headlines or keyword-filtered articles from [NewsAPI](https://newsapi.org) sources
- **Web search** — query Google via [Serper](https://serper.dev) and inject results into context
- **Streaming inference** — responses stream token-by-token via Ollama
- **Persistent conversation history** — full message history maintained within a session
- **Multi-line input** — paste large blocks of text using `+++` / `END` delimiters

## Requirements

- [Rust](https://rustup.rs/) (stable)
- [Python 3.10+](https://www.python.org/)
- [Ollama](https://ollama.ai) running locally
- [Maturin](https://github.com/PyO3/maturin) (for building the PyO3 extension)
- NewsAPI key (optional — required for `news` commands)
- Serper API key (optional — required for `search` command)

## Installation

```bash
# Clone the repo
git clone https://github.com/Aportra/spinach
cd spinach

# Create and activate a Python virtual environment
python -m venv ai
source ai/bin/activate

# Install Python dependencies
pip install ollama pyyaml

# Build and install the Rust extension
pip install maturin
maturin develop --release

# Copy and configure
cp config-default.yaml config.yaml
# Edit config.yaml with your model, API keys, and chunking preferences
```

## Configuration

`config.yaml` controls all runtime behavior:

```yaml
model: mistral          # Any model available in your local Ollama instance
chunk_size: 100         # Lines per chunk when embedding files
overlap: 50             # Stride between chunks (controls overlap)
news_api_key: ""        # NewsAPI key (newsapi.org)
search_api: ""          # Serper API key (serper.dev)
news_sources: "associated-press,reuters,the-wall-street-journal,financial-times"
```

## Usage

### Build the data index

```bash
# Chunk and embed a file or directory into the data store
spinach create <path> <label>

# Register a file path by alias (for quick lookup)
spinach add <path>
spinach add --folder <subfolder> <path>
```

### Start a chat session

```bash
spinach run
```

### Chat commands

| Command | Description |
|---|---|
| `>> your question` | Standard chat with the configured Ollama model |
| `look <path>` | Load a file into context, then ask a question |
| `look dyn <alias>` | Load a dynamically registered file by alias |
| `look data <label>` | Retrieve semantically relevant chunk from an indexed dataset |
| `news` | Summarize today's top headlines from configured sources |
| `news <source>` | Headlines from a specific source (e.g. `news reuters`) |
| `news <source> <n>` | Top `n` headlines from a source |
| `news search <source> <query>` | Search a source for a specific topic |
| `search <query>` | Google search via Serper, results injected into context |
| `reset` | Clear conversation history |
| `quit` / `bye` | Exit |

### Multi-line input

For pasting large blocks of text (code, documents, etc.):

```
>> +++
paste your content here
across multiple lines
END
```

## How RAG Works in Spinach

1. **Indexing** (`spinach create`) — files are chunked by line count with configurable overlap, each chunk is embedded using `fastembed` (BGE small model), and stored as JSON with the embedding vector.
2. **Retrieval** (`look data`) — at query time, the prompt is embedded and cosine similarity is computed against all stored chunk vectors. The highest-scoring chunk is injected into the context window before inference.
3. **Inference** — Ollama streams the response token-by-token with full conversation history passed on every turn.

Cosine similarity and vector operations (`argmax`, `normalize`, `dot`) are implemented from scratch in `util.rs` as a generic trait over `f32`/`f64`.

## Available News Sources

```
abc-news, associated-press, axios, bloomberg, breitbart-news,
business-insider, cbs-news, cnbc, cnn, espn, financial-times,
fox-news, fox-sports, msnbc, nbc-news, nbc-sports, politico,
reuters, techcrunch, the-hill, the-wall-street-journal,
the-washington-post, usa-today
```
