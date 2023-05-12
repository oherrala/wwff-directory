fn main() {
    tracing_subscriber::fmt::init();

    let dir = wwff_directory::from_path("wwff_directory.csv").unwrap();
    for e in dir {
        println!("{:#?}", e);
    }
}
