use std::{
    collections::{HashMap, HashSet},
    result,
    time::Duration,
};

use octocrab::{issues::IssueHandler, Octocrab};

use crate::labels::Label;

pub fn authenticate_github(token: String) -> std::io::Result<Octocrab> {
    Octocrab::builder()
        .personal_token(token)
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

pub async fn push_labels(
    octocrab: &Octocrab,
    repo: &str,
    issues: IssueHandler<'_>,
    labels: Vec<Label>,
    all_allowed_labels: &HashSet<String>,
    repo_allowed_labels: Option<&HashSet<String>>,
) -> std::io::Result<()> {
    let mut github_labels = Vec::new();

    let mut page = 1u32;

    loop {
        eprintln!("Requesting labels for page {}", page);
        match issues.list_labels_for_repo().page(page).send().await {
            Ok(results) => {
                if results.items.is_empty() {
                    break;
                } else {
                    github_labels.extend(results.items)
                }
            }
            Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }

        page += 1;
    }

    let mut label_map = labels
        .iter()
        .map(|l| (&*l.name, l))
        .collect::<HashMap<&str, &Label>>();
    let mut labels_to_delete = Vec::new();

    for label in &github_labels {
        if all_allowed_labels.contains(&label.name) {
            continue;
        } else if let Some(repo_allowed_labels) = repo_allowed_labels {
            if repo_allowed_labels.contains(&label.name) {
                continue;
            }
        }
        if let Some(ilabel) = label_map.remove(&*label.name) {
        } else {
            labels_to_delete.push(label)
        }
    }

    for label in &labels {
        if label_map.contains_key(&*label.name) {
            eprintln!("Submitting label: {} ({})", label.name, label.colour);
            issues
                .create_label(&label.name, &label.colour, &label.description)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
    }

    for label in labels_to_delete {
        eprintln!("Removing label: {}", label.name);
        let route = format!("/repos/{repo}/labels/{}", label.name);

        let name = label.name.replace(" ", "%20");

        issues
            .delete_label(name)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    }

    Ok(())
}
