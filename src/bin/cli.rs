#![allow(clippy::result_large_err)]

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, config::Region, meta::PKG_VERSION};
use clap::{Parser, Subcommand};
use s3_browser_tool::*;

#[derive(Debug, Parser)]
#[command(name = "s3-cli", about = "S3 bucket management CLI")]
struct Opt {
    /// AWS region (overrides AWS_REGION env var)
    #[arg(short, long, env = "AWS_REGION")]
    region: Option<String>,

    /// S3 bucket name (overrides AWS_S3_BUCKET env var)
    #[arg(short, long, env = "AWS_S3_BUCKET")]
    bucket: String,

    /// Print version and config info
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List objects in the bucket
    ListObjects,

    /// Upload a local file to the bucket
    UploadFile {
        #[arg(short, long)]
        file_name: String,
    },

    /// Delete an object from the bucket
    DeleteFile {
        #[arg(short, long)]
        file_name: String,
    },

    /// Download an object from the bucket to a local file
    DownloadFile {
        #[arg(short, long)]
        file_name: String,

        /// Local destination path (defaults to the object key name)
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), s3_browser_tool::error::S3ExampleError> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let Opt { region, bucket, verbose, command } = Opt::parse();

    let region_provider = RegionProviderChain::first_try(region.map(Region::new))
        .or_default_provider()
        .or_else(Region::new("us-east-1"));

    if verbose {
        println!("S3 client version: {}", PKG_VERSION);
        println!("Region:            {}", region_provider.region().await.unwrap().as_ref());
        println!("Bucket:            {}", &bucket);
        println!();
    }

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;
    let client = Client::new(&config);

    match command {
        Command::ListObjects => list_objects(&client, &bucket).await,

        Command::UploadFile { file_name } => {
            match upload_object(&client, &bucket, &file_name, &file_name).await {
                Ok(_) => {
                    println!("Uploaded '{}' successfully.", file_name);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to upload '{}': {}", file_name, e);
                    Err(e)
                }
            }
        }

        Command::DeleteFile { file_name } => {
            match remove_object(&client, &bucket, &file_name).await {
                Ok(_) => {
                    println!("Deleted '{}' successfully.", file_name);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to delete '{}': {}", file_name, e);
                    Err(e)
                }
            }
        }

        Command::DownloadFile { file_name, output } => {
            let destination = output.unwrap_or_else(|| {
                std::path::Path::new(&file_name)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| file_name.clone())
            });
            match download_object(&client, &bucket, &file_name, &destination).await {
                Ok(_) => {
                    println!("Downloaded '{}' to '{}'.", file_name, destination);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to download '{}': {}", file_name, e);
                    Err(e)
                }
            }
        }
    }
}
