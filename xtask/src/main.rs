fn main() {
    if let Err(error) = xtask::run(std::env::args().skip(1)) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
