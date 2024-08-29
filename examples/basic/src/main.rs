use std::net::SocketAddr;

use axum::{routing::get_service, Router};
use tower::ServiceBuilder;
use tower_image_xform::{
    image_type, ImageTransformerBuilder, Key, SignedUrlBuilder, SupportedImageTypes,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;

// Define image types we want to support.
const SUPPORTED_IMAGE_TYPES: SupportedImageTypes = &[image_type::WEBP, image_type::PNG];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
            |_| "example_basic=debug,tower_image_xform=debug".into(),
        )))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    // Service set up.
    let signing_key = Key::generate();
    let image_xformer = ImageTransformerBuilder::new(signing_key.clone())
        .set_supported_image_types(SUPPORTED_IMAGE_TYPES)
        .build();

    // URL construction.
    let base_url: Url = "http://localhost:3000/_image/".parse()?;
    let target_url = "https://www.rustacean.net/assets/rustacean-orig-noshadow.png".parse()?;
    let signed_url = SignedUrlBuilder::new()
        .key(signing_key)
        .base(base_url)
        .params()
        .height(100)
        .width(150)
        .target(target_url)
        .build()
        .generate_signed_url()?;

    tracing::info!(%signed_url, "Open this link in your browser to view the transformed image");

    // Nest service within an Axum app at `/_image` path.
    let image_xform_service = get_service(ServiceBuilder::new().service(image_xformer));
    let app = Router::new().nest_service("/_image", image_xform_service);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
