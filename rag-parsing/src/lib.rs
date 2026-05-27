use anyhow::{bail, Result};
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
use scraper::{Html, Selector};
use util::VecMath;
use walkdir::WalkDir;

mod util;

type DataPassBack = HashMap<String, String>;
type NewsResult = HashMap<String, HashMap<String, String>>;

#[derive(Debug, Deserialize, Serialize)]
struct JsonData {
    chunk_id: String,
    content: String,
    chunks: Vec<f32>,
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
pub fn req_news() -> PyResult<HashMap<String, String>> {
    let num_articles = 10;

    let config = parse_yaml().unwrap();

    let news_api_key = config.get("news_api_key").and_then(|a| a.as_str()).unwrap();

    let default_news_sources = config.get("news_sources").and_then(|a| a.as_str()).unwrap();

    fn get_news(url: String, number: usize) -> Result<HashMap<String,String>> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"));
        headers.insert("Accept",HeaderValue::from_static("application/json, text/plain, */*"));

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
                let mut d = HashMap::new();
                let title = a.get("title").and_then(|t| t.as_str()).unwrap_or("");
                let source = a.get("source").and_then(|s| s.get("name")).and_then(|d| d.as_str()).unwrap_or("");
                let description = a.get("description").and_then(|d| d.as_str()).unwrap_or("");
                let content = a.get("content").and_then(|d| d.as_str()).unwrap_or("");
                let url = a.get("url").and_then(|d| d.as_str()).unwrap_or("");
                let date = a.get("publishedAt").and_then(|d| d.as_str()).unwrap_or("");
                   d.insert( 
                        "description".to_string(),description.to_string()
                   );
                   d.insert("content".to_string(),
                        content.to_string());
                   d.insert("url".to_string(),
                        url.to_string());
                    d.insert("published_date".to_string(),
                        date.to_string());
                    d.insert("source".to_string(),
                        source.to_string());
                news_result.insert(
                    title.to_string(),
                    d
                );
            }
        };
        let mut final_hash:HashMap<String,String> = HashMap::new(); 
        for key in news_result.keys(){
            let url = &news_result[key]["url"];

            let l = client.get(url).send()
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("HTTP error: {e}"))
            })?
            .text()
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("JSON parse error: {e}"))
            })?;

            let document = Html::parse_document(&l);

            let p_tags = Selector::parse("p").unwrap();

            let mut doc = String::new();
            doc.push_str(&format!("URL: {}",url));
            for ele in document.select(&p_tags){
                doc.push_str(&ele.text().collect::<String>());
            }
            final_hash.insert(key.to_owned(),doc);
            
        }

        Ok(final_hash)
    }

    let result = 
            get_news(format!(
            "https://newsapi.org/v2/top-headlines?sources={default_news_sources}&apiKey={news_api_key}"
        ),num_articles).unwrap();
    

    Ok(result)
}

pub fn cosine_similarity(prompt: Vec<f32>, embedded_text: Vec<Vec<f32>>) -> Result<usize> {
    fn dot(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    fn magnitude(a: &[f32]) -> f32 {
        a.iter().map(|x| x * x).sum::<f32>().sqrt()
    }
    let prompt_mag = magnitude(prompt.as_slice());

    let text_mag: Vec<f32> = embedded_text
        .iter()
        .map(|x| x.as_slice())
        .map(magnitude)
        .collect();
    let similarities: Vec<f32> = embedded_text
        .iter()
        .zip(text_mag.iter())
        .map(|(x, m)| dot(x, prompt.as_slice()) / (m * prompt_mag))
        .collect();
    Ok(similarities.argmax().unwrap())
}

#[pyfunction]
pub fn look(context: String, file: HashMap<String, String>) -> PyResult<DataPassBack> {
    let config = parse_yaml().unwrap();

    let overlap = config.get("overlap").and_then(|a| a.as_u64()).unwrap() as usize;

    let chunk = config.get("chunk_size").and_then(|a| a.as_u64()).unwrap() as usize;

    let embeder = TextEmbedding::try_new(Default::default()).unwrap();

    let mut pass_back: DataPassBack = DataPassBack::new();

    let embedded_prompt = embeder
        .embed(vec![context.as_str()], None)
        .unwrap()
        .pop()
        .unwrap();
    for key in file.keys() {
        let file_content: Vec<&str> = file[key].split_inclusive('\n').collect();

        let mut start = 0;
        let chunk_size = chunk;
        let stride = overlap;

        let mut chunks = Vec::new();
        let mut c_text = Vec::new();
        while start < file_content.len() {
            let end = usize::min(start + chunk_size, file_content.len());
            let file_c = &file_content[start..end];
            let chunk_text = if file_c.iter().any(|w| w.contains("\n")) {
                file_c.join("")
            } else {
                file_c.join(" ")
            };

            let embedded_text = embeder
                .embed(vec![chunk_text.clone()], None)
                .unwrap()
                .pop()
                .unwrap();
            chunks.push(embedded_text);
            c_text.push(chunk_text);

            start += stride;
        }
        let top_idx = cosine_similarity(embedded_prompt.clone(), chunks).unwrap();
        pass_back.insert(key.to_string(), c_text[top_idx].clone());
    }

    Ok(pass_back)
}

#[pyfunction]
pub fn search(q: String) -> PyResult<HashMap<String, Vec<String>>> {
    let config = parse_yaml().unwrap();

    let search_api = config.get("search_api").and_then(|a| a.as_str()).unwrap_or_default();

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("spinach-cli/0.1"));
    headers.insert("Accept",HeaderValue::from_static("application/json"));
    headers.insert("X-Subscription-Token",HeaderValue::from_str(search_api).unwrap());

    let client = Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Client build error: {e}"))
        })?;

    let request = client
        .get(format!(
            "https://api.search.brave.com/res/v1/web/search"
        ))
        .query(&[("q",q)])
        .send()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("HTTP error: {e}")))?
        .json::<serde_json::Value>()
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("JSON parse error: {e}"))
        })?;

    let mut search_result: HashMap<String, Vec<String>> = HashMap::new();
    
    if let Some(search) = request.get("web").and_then(|a| a.get("results")).and_then(|d| d.as_array()) {
        for a in search.iter().take(10) {
            let title = a.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let url = a.get("url").and_then(|t| t.as_str()).unwrap_or("");
            let description = a.get("description").and_then(|d| d.as_str()).unwrap_or("");
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
    m.add_function(wrap_pyfunction!(req_news, m)?)?;
    m.add_function(wrap_pyfunction!(look, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    Ok(())
}
