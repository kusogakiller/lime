mod cli;
mod project;
mod build;
mod test_cmd;
mod fmt;
mod pkg;

fn main() {
    let command = cli::parse();
    let result = match command {
        cli::Command::New { name } => project::create_project(&name),
        cli::Command::Build { release } => {
            let profile = if release { build::Profile::Release } else { build::Profile::Debug };
            build::build(profile).map(|_| ())
        }
        cli::Command::Run { release, args } => build::run(release, &args),
        cli::Command::Test { release } => test_cmd::run(release),
        cli::Command::Fmt => fmt::run(),
        cli::Command::Add { name } => pkg::add(&name),
        cli::Command::Remove { name } => pkg::remove(&name),
        cli::Command::Install => pkg::install(),
        cli::Command::Update => pkg::update(),
    };
    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
