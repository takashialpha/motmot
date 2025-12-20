use crate::config::Logging;
use once_cell::sync::Lazy;
use std::io::{self, Write};
use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

static FILE_CHANNEL: Lazy<Mutex<Option<mpsc::UnboundedSender<String>>>> =
    Lazy::new(|| Mutex::new(None));

pub async fn init_logging_async(cfg: &Logging) -> anyhow::Result<()> {
    if let Some(path) = &cfg.file_path {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        *FILE_CHANNEL.lock().unwrap() = Some(tx);

        let path = path.clone();
        tokio::spawn(async move {
            use tokio::fs::OpenOptions;
            use tokio::io::AsyncWriteExt;

            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .expect("Failed to open log file");

            while let Some(line) = rx.recv().await {
                let _ = file.write_all(line.as_bytes()).await;
                let _ = file.write_all(b"\n").await;
                let _ = file.flush().await;
            }
        });
    }

    let subscriber = tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::LocalTime::rfc_3339())
        .with_writer(MultiWriter)
        .with_env_filter(&cfg.level)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

struct MultiWriter;

impl<'a> MakeWriter<'a> for MultiWriter {
    type Writer = MultiWriterHandle;

    fn make_writer(&'a self) -> Self::Writer {
        MultiWriterHandle
    }
}

struct MultiWriterHandle;

impl Write for MultiWriterHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = std::str::from_utf8(buf).unwrap_or("<invalid utf8>");

        print!("{}", s);

        if let Some(tx) = &*FILE_CHANNEL.lock().unwrap() {
            let _ = tx.send(s.to_string());
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}
