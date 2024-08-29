use std::{
    convert::Infallible,
    io::{BufWriter, Cursor},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::Future;
use headers_accept::Accept;
use http::{header, HeaderMap, Request, Response};
use http_body::Body;
use http_body_util::Full;
use image::{io::Reader as ImageReader, ImageFormat};
use percent_encoding::percent_decode_str;
use tokio::task;
use tower_service::Service;
use tracing::instrument;
use url::Url;

use crate::{
    image_type::{SupportedImageType, SupportedImageTypes, DEFAULT_SUPPORTED_IMAGE_TYPES},
    key::Key,
    signed::Verifier,
    transformation_params::TransformationParams,
};

#[derive(Debug, thiserror::Error)]
pub enum ImageXformError {
    #[error(transparent)]
    Image(#[from] image::error::ImageError),

    #[error(transparent)]
    WriterFinalization(#[from] std::io::IntoInnerError<BufWriter<Cursor<Vec<u8>>>>),
}

struct TransformedImage {
    bytes: Vec<u8>,
    format: ImageFormat,
}

#[derive(Debug, Clone)]
pub struct ImageTransformer<ResBody = Full<Bytes>> {
    client: reqwest::Client,
    verifier: Verifier,
    supported_image_types: SupportedImageTypes,

    // Covariant over ResBody; no dropping of ResBody.
    _marker: PhantomData<fn() -> ResBody>,
}

/// Builder for [`ImageTransformer`].
#[derive(Debug)]
pub struct ImageTransformerBuilder {
    client: reqwest::Client,
    verifier: Verifier,
    supported_image_types: SupportedImageTypes,
}

impl ImageTransformerBuilder {
    /// Create a new [`ImageTransformerBuilder`] with the provided [`Key`].
    pub fn new(key: Key) -> Self {
        let client = reqwest::Client::new();
        let verifier = Verifier::new(key);

        Self {
            client,
            verifier,
            supported_image_types: DEFAULT_SUPPORTED_IMAGE_TYPES,
        }
    }

    /// Configure the `client`.
    pub fn set_client(self, client: reqwest::Client) -> Self {
        Self { client, ..self }
    }

    /// Configure supported image types.
    pub fn set_supported_image_types(self, supported_image_types: SupportedImageTypes) -> Self {
        Self {
            supported_image_types,
            ..self
        }
    }

    /// Build the [`ImageTransformer`].
    pub fn build(self) -> ImageTransformer {
        ImageTransformer {
            client: self.client,
            verifier: self.verifier,
            supported_image_types: self.supported_image_types,
            _marker: PhantomData,
        }
    }
}

impl<ReqBody, ResBody> Service<Request<ReqBody>> for ImageTransformer<ResBody>
where
    ReqBody: Send + 'static,
    ResBody: Body<Data = Bytes>,
    ResBody::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = Response<Full<Bytes>>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let client = self.client.clone();
        let supported_image_types = self.supported_image_types;
        let verifier = self.verifier.clone();

        Box::pin(async move {
            // Parse accept header.
            let accept_header = req.headers().get(header::ACCEPT);
            let Some::<Accept>(accept) = accept_header.and_then(|v| v.try_into().ok()) else {
                tracing::error!(
                    header_value = ?accept_header,
                    "missing or invalid accept header"
                );
                return Ok(response_with_status(http::StatusCode::BAD_REQUEST));
            };

            let uri = req.uri();

            // Expected format should follow:
            //
            //   /{signature}/{transform_param1},...,{transform_paramN}/{image_url}
            //
            // This provides three segments:
            //
            //   1. HMAC digest (i.e. signature) of the transform parameters and image URL,
            //   2. Comma-separated transform parameters,
            //   3. And URL-encoded image URL.
            //
            // For example, a valid request might look like:
            //
            //   https://example.com/_image/36c6...5xE=/w_100,h_100/https%3A%2F%2Fwww.rustacean.net%2Fassets%2Frustacean-orig-noshadow.png
            let segments: Vec<&str> = uri.path().trim_start_matches('/').splitn(3, '/').collect();

            if segments.len() != 3 {
                tracing::error!(uri = %uri, "invalid path");
                return Ok(response_with_status(http::StatusCode::BAD_REQUEST));
            }

            let signature = segments[0];
            let value = [segments[1], segments[2]].concat();

            if !verifier.verify(signature, &value) {
                tracing::error!(uri = %uri, "could not verify signature");
                return Ok(response_with_status(http::StatusCode::BAD_REQUEST));
            }

            let Ok(transformation_params) = segments[1].parse::<TransformationParams>() else {
                tracing::error!(uri = %uri, "invalid transformation parameters");
                return Ok(response_with_status(http::StatusCode::BAD_REQUEST));
            };

            let Some(target_url) = percent_decode_str(segments[2])
                .decode_utf8()
                .ok()
                .and_then(|decoded| decoded.parse::<Url>().ok())
            else {
                tracing::error!(uri = %uri, "invalid target URL");
                return Ok(response_with_status(http::StatusCode::BAD_REQUEST));
            };

            // Load the image from the provided image URL.
            let proxy_res = match client.get(target_url).send().await {
                Err(err) => {
                    tracing::error!(err = %err, "failed to load image");
                    return Ok(response_with_status(http::StatusCode::BAD_GATEWAY));
                }

                Ok(proxy_res) => proxy_res,
            };

            // Load image bytes from the proxied response.
            let image_bytes = match proxy_res.bytes().await {
                Err(err) => {
                    tracing::error!(err = %err, "failed to load image bytes");
                    return Ok(response_with_status(http::StatusCode::BAD_GATEWAY));
                }

                Ok(image_bytes) => image_bytes,
            };

            // Apply image transformation in accordance with the request specification.
            //
            // Note that this is a blocking action, so we spawn a dedicated blocking task.
            let transformed_image = match task::spawn_blocking(move || {
                transform_image(
                    &accept,
                    supported_image_types,
                    &image_bytes,
                    &transformation_params,
                )
            })
            .await
            {
                // Something went wrong with the task.
                Err(err) => {
                    tracing::error!(err = %err, "failed to transform image (task failed)");
                    return Ok(response_with_status(
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }

                // Something went wrong with the image transformation.
                Ok(Err(err)) => {
                    tracing::error!(err = %err, "failed to transform image (transform failed)");
                    return Ok(response_with_status(
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }

                Ok(Ok(transformed_image)) => transformed_image,
            };

            // Construct response headers.
            //
            // We provide `Vary`, to ensure appropriate caching; i.e. based on the value of
            // `Accept`.
            //
            // A `Cache-Control` is hardcoded for now, but should be made configurable in
            // the future.
            //
            // Both `Content-Type` and `Content-Length` are derived from the transformed
            // image directly.
            let mut res_headers = HeaderMap::new();
            res_headers.insert(http::header::VARY, http::header::ACCEPT.into());
            res_headers.insert(
                http::header::CACHE_CONTROL,
                // TODO: This should be made configurable with a default when not explicitly
                // configured.
                "public, must-revalidate, max-age=31536000, s-maxage=31536000"
                    .parse()
                    .expect("Must parse a header value"),
            );
            res_headers.insert(
                http::header::CONTENT_TYPE,
                transformed_image
                    .format
                    .to_mime_type()
                    .parse()
                    .expect("Must parse a header value"),
            );
            res_headers.insert(
                http::header::CONTENT_LENGTH,
                transformed_image.bytes.len().into(),
            );

            let mut res = Response::new(Full::from(Bytes::from(transformed_image.bytes)));
            *res.headers_mut() = res_headers;

            Ok(res)
        })
    }
}

fn response_with_status<B>(status_code: http::StatusCode) -> Response<B>
where
    B: Default,
{
    let mut res = Response::default();
    *res.status_mut() = status_code;
    res
}

#[instrument(skip_all, fields(accept, supported_image_types, image_xform_req), err)]
fn transform_image<'a>(
    accept: &Accept,
    supported_image_types: &'a [SupportedImageType<'a>],
    image_bytes: &[u8],
    transformation_params: &TransformationParams,
) -> Result<TransformedImage, ImageXformError> {
    let image_reader = ImageReader::new(Cursor::new(image_bytes))
        .with_guessed_format()
        .map_err(|err| ImageXformError::Image(image::error::ImageError::IoError(err)))?;

    let guessed_format = image_reader.format();
    let format = determine_format(accept, supported_image_types, guessed_format);

    let mut image = image_reader.decode().map_err(ImageXformError::Image)?;

    if transformation_params.width.is_some() || transformation_params.height.is_some() {
        let width = transformation_params.width.unwrap_or_else(|| image.width());
        let height = transformation_params
            .height
            .unwrap_or_else(|| image.height());
        image = image.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    }

    let mut writer = BufWriter::new(Cursor::new(Vec::with_capacity(image.as_bytes().len())));
    image.write_to(&mut writer, format)?;

    Ok(TransformedImage {
        bytes: writer
            .into_inner()
            .map_err(ImageXformError::WriterFinalization)?
            .into_inner(),
        format,
    })
}

#[instrument(skip_all, fields(accept, supported_image_types, guessed_format), ret)]
fn determine_format<'a>(
    accept: &Accept,
    supported_image_types: &'a [SupportedImageType<'a>],
    guessed_format: Option<ImageFormat>,
) -> ImageFormat {
    let supported_media_types = supported_image_types.iter().map(Into::into);

    if let Some(negotiated) = accept.negotiate(supported_media_types) {
        for supported in supported_image_types {
            if supported.media_type == *negotiated {
                return supported.image_format;
            }
        }
    }

    tracing::warn!(
        accept = %accept,
        supported_media_types = ?supported_image_types,
        "No supported image type found"
    );

    // Default to PNG if no media type is negotiated
    guessed_format.unwrap_or(ImageFormat::Png)
}
