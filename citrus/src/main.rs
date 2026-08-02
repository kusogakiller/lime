mod cli;
mod project;
mod manifest;
mod build;
mod test_cmd;
mod fmt;

fn main() {
    let command = cli::parse();
    let result = match command {
        cli::Command::New { name } => project::create_project(&name),
        cli::Command::Build { release } => {
            let profile = if release { build::Profile::Release } else { build::Profile::Debug };
            build::build(profile).map(|_| ())
        }
        cli::Command::Run { release } => build::run(release),
        cli::Command::Test { release } => test_cmd::run(release),
        cli::Command::Fmt => fmt::run(),
    };
    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
