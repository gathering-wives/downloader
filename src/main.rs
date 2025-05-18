use std::{fs::File, io::Write, path::PathBuf, sync::Arc};

use clap::Parser;
use futures::{future::join_all, StreamExt};
use globset::{Glob, GlobSetBuilder};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::sync::Semaphore;

mod cdn;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Parser)]
struct Cli {
    #[clap(short, long)]
    index_url: String,
    #[clap(short, long)]
    filelist_path: Option<String>,
    #[clap(short, long)]
    output_path: String,
}

async fn get_index(client: &reqwest::Client, url: &str) -> Result<cdn::IndexResponse> {
    let response = client.get(url).send().await?;
    let index: cdn::IndexResponse = response.json().await?;
    Ok(index)
}

async fn get_resources(client: &reqwest::Client, url: &str) -> Result<Vec<cdn::Resource>> {
    let response = client.get(url).send().await?;
    let resources: cdn::ResourcesResponse = response.json().await?;
    Ok(resources.resource)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let multi_progress = MultiProgress::new();
    let style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")?;

    let client = reqwest::Client::new();
    let semaphore = Arc::new(Semaphore::new(15));

    let index = get_index(&client, &cli.index_url).await?;
    let cdn = &index.default.cdnList[0].url;
    let resources_url = format!("{}/{}", cdn, index.default.resources);
    let resources_base = format!("{}/{}", cdn, index.default.resourcesBasePath);
    let output_path = PathBuf::from(&cli.output_path);
    let resources = get_resources(&client, &resources_url).await?;

    println!("Version: {}", index.default.version);
    println!("Resources: {}", resources.len());

    let glob = if let Some(filelist_path) = &cli.filelist_path {
        let filelist = std::fs::read_to_string(filelist_path)?;
        let globlist = filelist.lines();

        let mut builder = GlobSetBuilder::new();
        for glob in globlist {
            builder.add(Glob::new(glob)?);
        }

        Some(builder.build()?)
    } else {
        None
    };

    let mut tasks = Vec::new();

    for resource in resources {
        if let Some(glob) = &glob {
            if !glob.is_match(&resource.dest) {
                continue;
            }
        }

        let client = client.clone();
        let semaphore = semaphore.clone();
        let resource = resource.clone();
        let url = format!("{}/{}", resources_base, resource.dest);
        let output_path = output_path.join(&resource.dest[1..]);

        let pb = multi_progress.add(ProgressBar::new(resource.size));
        pb.set_style(style.clone());
        pb.set_message(resource.dest.clone());

        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            download_file(&client, pb, url, output_path, resource).await
        });

        tasks.push(task);
    }

    join_all(tasks).await;

    Ok(())
}

async fn download_file(
    client: &Client,
    pb: ProgressBar,
    url: String,
    output_path: PathBuf,
    _resource: cdn::Resource,
) -> Result<()> {
    std::fs::create_dir_all(output_path.parent().unwrap()).unwrap();

    let response = client.get(url).send().await?;
    let mut file = File::create(output_path).unwrap();
    let mut downloaded = 0;
    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_and_clear();

    Ok(())
}
