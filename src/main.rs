use anyhow::{anyhow, Result};
use clap::{arg, command};
use commit::CommitRequest;
use providers::{get_repository, Provider};
use templaters::{mutate, Mutation};

mod commit;
mod providers;
mod repository;
mod templaters;

struct Task {
    pub provider: Provider,
    pub changes: Vec<Mutation>,
    pub branch: String,
    pub author: String,
    pub message: String,
}

fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::new().filter("SHIPIT_LOG"));

    let matches = command!()
        .arg(arg!(-p --provider <provider> "Provider info (as JSON)").env("SHIPIT_PROVIDER"))
        .arg(arg!(-c --changeset <changes> "Changes to apply (as JSON)").env("SHIPIT_CHANGES"))
        .arg(
            arg!(-a --author <author> "Commit author (as 'name <email>')")
                .env("SHIPIT_AUTHOR")
                .default_value("shipit"),
        )
        .arg(
            arg!(-b --branch <branch> "Branch to commit to")
                .env("SHIPIT_BRANCH")
                .default_value("main"),
        )
        .arg(
            arg!(-m --message <message> "Commit message")
                .env("SHIPIT_MESSAGE")
                .default_value("Update deployment"),
        )
        .get_matches();

    let config = Task {
        provider: serde_json::from_str(
            matches
                .get_one::<String>("provider")
                .ok_or_else(|| anyhow!("Missing provider info (--provider or SHIPIT_PROVIDER)"))?
                .as_str(),
        )?,
        changes: serde_json::from_str(
            matches
                .get_one::<String>("changeset")
                .ok_or_else(|| anyhow!("Missing changeset (--changeset or SHIPIT_CHANGES)"))?
                .as_str(),
        )?,
        branch: matches.get_one::<String>("branch").unwrap().into(),
        author: matches.get_one::<String>("author").unwrap().into(),
        message: matches.get_one::<String>("message").unwrap().into(),
    };

    log::debug!("using provider {}", config.provider.name());

    let mut repo = get_repository(config.provider);

    log::debug!(
        "computing changes: {:?} to branch {}",
        config.changes,
        config.branch
    );

    let changes = mutate(&*repo, &config.branch, &config.changes)?;

    let commit = CommitRequest {
        branch: config.branch,
        author: config.author,
        message: config.message,
        files: changes,
    };

    repo.commit(commit)?;

    log::info!("done!");

    Ok(())
}
