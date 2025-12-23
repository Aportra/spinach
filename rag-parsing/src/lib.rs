use anyhow::{bail, Result};
use dirs::home_dir;
use fastembed::TextEmbedding;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use serde::Serialize;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use util::VecMath;
use walkdir::WalkDir;

mod util;
//syntax would be to llm look at <path> <folder/file-name>

type DataPassBack = Vec<HashMap<String, String>>;
type NewsResult = HashMap<String, Vec<String>>;

#[derive(Debug, Deserialize, Serialize)]
struct JsonData {
    chunk_id: String,
    content: String,
    chunks: Vec<f32>,
}

#[pyfunction]
pub fn find_file(path: String) -> PyResult<String> {
    let full_path = format!("{path}");
    println!("Path exists? {}", Path::new(&full_path).exists());
    let directory: Vec<_> = WalkDir::new(full_path)
        .into_iter()
        .filter_map(|e| match e {
            Ok(e) if e.file_type().is_file() => Some(e),

            Err(err) => {
                println!("Walk dir error:{}", err);
                None
            }

            _ => None,
        })
        .filter_map(|e| read_to_string(e.path()).ok())
        .collect();
    let json: Vec<serde_json::Value> = directory
        .into_iter()
        .map(|s| serde_json::from_str(&s).unwrap())
        .collect();
    let json = serde_json::to_string(&json).map_err(|e| {
        println!("Error{e}");
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("JSON error: {e}"))
    })?;
    Ok(json)
}

fn parse_yaml() -> Result<Value> {
    let config_path = format!("{}/config.yaml", env!("CARGO_MANIFEST_DIR"));
    let default_config_path = format!("{}/config-default.yaml", env!("CARGO_MANIFEST_DIR"));
    let mut yaml = match File::open(config_path) {
        Ok(file) => file,
        Err(_) => File::open(default_config_path)?,
    };
    let mut content = String::new();
    yaml.read_to_string(&mut content)?;
    Ok(serde_yaml::from_str(&content).expect("error"))
}

