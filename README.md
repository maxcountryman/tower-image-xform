<h1 align="center">
    tower-image-xform
</h1>

<p align="center">
    🖼️ Image transformations as a `tower` middleware.
</p>

<div align="center">
    <a href="https://crates.io/crates/tower-image-xform">
        <img src="https://img.shields.io/crates/v/tower-image-xform.svg" />
    </a>
    <a href="https://docs.rs/tower-image-xform">
        <img src="https://docs.rs/tower-image-xform/badge.svg" />
    </a>
    <a href="https://github.com/maxcountryman/tower-image-xform/actions/workflows/rust.yml">
        <img src="https://github.com/maxcountryman/tower-image-xform/actions/workflows/rust.yml/badge.svg" />
    </a>
    <a href="https://codecov.io/gh/maxcountryman/tower-image-xform" > 
        <img src="https://codecov.io/gh/maxcountryman/tower-image-xform/graph/badge.svg?token=4WKTLPEGJC"/> 
    </a>
</div>

## 🎨 Overview

This crate provides image transformations, such as resize, as a `tower`
middleware.

### 🚧 Work-In-Progress 🚧

> [!WARNING]
> This crate's API is incomplete and subject to change.

Some things that might be nice for the future:

    -[] Load images directly from object stores via S3-compatible APIs
    -[] Additional transformations (quality, rotation, etc)
    -[] Target encryption?

## 📦 Install

To use the crate in your project, add the following to your `Cargo.toml` file:

```toml
[dependencies]
tower-image-xform = "0.1.0"
```

## 🤸 Usage

```rust
use std::net::SocketAddr;

use axum::{routing::get_service, Router};
use tower::ServiceBuilder;
use tower_image_xform::{
    image_type, ImageTransformerBuilder, Key, SignedUrlBuilder, SupportedImageTypes,
};
use url::Url;

// Define image types we want to support.
const SUPPORTED_IMAGE_TYPES: SupportedImageTypes = &[image_type::WEBP, image_type::PNG];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Service set up.
    let signing_key = Key::generate();
    let image_xformer = ImageTransformerBuilder::new(signing_key.clone())
        .set_supported_image_types(SUPPORTED_IMAGE_TYPES)
        .build();

    // URL construction.
    let base_url: Url = "http://localhost:3000/_image/".parse()?;
    let target_url = "https://www.rustacean.net/assets/rustacean-orig-noshadow.png"
        .parse()?;
    let signed_url = SignedUrlBuilder::new()
        .key(signing_key)
        .base(base_url)
        .params()
        .height(100)
        .width(150)
        .target(target_url)
        .build()
        .generate_signed_url()?;

    println!(
        "Open this link in your browser to view the transformed image: {}",
        signed_url
    );

    // Nest service within an Axum app at `/_image` path.
    let image_xform_service = get_service(ServiceBuilder::new().service(image_xformer));
    let app = Router::new().nest_service("/_image", image_xform_service);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
```

## 🦺 Safety

This crate uses `#![forbid(unsafe_code)]` to ensure everything is implemented in 100% safe Rust.

## 🛟 Getting Help

We've put together a number of [examples][examples] to help get you started. You're also welcome to [open a discussion](https://github.com/maxcountryman/tower-image-xform/discussions/new?category=q-a) and ask additional questions you might have.

## 👯 Contributing

We appreciate all kinds of contributions, thank you!

[examples]: https://github.com/maxcountryman/tower-image-xform/tree/main/examples
[docs]: https://docs.rs/tower-image-xform
