use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use chrono::Local;


pub struct Logger {
    sender: mpsc::Sender<String>,
}

impl Logger {

    pub fn new(file_path: &str) -> Self {
        let (sender, mut receiver): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel(100);

        let file_path = file_path.to_string();
        tokio::spawn(async move {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)
                .await
                .unwrap();

            while let Some(log) = receiver.recv().await {
                if let Err(e) = file.write_all(log.as_bytes()).await {
                    eprintln!("Failed to write log: {}", e);
                }
            }
        });
        Self { sender }
    }

    pub async fn log(&self, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let log_entry = format!("[{}] {}\n", timestamp, message);
        
        if let Err(e) = self.sender.send(log_entry).await {
            eprintln!("Failed to send log message: {}", e);
        }
    }
}