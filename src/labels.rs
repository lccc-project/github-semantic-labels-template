use std::collections::HashMap;

use crate::data::{FormatString, LabelGroupSpec, LabelsFile};

pub struct Label {
    pub name: String,
    pub colour: String,
    pub description: String,
}

fn gather_labels<'a>(
    for_repo: &str,
    into: &mut Vec<Label>,
    colour: Option<&String>,
    description: Option<&'a FormatString>,
    pattern: &mut Vec<&'a FormatString>,
    keys: &mut HashMap<String, String>,
    spec: &'a LabelGroupSpec,
) -> std::io::Result<()> {
    if let Some(repos) = &spec.label_spec.repos {
        if !repos.contains(for_repo) {
            return Ok(());
        }
    }

    pattern.push(&spec.pattern);

    let colour = spec.label_spec.colour.as_ref().or(colour);
    let description = spec.label_spec.description.as_ref().or(description);

    for label in &spec.labels {
        keys.insert("stem".to_string(), label.to_string());
        let colour = colour
            .map(String::from)
            .unwrap_or_else(|| "7f7f7f".to_string());
        let description = description
            .map(|f| -> std::io::Result<_> {
                let mut str = String::new();
                f.eval(label, keys, &mut str)?;

                Ok(str)
            })
            .transpose()?
            .unwrap_or_else(|| String::new());

        let mut name = label.to_string();
        let mut keys = keys.clone();

        for (n, pat) in pattern.iter().rev().enumerate() {
            let stem = core::mem::take(&mut name);

            pat.eval(&stem, &keys, &mut name)?;

            keys.insert(n.to_string(), stem);
        }

        let label = Label {
            colour,
            description,
            name,
        };
        into.push(label);
    }

    for (label, spec) in &spec.elaborated_labels {
        if let Some(repos) = &spec.repos {
            if !repos.contains(for_repo) {
                continue;
            }
        }

        keys.insert("stem".to_string(), label.to_string());

        let colour = spec
            .colour
            .as_ref()
            .or(colour)
            .map(String::from)
            .unwrap_or_else(|| "#7f7f7f".to_string());
        let description = spec
            .description
            .as_ref()
            .or(description)
            .map(|f| -> std::io::Result<_> {
                let mut str = String::new();
                f.eval(label, keys, &mut str)?;

                Ok(str)
            })
            .transpose()?
            .unwrap_or_else(|| String::new());

        let mut name = label.to_string();
        let mut keys = keys.clone();

        for (n, pat) in pattern.iter().rev().enumerate() {
            let stem = core::mem::take(&mut name);

            pat.eval(&stem, &keys, &mut name)?;

            keys.insert(n.to_string(), stem);
        }

        let label = Label {
            colour,
            description,
            name,
        };
        into.push(label);
    }

    keys.remove("stem");

    for (name, spec) in &spec.subgroups {
        gather_labels(for_repo, into, colour, description, pattern, keys, spec)?;
    }

    pattern.pop();

    Ok(())
}

pub fn build_labels(repo: &str, labels: &LabelsFile) -> std::io::Result<Vec<Label>> {
    let mut output = Vec::new();

    for (name, spec) in &labels.groups {
        let mut keys = HashMap::new();
        let mut patterns = vec![];
        gather_labels(
            repo,
            &mut output,
            None,
            None,
            &mut patterns,
            &mut keys,
            spec,
        )?;
    }

    output.sort_by_key(|l| l.name.clone());

    Ok(output)
}
