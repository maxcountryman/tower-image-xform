//! # Overview
//!
//! This crate provides image transformations, such as resize, as a `tower`
//! middleware.
//!
//! # Usage with an `axum` application
//!
//! ```rust,no_run
//! use std::net::SocketAddr;
//!
//! use axum::{routing::get_service, Router};
//! use tower::ServiceBuilder;
//! use tower_image_xform::{
//!     image_type, ImageTransformerBuilder, Key, SignedUrlBuilder, SupportedImageTypes,
//! };
//! use url::Url;
//!
//! const SUPPORTED_IMAGE_TYPES: SupportedImageTypes = &[image_type::WEBP, image_type::PNG];
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Service set up.
//!     let signing_key = Key::generate();
//!     let image_xformer = ImageTransformerBuilder::new(signing_key.clone())
//!         .set_supported_image_types(SUPPORTED_IMAGE_TYPES)
//!         .build();
//!
//!     // URL construction.
//!     let base_url: Url = "http://localhost:3000/_image/".parse()?;
//!     let target_url = "https://www.rustacean.net/assets/rustacean-orig-noshadow.png".parse()?;
//!     let signed_url = SignedUrlBuilder::new()
//!         .key(signing_key)
//!         .base(base_url)
//!         .params()
//!         .height(100)
//!         .width(150)
//!         .target(target_url)
//!         .build()
//!         .generate_signed_url()?;
//!
//!     println!(
//!         "Open this link in your browser to view the transformed image: {}",
//!         signed_url
//!     );
//!
//!     // Nest service within an Axum app at `/_image` path.
//!     let image_xform_service = get_service(ServiceBuilder::new().service(image_xformer));
//!     let app = Router::new().nest_service("/_image", image_xform_service);
//!
//!     let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
//!     let listener = tokio::net::TcpListener::bind(&addr).await?;
//!     axum::serve(listener, app.into_make_service()).await?;
//!
//!     Ok(())
//! }
//! ```
#![warn(
    clippy::all,
    nonstandard_style,
    future_incompatible,
    missing_docs,
    missing_debug_implementations
)]
#![forbid(unsafe_code)]

pub mod image_type;
mod key;
mod service;
mod signed;
mod transformation_params;

pub use image_type::{SupportedImageTypes, DEFAULT_SUPPORTED_IMAGE_TYPES};
pub use key::Key;
pub use service::ImageTransformerBuilder;
pub use signed::{SignedUrlBuilder, Verifier};
