use wwff_directory::WwffDirectory;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let mut wwff_directory = WwffDirectory::from_download().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    wwff_directory.try_download_update().await.unwrap();
}
