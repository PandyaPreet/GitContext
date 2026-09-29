fn main() {
    if let Err(error) = git_context_lib::bridge::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
