use std::path::PathBuf;

use serde::Deserialize as _;

mod data;
mod github;
mod labels;

#[tokio::main]
async fn main() {
    let mut args = std::env::args();

    let prg_name = args.next().unwrap();

    match realmain().await {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}: {}", prg_name, e);
            std::process::exit(1)
        }
    }
}

async fn realmain() -> std::io::Result<()> {
    let data_dir = std::env::var_os("DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"));

    let labels_file = std::env::var_os("LABELS_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut buf = data_dir.clone();
            buf.push("labels.toml");
            buf
        });

    let repos_file = std::env::var_os("REPOS_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut buf = data_dir.clone();
            buf.push("repos.toml");
            buf
        });

    let token = match std::env::var("GITHUB_TOKEN") {
        Ok(tok) => Some(tok),
        Err(std::env::VarError::NotPresent) => None,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)),
    };

    let labels = std::fs::read_to_string(labels_file)?;
    let repos = std::fs::read_to_string(repos_file)?;
    let labels = toml::de::Deserializer::new(&labels);
    let labels = data::LabelsFile::deserialize(labels)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let repos = toml::de::Deserializer::new(&repos);
    let repos = data::ReposFile::deserialize(repos)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if token.is_none() {
        println!("No GITHUB_TOKEN specified in envirnonment. Building label lists only (dry run)");
    }

    let octocrab = token.map(github::authenticate_github).transpose()?;
    let mut futures = Vec::new();
    for (key, repo_name) in &repos.repos {
        let repo_labels = labels::build_labels(key, &labels)?;
        println!("Labels for repo: {}", repo_name);
        for label in &repo_labels {
            println!("\t{}", label.name)
        }

        if let Some(crab) = &octocrab {
            let (l, r) = repo_name.split_once("/").unwrap();
            let issues = crab.issues(l, r);
            futures.push(github::push_labels(
                crab,
                repo_name,
                issues,
                repo_labels,
                &repos.allowed_labels.all,
                repos.allowed_labels.by_repo.get(key),
            ));
        }
    }

    let futures = futures::future::join_all(futures);

    futures.await.into_iter().collect::<std::io::Result<()>>()?;

    Ok(())
}
