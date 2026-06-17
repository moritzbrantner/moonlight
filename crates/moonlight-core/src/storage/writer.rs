use crate::ComparisonRun;
use std::{path::Path, sync::Arc};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
    sync::Mutex,
};

#[derive(Clone)]
pub struct RunWriter {
    file: Arc<Mutex<BufWriter<File>>>,
}

impl RunWriter {
    pub async fn open(write_path: std::path::PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = write_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(write_path)
            .await?;

        Ok(Self {
            file: Arc::new(Mutex::new(BufWriter::new(file))),
        })
    }

    pub async fn append(&self, run: &ComparisonRun) -> anyhow::Result<()> {
        let line = serde_json::to_string(run)?;
        let mut file = self.file.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn flush(&self) -> anyhow::Result<()> {
        self.file.lock().await.flush().await?;
        Ok(())
    }

    pub async fn reopen(&self, write_path: &Path) -> anyhow::Result<()> {
        let mut writer = self.file.lock().await;
        writer.flush().await?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(write_path)
            .await?;
        *writer = BufWriter::new(file);
        Ok(())
    }
}
