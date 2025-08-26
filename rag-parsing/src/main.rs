use anyhow::{anyhow, bail, Result};
use clap::{Args, Parser, Subcommand};
use dirs::home_dir;
use fastembed::TextEmbedding;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use serde_yaml::Value;
use std::fs::{create_dir_all, read_to_string, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "spinach")]
#[derive(Debug)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}
#[derive(Serialize)]
struct Chunks {
    chunk_id: String,
    content: String,
    chunks: Vec<f32>,
}
#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Create tokenized file from a file or an entire directory")]
    Create {
        path: PathBuf,
        file_type: String,
    },
    Run,
    Add {
        #[arg(short, long)]
        folder: Option<String>,
        path: PathBuf,
    },
}
//syntax should be spinach create <name-of-file> from <path>
fn chunk_file(path: &Path, file_type: &String) -> Result<()> {
    if !path.exists() {
        eprintln!("Invalid Path");
        bail!("Invalid Path");
    }

    let config_path = format!("{}/config.yaml", env!("CARGO_MANIFEST_DIR"));
    let mut yaml = File::open(config_path)?;
    let mut content = String::new();
    yaml.read_to_string(&mut content)?;

    let config: Value = serde_yaml::from_str(&content).expect("error");

    let overlap = config.get("overlap").and_then(|a| a.as_u64()).unwrap() as usize;

    let chunk = config.get("chunk_size").and_then(|a| a.as_u64()).unwrap() as usize;

    let all_files: Vec<_> = WalkDir::new(path)
        .into_iter()
        .filter_map(|f| match f {
            Ok(e)
                if e.file_type().is_file()
                    && !e.path().to_string_lossy().contains(".git")
                    && !e.path().to_string_lossy().contains(".json")
                    && !e.file_name().to_string_lossy().contains(".jpg")
                    && !e.file_name().to_string_lossy().contains(".png")
                    && !e.file_name().to_string_lossy().contains(".pyc")
                    && !e.file_name().to_string_lossy().contains(".pkl") =>
            {
                Some(e)
            }
            _ => None,
        })
        .collect();

    let home = home_dir();
    let file_path = format!("spinach-rag/data/{}", file_type);

    let directory = home
        .clone()
        .ok_or_else(|| anyhow!("Home Directory not available"))?
        .join(file_path);

    match create_dir_all(&directory) {
        Ok(_) => {
            println!("Directory Created: {}", &directory.display());
        }
        Err(error) => {
            println!("{}", error);
        }
    };

    println!("{:?}", all_files);

    for f in all_files {
        let mut file_name = Path::new(&f.file_name()).to_path_buf();

        if file_name.extension().unwrap().to_str() == Some("pdf") {
            println!("file is pdf");
        }

        let mut file_name_str = file_name.to_string_lossy().to_string();
        println!("{:?}", file_name_str);
        file_name = PathBuf::from(file_name_str);

        let full_path = directory.join(file_name);
        create_dir_all(&full_path)?;
        let path_chunk: &str = f.path().to_str().unwrap();
        let mut contents = Vec::new();
        File::open(path_chunk)
            .map_err(|e| {
                eprintln!("failed to open file,{}", e);
                anyhow!("failed to open file,{}", e)
            })?
            .read_to_end(&mut contents)
            .map_err(|e| {
                eprintln!("failed to read file,{}", e);
                anyhow!("failed to read file,{}", e)
            })?;

        let final_content = String::from_utf8_lossy(&contents).to_string();
        let words: Vec<String> = final_content
            .split_whitespace()
            .map(|e| e.to_string())
            .collect();
        let mut start = 0;
        let chunk_size = chunk;
        let stride = overlap;
        let mut chunk_id = 0;
        let model = TextEmbedding::try_new(Default::default())?;

        while start < words.len() {
            let end = usize::min(start + chunk_size, words.len());
            let chunk_text = words[start..end].join(" ");

            let embedding = model.embed(vec![chunk_text.clone()], None)?.pop().unwrap();

            let file_json = Chunks {
                chunk_id: format!("chunk_{:03}", &chunk_id).to_string(),
                content: chunk_text,
                chunks: embedding,
            };

            let json_string = serde_json::to_string_pretty(&file_json)?;
            let mut new_file =
                File::create(&full_path.join(format!("chunk_{:03}.json", &chunk_id)))?;
            new_file.write_all(json_string.as_bytes())?;

            start += stride;
            chunk_id += 1;
        }
    }
    Ok(())
}

fn run_chat() -> Result<()> {
    let path_script = home_dir().unwrap().join("spinach/model.py");
    let python_path = home_dir().unwrap().join("spinach/ai/bin/python3");
    let status = Command::new(python_path)
        .arg(path_script)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    std::process::exit(status.code().unwrap_or(1));
    Ok(())
}

fn add(path: &PathBuf, folder: Option<String>) -> Result<()> {
    let file_path = &path.to_string_lossy();
    let file_name = &path.file_name().unwrap().to_string_lossy();

    let home = home_dir().unwrap().to_owned();

    let full_path = match folder.as_ref() {
        Some(folder) => home.join(format!("spinach-rag/dynamic/{folder}/")),

        _ => home.join("spinach-rag/dynamic/"),
    };

    create_dir_all(&full_path)?;

    let mut file = File::create_new(full_path.join(format!("{file_name}.txt")))?;

    file.write_all(file_path.as_bytes())?;

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let _ = match cli.command {
        Commands::Create { path, file_type } => chunk_file(&path, &file_type),
        Commands::Run => run_chat(),
        Commands::Add { path, folder } => add(&path, folder),
    };
}
