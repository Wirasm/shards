use std::path::{Path, PathBuf};

use clap::ArgMatches;
use tracing::{error, info};

use kild_core::projects::{
    Project, ProjectError, ProjectsData, generate_project_id, load_projects, save_projects,
};

use crate::color;

pub(crate) fn handle_project_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        Some(("add", sub)) => handle_project_add(sub),
        Some(("list", sub)) => handle_project_list(sub),
        Some(("remove", sub)) => handle_project_remove(sub),
        Some(("info", sub)) => handle_project_info(sub),
        Some(("default", sub)) => handle_project_default(sub),
        _ => Err("Unknown project subcommand".into()),
    }
}

fn handle_project_add(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let path_str = matches.get_one::<String>("path").unwrap();
    let name = matches.get_one::<String>("name").cloned();

    info!(event = "cli.projects.add_started", path = path_str.as_str());

    let project = match Project::new(PathBuf::from(path_str), name) {
        Ok(p) => p,
        Err(e) => {
            error!(event = "cli.projects.add_failed", error = %e);
            eprintln!("{}", color::error(&e.to_string()));
            return Err(e.into());
        }
    };

    let mut data = load_projects();

    if data.projects.iter().any(|p| p.path() == project.path()) {
        let e = ProjectError::AlreadyExists;
        error!(event = "cli.projects.add_failed", error = %e);
        eprintln!("{}", color::error(&e.to_string()));
        return Err(e.into());
    }

    let canonical_path = project.path().to_path_buf();
    let project_name = project.name().to_string();

    // Auto-select first project
    if data.projects.is_empty() {
        data.active = Some(canonical_path.clone());
    }
    data.projects.push(project);
    save_projects(&data)?;

    println!(
        "{} {}",
        color::bold("Project registered:"),
        color::aurora(&project_name)
    );
    println!(
        "  {} {}",
        color::muted("path:"),
        color::ice(&canonical_path.display().to_string())
    );

    info!(
        event = "cli.projects.add_completed",
        name = project_name.as_str()
    );

    Ok(())
}

fn handle_project_list(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let json_output = matches.get_flag("json");

    info!(
        event = "cli.projects.list_started",
        json_output = json_output
    );

    let data = load_projects();

    if json_output {
        let projects_json: Vec<serde_json::Value> = data
            .projects
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name(),
                    "path": p.path().display().to_string(),
                    "id": generate_project_id(p.path()).to_string(),
                })
            })
            .collect();
        let active = data.active.as_ref().map(|a| a.display().to_string());
        let output = serde_json::json!({
            "projects": projects_json,
            "active": active,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if data.projects.is_empty() {
        println!("No projects registered.");
    } else {
        println!("{}", color::bold("Registered projects:"));
        for p in &data.projects {
            let id = generate_project_id(p.path());
            let active_marker = if data.active.as_deref() == Some(p.path()) {
                color::aurora("*")
            } else {
                " ".to_string()
            };
            println!(
                "  {} {}  {}  {}",
                active_marker,
                color::ice(id.as_ref()),
                color::aurora(p.name()),
                color::muted(&p.path().display().to_string()),
            );
        }
    }

    info!(
        event = "cli.projects.list_completed",
        count = data.projects.len()
    );

    Ok(())
}

fn handle_project_remove(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let identifier = matches.get_one::<String>("identifier").unwrap();

    info!(
        event = "cli.projects.remove_started",
        identifier = identifier.as_str()
    );

    let mut data = load_projects();
    let resolved_path = match resolve_identifier(&data, identifier) {
        Some(path) => path,
        None => {
            let msg = format!("Project not found: {}", identifier);
            eprintln!("{}", color::error(&msg));
            return Err(msg.into());
        }
    };

    let original_len = data.projects.len();
    data.projects
        .retain(|p| p.path() != resolved_path.as_path());

    if data.projects.len() == original_len {
        return Err(ProjectError::NotFound.into());
    }

    // Clear active if removed, select first remaining
    if data.active.as_deref() == Some(resolved_path.as_path()) {
        data.active = data.projects.first().map(|p| p.path().to_path_buf());
    }
    save_projects(&data)?;

    println!("{}", color::aurora("Project removed."));
    info!(event = "cli.projects.remove_completed", path = %resolved_path.display());

    Ok(())
}

fn handle_project_info(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let identifier = matches.get_one::<String>("identifier").unwrap();

    let data = load_projects();
    let resolved_path = match resolve_identifier(&data, identifier) {
        Some(path) => path,
        None => {
            let msg = format!("Project not found: {}", identifier);
            eprintln!("{}", color::error(&msg));
            return Err(msg.into());
        }
    };

    let project = match data
        .projects
        .iter()
        .find(|p| p.path() == resolved_path.as_path())
    {
        Some(p) => p,
        None => return Err("Internal error: project not found after resolution".into()),
    };

    let id = generate_project_id(project.path());
    let is_active = data.active.as_deref() == Some(project.path());

    println!("{}", color::aurora(project.name()));
    println!(
        "  {} {}",
        color::muted("path:  "),
        color::ice(&project.path().display().to_string())
    );
    println!("  {} {}", color::muted("id:    "), color::ice(id.as_ref()));
    println!(
        "  {} {}",
        color::muted("active:"),
        if is_active {
            color::aurora("yes")
        } else {
            color::muted("no")
        }
    );

    Ok(())
}

fn handle_project_default(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let identifier = matches.get_one::<String>("identifier").unwrap();

    info!(
        event = "cli.projects.default_started",
        identifier = identifier.as_str()
    );

    let mut data = load_projects();
    let resolved_path = match resolve_identifier(&data, identifier) {
        Some(path) => path,
        None => {
            let msg = format!("Project not found: {}", identifier);
            eprintln!("{}", color::error(&msg));
            return Err(msg.into());
        }
    };

    let project_name = data
        .projects
        .iter()
        .find(|p| p.path() == resolved_path.as_path())
        .map(|p| p.name().to_string())
        .unwrap_or_else(|| resolved_path.display().to_string());

    data.active = Some(resolved_path.clone());
    save_projects(&data)?;

    println!(
        "{} {}",
        color::bold("Default project set to:"),
        color::aurora(&project_name)
    );
    info!(event = "cli.projects.default_completed", path = %resolved_path.display());

    Ok(())
}

/// Resolve a user-supplied identifier (path string or hex project ID) to a canonical PathBuf.
/// Returns None if not found in the projects list.
fn resolve_identifier(data: &ProjectsData, identifier: &str) -> Option<PathBuf> {
    // Try as a filesystem path first
    if let Ok(canonical) = Path::new(identifier).canonicalize()
        && data
            .projects
            .iter()
            .any(|p| p.path() == canonical.as_path())
    {
        return Some(canonical);
    }
    // Fall back to project ID (hex hash)
    data.projects
        .iter()
        .find(|p| generate_project_id(p.path()).as_ref() == identifier)
        .map(|p| p.path().to_path_buf())
}
