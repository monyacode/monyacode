use gpui::{AsyncApp, actions};

/// prefix for the monyacode:// url scheme
const MONYACODE_URL_SCHEME: &str = "monyacode";

actions!(
    cli,
    [
        /// Registers the monyacode:// URL scheme handler.
        RegisterUriScheme
    ]
);

pub async fn register_uri_scheme(cx: &AsyncApp) -> anyhow::Result<()> {
    cx.update(|cx| cx.register_url_scheme(MONYACODE_URL_SCHEME))?.await
}
