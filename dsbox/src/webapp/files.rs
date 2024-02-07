use std::io;

include!(concat!(env!("OUT_DIR"), '/', "embedded_files.rs"));

#[cfg(not(debug_assertions))]
#[derive(Copy, Clone)]
pub struct EmbeddedFile {
    pub data: &'static [u8],
    pub mime_type: &'static str,
    pub compressed: bool,
}

#[cfg(debug_assertions)]
pub struct EmbeddedFile {
    pub data: Vec<u8>,
    pub mime_type: String,
    pub compressed: bool,
}

#[cfg(not(debug_assertions))]
pub async fn lookup(file: &str) -> io::Result<Option<EmbeddedFile>> { Ok(EMBEDDED_FILES.get(file).copied()) }

#[cfg(debug_assertions)]
pub async fn lookup(file: &str) -> io::Result<Option<EmbeddedFile>> {
    use std::path::Path;
    use tokio::io::AsyncReadExt;

    let path = Path::new(WEBAPP_ROOT).join(file);
    let mut reader = match tokio::fs::File::open(&path).await {
        Ok(reader) => reader,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut data = Vec::new();
    reader.read_to_end(&mut data).await?;
    let mime_type = mime_guess::from_path(&path).first_or_text_plain().essence_str().to_owned();
    Ok(Some(EmbeddedFile {
        data,
        mime_type,
        compressed: false,
    }))
}