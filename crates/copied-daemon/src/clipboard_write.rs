use wl_clipboard_rs::copy::{self, MimeType, Options, Source};

pub fn write_text(content: &str) -> Result<(), copy::Error> {
    let options = Options::default();
    let source = Source::Bytes(content.as_bytes().to_vec().into_boxed_slice());
    copy::copy(options, source, MimeType::Text)
}

pub fn write_image(bytes: Vec<u8>, mime: &str) -> Result<(), copy::Error> {
    let options = Options::default();
    let source = Source::Bytes(bytes.into_boxed_slice());
    copy::copy(options, source, MimeType::Specific(mime.to_string()))
}