#[pyfunction]
pub fn req_news(
    q: Option<String>,
    search: Option<String>,
    num: Option<usize>,
) -> PyResult<HashMap<String, Vec<String>>> {
    let num_articles = num.unwrap_or(10);

    let config = parse_yaml().unwrap();

    let news_api_key = config.get("news_api_key").and_then(|a| a.as_str()).unwrap();

    let default_news_sources = config.get("news_sources").and_then(|a| a.as_str()).unwrap();

    fn get_news(url: String, number: usize) -> Result<NewsResult> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("spinach-cli/0.1"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client build error: {e}"
                ))
            })?;

        let request = client
            .get(url)
            .send()
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("HTTP error: {e}"))
            })?
            .json::<serde_json::Value>()
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("JSON parse error: {e}"))
            })?;

        let mut news_result: NewsResult = HashMap::new();

        if let Some(articles) = request.get("articles").and_then(|a| a.as_array()) {
            for a in articles.iter().take(number) {
                let title = a.get("title").and_then(|t| t.as_str()).unwrap_or("");
                let description = a.get("description").and_then(|d| d.as_str()).unwrap_or("");
                let content = a.get("content").and_then(|d| d.as_str()).unwrap_or("");
                let url = a.get("url").and_then(|d| d.as_str()).unwrap_or("");
                news_result.insert(
                    title.to_string(),
                    vec![
                        description.to_string(),
                        content.to_string(),
                        url.to_string(),
                    ],
                );
            }
        };
        Ok(news_result)
    }

    let news_result = match (q,search){
        (Some(q),None) => {get_news(format!(
            "https://newsapi.org/v2/top-headlines?sources={q}&apiKey={news_api_key}"
        ),num_articles)},
(Some(q), Some(search))=> {get_news(format!(
            "https://newsapi.org/v2/everything?sources={q}&q={search}&sortBy=relevancy&apiKey={news_api_key}"
        ),num_articles)},
(None, Some(search))=> {get_news(format!(
            "https://newsapi.org/v2/everything?sources={default_news_sources}&q={search}&sortBy=relevancy&apiKey={news_api_key}"
        ),num_articles)}
        _ => {
            get_news(format!(
            "https://newsapi.org/v2/top-headlines?sources={default_news_sources}&apiKey={news_api_key}"
        ),num_articles)}
    };

    Ok(news_result.unwrap())
}
fn cosine_similarity(prompt: &[f32], embedded_text: &[Vec<f32>]) -> Result<Vec<f32>> {
    let len_prompt = prompt.len();
    let len_text = embedded_text[0].len();

    if len_prompt != len_text {
        bail!("embedding dimension mismatch {len_prompt}|{len_text}");
    }

    fn dot(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    fn magnitude(a: &[f32]) -> f32 {
        a.iter().map(|x| x * x).sum::<f32>().sqrt()
    }
    let prompt_mag = magnitude(prompt);

    let text_mag: Vec<f32> = embedded_text
        .iter()
        .map(|x| x.as_slice())
        .map(magnitude)
        .collect();

    Ok(embedded_text
        .iter()
        .zip(text_mag.iter())
        .map(|(x, m)| dot(x, prompt) / (m * prompt_mag))
        .collect())
}

fn user_input() -> String {
    let mut input = String::new();

    print!(">>");

    io::stdout().flush().unwrap();

    io::stdin()
        .read_line(&mut input)
        .expect("failed to read line");

    let prompt = input.trim();

    if prompt.ends_with("+++") {
        let mut o = String::new();
        loop {
            let mut pasted_string = String::new();

            io::stdin()
                .read_line(&mut pasted_string)
                .expect("failed to read pasted text");
            if pasted_string.trim_end() == "END" {
                break;
            }

            o.push_str(&pasted_string);
        }
        let output = prompt.to_owned() + &o;

        output.to_string()
    } else {
        prompt.to_string()
    }
}

#[pyfunction]
pub fn look(context: String, folder: Option<String>) -> PyResult<(DataPassBack, usize, String)> {
    let config = parse_yaml().unwrap();

    let overlap = config.get("overlap").and_then(|a| a.as_u64()).unwrap() as usize;

    let chunk = config.get("chunk_size").and_then(|a| a.as_u64()).unwrap() as usize;

    let prompt: Vec<&str> = context.split_whitespace().collect();

    let embeder = TextEmbedding::try_new(Default::default()).unwrap();

    let path = prompt[1];

    if Path::exists(Path::new(&path)) {
        let m_data = File::open(path).unwrap().metadata().unwrap();
        if m_data.len() < (100 << (20)) {
            let user_prompt = user_input();
            let split_user = user_prompt.split_whitespace().collect();
            let content = read_to_string(path)?;

            let final_content: Vec<&str> = content.split_inclusive('\n').collect();

            let mut start = 0;
            let chunk_size = chunk;
            let stride = overlap;

            let mut pass_back: DataPassBack = Vec::new();
            let mut chunk_id = 0;
            let mut chunks = Vec::new();
            let embedded_prompt = embeder.embed(split_user, None).unwrap().pop().unwrap();
            while start < final_content.len() {
                let mut prepped_file: HashMap<String, String> = HashMap::new();
                let end = usize::min(start + chunk_size, final_content.len());
                let chunk_text = if final_content.iter().any(|w| w.contains("\n")) {
                    final_content.join("")
                } else {
                    final_content.join(" ")
                };

                let embedded_text = embeder
                    .embed(vec![chunk_text.clone()], None)
                    .unwrap()
                    .pop()
                    .unwrap();
                chunks.push(embedded_text.clone());
                prepped_file.insert("content".to_string(), chunk_text);
                prepped_file.insert("chunk_id".to_string(), chunk_id.to_string());
                prepped_file.insert("path".to_string(), path.to_string());

                let _ = embedded_text
                    .iter()
                    .map(|x| prepped_file.insert("chunks".to_string(), x.to_string()));

                start += stride;

                pass_back.push(prepped_file);

                chunk_id += 1
            }

            let similarities = cosine_similarity(&embedded_prompt, &chunks).unwrap();
            let top_idx = similarities.argmax().unwrap();

            // similarities[top_idx] = 0

            Ok((pass_back, top_idx, user_prompt))
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(
                "Error file exceeds size 300mb",
            ))
        }
    } else {
        let result = match folder {
            Some(folder) if folder == "dynamic" => {
                let file = home_dir()
                    .unwrap()
                    .join(format!("spinach-rag/dynamic/{}", prompt[2]));

                let dyn_prompt = read_to_string(file)?;

                look(format!("look {dyn_prompt}"), None)
            }
            Some(folder) if folder == "data" => {
                let user_prompt = user_input();
                let split_user = user_prompt.split_whitespace().collect();
                let file = home_dir()
                    .unwrap()
                    .join(format!("spinach-rag/data/{}", prompt[2]));

                let result_json = find_file(file.to_string_lossy().to_string())?;

                let embed_files: Vec<JsonData> = serde_json::from_str(&result_json).unwrap();

                let chunks: Vec<Vec<f32>> =
                    embed_files.iter().map(|x| x.chunks.to_owned()).collect();

                let embedded_prompt = embeder.embed(split_user, None).unwrap().pop().unwrap();
                let similarities: Vec<f32> = cosine_similarity(&embedded_prompt, &chunks).unwrap();

                let top_idx = similarities.argmax().unwrap();

                let mut final_data: Vec<HashMap<String, String>> = Vec::new();
                let converted_chunks: Vec<String> = embed_files
                    .iter()
                    .flat_map(|item| item.chunks.iter().map(|f| f.to_string()))
                    .collect();
                for i in 0..embed_files.len() {
                    let mut data: HashMap<String, String> = HashMap::new();
                    data.insert("content".to_string(), embed_files[i].content.clone());
                    data.insert("chunk_id".to_string(), embed_files[i].chunk_id.clone());
                    data.insert("path".to_string(), converted_chunks[i].clone());

                    final_data.push(data)
                }

                Ok((final_data, top_idx, user_prompt))
            }
            None => Err(pyo3::exceptions::PyTypeError::new_err(
                "Error file not found",
            )),
            _ => Err(pyo3::exceptions::PyTypeError::new_err("File error")),
        };

        Ok(result?)
    }
}

