# S3 Browser

Dark-themed desktop GUI for browsing and managing Amazon S3 buckets, built with Rust + [Iced](https://iced.rs/) and the AWS SDK for Rust.

## Features

- List objects in an S3 bucket
- Upload files via native file picker or manual path entry
- Delete objects with confirmation dialog
- Right-click context menu on files
- Credentials and bucket loaded automatically from `.env`

## Prerequisites

- [Rust](https://rustup.rs/) (stable, 2024 edition)
- AWS credentials with `s3:ListBucket`, `s3:GetObject`, `s3:PutObject`, `s3:DeleteObject` on your bucket

## Configuration

Create a `.env` file in the project root (never committed):

```env
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key
AWS_REGION=us-east-1
AWS_S3_BUCKET=your-bucket-name
```

The bucket name pre-populates in the GUI on startup. Region defaults to `us-east-1` if unset.

## Build

```bash
cargo build --release
```

The compiled binary is at `target/release/s3_test`.

## Run

### GUI (default)

```bash
cargo run --release
```

Or run the compiled binary directly:

```bash
./target/release/s3_test
```

**Usage:**
- On launch the bucket from `.env` is loaded and objects listed automatically
- Change the bucket name in the text field and click **Connect** to switch buckets
- Click **Refresh** to reload the object list
- Click **Upload File** to upload — use **Browse** for a native file picker
- **Right-click** any object to open the context menu (Delete)

## Project Structure

```
src/
  main.rs      # Iced GUI application
  lib.rs       # S3 operations (list, upload, delete)
  error.rs     # S3ExampleError type
```
