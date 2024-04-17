// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use futures_util::StreamExt;
use scraper::selectable::Selectable;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::{fs::File, io::Write, time::Instant};
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Progress {
    pub download_id: i64,
    pub filesize: u64,
    pub transfered: u64,
    pub transfer_rate: f64,
    pub percentage: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Product {
    name: String,
    description: String,
    url: String,
    download_link: String,
}

impl Progress {
    pub fn emit_progress(&self, handle: &AppHandle) {
        handle.emit_all("DOWNLOAD_PROGRESS", &self).ok();
    }

    pub fn emit_finished(&self, handle: &AppHandle) {
        handle.emit_all("DOWNLOAD_FINISHED", &self).ok();
    }
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

async fn scrape_cr(page: &str) -> Result<Vec<Product>, ()> {
    let url = format!("https://filecr.com/macos/?page={}", page);
    let response = reqwest::get(url).await.unwrap();

    let body = response.text().await.unwrap();
    let document = scraper::Html::parse_document(&body);

    let section_selector = scraper::Selector::parse("section.products").unwrap();
    let section = document.select(&section_selector).next().unwrap();

    let product_list_selector = scraper::Selector::parse("div.product-list").unwrap();
    let product_list = section.select(&product_list_selector).next().unwrap();

    let product_div_selector = scraper::Selector::parse("div").unwrap();
    let product_div = product_list.select(&product_div_selector);

    let mut product_struct: Vec<Product> = Vec::new();

    for product in product_div {
        let parent_selector = scraper::Selector::parse("div").unwrap();
        let child_selector = scraper::Selector::parse("a").unwrap();

        if let Some(parent) = product.select(&parent_selector).nth(1) {
            if let Some(child) = parent.select(&child_selector).nth(0) {
                if child.value().attr("href").unwrap().eq("/macos/") {
                    continue;
                }
                let url = child.value().attr("href").unwrap().to_string();
                let clean_url = url.replace("/macos/", "").replace("/", "");
                let clean_url_duplicate = clean_url.clone();

                product_struct.push(Product {
                    name: clean_url,
                    description: url.to_string(),
                    url: url.to_string(),
                    download_link: clean_url_duplicate,
                });
            }
        }
    }
    Ok(product_struct)
}

async fn get_download_url(url: &str) -> String {
    let response = reqwest::get(format!(
        "https://filecr.com/_next/data/QY65jIg1vE9Ef3Z159Z-z/macos/{}.json?categorySlug=macos",
        url
    ))
    .await
    .unwrap();

    // Get the Content of the HTML and print it
    let body = response.text().await.unwrap();
    let v: Value = serde_json::from_str(&body).unwrap();

    let mut product_id = String::new();
    // Access nested values
    if let Some(page_props) = v.get("pageProps") {
        if let Some(post) = page_props.get("post") {
            if let Some(downloads) = post.get("downloads") {
                if let Some(first_download) = downloads.get(0) {
                    if let Some(links) = first_download.get("links") {
                        if let Some(first_link) = links.get(0) {
                            if let Some(id) = first_link.get("id") {
                                product_id = id.to_string();
                            }
                        }
                    } else {
                        if let Some(id) = first_download.get("id") {
                            product_id = id.to_string();
                        }
                    }
                }
            }
        }
    }

    let download_req = reqwest::get(format!(
        "https://filecr.com/api/actions/downloadlink/?id={}",
        product_id
    ))
    .await
    .unwrap();

    let body = download_req.text().await.unwrap();
    let v: Value = serde_json::from_str(&body).unwrap();

    let mut url = String::new();
    if let Some(dl_url) = v.get("url") {
        url = dl_url.to_string();
    }

    return url;
}

#[tauri::command]
async fn download_file(url: &str, handle: AppHandle) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .get(url.replace("\"", "").replace("\"", ""))
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    let source_size = response
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;

    let start = Instant::now();
    let mut last_update = Instant::now();

    let response_clone = reqwest::get(url.replace("\"", "").replace("\"", ""))
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;
    let fname = response_clone
        .url()
        .path_segments()
        .and_then(|segments| segments.last())
        .and_then(|name| if name.is_empty() { None } else { Some(name) })
        .unwrap_or("phunk.zip");

    let mut download_size: u64 = 0;
    let mut stream = response.bytes_stream();

    let id = Uuid::new_v4();

    let mut downloads_path = dirs::download_dir().unwrap_or_else(|| PathBuf::from("."));
    downloads_path.push(fname);

    let path = format!("{}", downloads_path.display());
    let mut file = File::create(&path).or(Err(format!("Failed to create '{}'", &path)))?;

    let mut progress = Progress {
        download_id: id.as_u128() as i64,
        filesize: source_size,
        transfered: 0,
        transfer_rate: 0.0,
        percentage: 0.0,
    };

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Failed to get chunk from '{}'", &url)))?;
        file.write(&chunk)
            .or(Err(format!("Failed to write chunk from '{}'", &url)))?;

        download_size += chunk.len() as u64;
        progress.transfered = download_size;
        progress.percentage = (progress.transfered * 100 / source_size) as f64;
        progress.transfer_rate = (download_size as f64) / (start.elapsed().as_secs() as f64)
            + (start.elapsed().subsec_nanos() as f64 / 1_000_000_000.0).trunc();

        if last_update.elapsed() > std::time::Duration::from_secs(1) {
            progress.emit_progress(&handle);
            last_update = Instant::now();
        }
    }

    if progress.percentage >= 100.0 {
        progress.emit_finished(&handle);
    }

    if download_size == 0 || download_size < source_size {
        Err(format!("Failed to download file from '{}'", &url))
    } else {
        format!("{}", fname);

        Ok(())
    }
}

#[tauri::command]
async fn get_products(page: &str) -> Result<Vec<Product>, ()> {
    let (tx, rx) = oneshot::channel::<Result<Vec<Product>, ()>>();

    tokio::spawn(async move {
        let products = scrape_cr("1").await;
        tx.send(products).unwrap();
    });

    let response = rx.await.unwrap()?;

    let (sender, mut receiver) = mpsc::channel::<Vec<Product>>(10);

    tokio::spawn(async move {
        let mut final_products: Vec<Product> = Vec::new();
        for product in response {
            let download_url = get_download_url(&product.download_link).await;
            let product = Product {
                name: product.name.clone(),
                description: product.description.clone(),
                url: product.url.clone(),
                download_link: download_url,
            };
            final_products.push(product);
        }

        sender.send(final_products).await.unwrap();
    });

    let products = receiver.recv().await.unwrap();
    Ok(products)
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, get_products, download_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