#[pyfunction]
pub fn search(q: String) -> PyResult<HashMap<String, Vec<String>>> {
    let config = parse_yaml().unwrap();

    let search_api = config.get("search_api").and_then(|a| a.as_str()).unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("spinach-cli/0.1"));

    let client = Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Client build error: {e}"))
        })?;

    let search_query = q.replace(" ", "%20");

    let request = client
        .get(format!(
            "https://google.serper.dev/search?q={search_query}&apiKey={search_api}"
        ))
        .send()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("HTTP error: {e}")))?
        .json::<serde_json::Value>()
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("JSON parse error: {e}"))
        })?;

    let mut search_result: HashMap<String, Vec<String>> = HashMap::new();

    if let Some(search) = request.get("organic").and_then(|a| a.as_array()) {
        for a in search.iter().take(10) {
            let title = a.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let description = a.get("snippet").and_then(|d| d.as_str()).unwrap_or("");
            let url = a.get("link").and_then(|d| d.as_str()).unwrap_or("");
            search_result.insert(
                title.to_string(),
                vec![description.to_string(), url.to_string()],
            );
        }
    };
    Ok(search_result)
}

#[pymodule]
fn spinach(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(find_file, m)?)?;
    m.add_function(wrap_pyfunction!(req_news, m)?)?;
    m.add_function(wrap_pyfunction!(look, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    Ok(())
}
