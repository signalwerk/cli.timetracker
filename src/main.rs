use clap::{Parser, Subcommand};
use anyhow::Result;

mod api;
mod logger;
mod commands;

use api::ApiClient;
use logger::Logger;

/// A minimal CLI tool for time tracking
#[derive(Parser)]
#[command(name = "timetracker")]
#[command(about = "A CLI tool for time tracking with REST API backend")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Project management operations
    #[command(name = "project")]
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// Time tracking operations
    #[command(name = "time")]
    Time {
        #[command(subcommand)]
        action: TimeAction,
    },
    /// Export all data as JSON files
    Export {
        /// Output directory
        #[arg(short, long, default_value = "./DATA")]
        output_dir: String,
        /// Filename template with placeholders: {project-name}, {timestamp}, {key-name}
        #[arg(short = 't', long, default_value = "{timestamp}_{key-name}.json")]
        filename_template: String,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// Add a new project
    Add {
        /// Project slug
        slug: String,
        /// Project name
        #[arg(short, long)]
        name: Option<String>,
        /// Project description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List all projects
    List,
    /// Edit project details (name, description, slug)
    Edit {
        /// Project slug (optional - if not provided, shows selection list)
        #[arg()]
        project: Option<String>,
    },
    /// Delete a project
    Delete {
        /// Project slug (optional - if not provided, shows selection list)
        #[arg()]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum TimeAction {
    /// Start tracking time for a project
    Start {
        /// Project slug (optional - if not provided, shows selection list)
        project: Option<String>,
        /// Optional description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Stop tracking time for a project
    Stop {
        /// Project slug (optional - if not provided, shows selection list)
        project: Option<String>,
        /// Description of what was accomplished during this time session
        #[arg(short, long)]
        description: String,
    },
    /// Check if a project is currently running
    Status {
        /// Project slug (optional - if not provided, shows selection list)
        project: Option<String>,
    },
    /// List time entries for a project
    List {
        /// Project slug (optional - if not provided, shows selection list)
        project: Option<String>,
    },
    /// Show total time for a project
    Total {
        /// Project slug (optional - if not provided, shows selection list)
        project: Option<String>,
    },
    /// Edit the description of a time entry
    Edit {
        /// Project slug (optional - if not provided, shows selection list)
        project: Option<String>,
    },
    /// Delete time entries for a project
    Delete {
        /// Project slug (optional - if not provided, shows selection list)
        project: Option<String>,
        /// Delete by specific timestamp (safer than deleting all)
        #[arg(short, long)]
        timestamp: Option<i64>,
        /// Force delete ALL time entries (DANGEROUS! Requires confirmation)
        #[arg(long)]
        all: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let logger = Logger::new()?;
    let mut api_client = ApiClient::new()?;

    // Attempt to authenticate
    if let Err(e) = api_client.authenticate().await {
        eprintln!("Warning: Authentication failed: {}. Some commands may not work.", e);
        logger.log(&format!("Authentication failed: {}", e)).await?;
    }

    match cli.command {
        Commands::Project { action } => {
            match action {
                ProjectAction::Add { slug, name, description } => {
                    commands::add_project(&api_client, &logger, &slug, name, description).await?;
                }
                ProjectAction::List => {
                    commands::list_projects(&api_client, &logger).await?;
                }
                ProjectAction::Edit { project } => {
                    if let Some(project_slug) = project {
                        commands::edit_project_by_slug(&api_client, &logger, &project_slug).await?;
                    } else {
                        commands::edit_project_details(&api_client, &logger).await?;
                    }
                }
                ProjectAction::Delete { project } => {
                    if let Some(project_slug) = project {
                        commands::delete_project_with_confirmation(&api_client, &logger, &project_slug).await?;
                    } else {
                        commands::delete_project_with_selection(&api_client, &logger).await?;
                    }
                }
            }
        }
        Commands::Time { action } => {
            match action {
                TimeAction::Start { project, description } => {
                    if let Some(project_slug) = project {
                        commands::start_tracking(&api_client, &logger, &project_slug, description).await?;
                    } else {
                        commands::start_tracking_with_selection(&api_client, &logger, description).await?;
                    }
                }
                TimeAction::Stop { project, description } => {
                    if let Some(project_slug) = project {
                        commands::end_tracking(&api_client, &logger, &project_slug, description).await?;
                    } else {
                        commands::end_tracking_with_selection(&api_client, &logger, description).await?;
                    }
                }
                TimeAction::Status { project } => {
                    if let Some(project_slug) = project {
                        commands::show_status(&api_client, &logger, &project_slug).await?;
                    } else {
                        commands::show_status_with_selection(&api_client, &logger).await?;
                    }
                }
                TimeAction::List { project } => {
                    if let Some(project_slug) = project {
                        commands::list_times(&api_client, &logger, &project_slug).await?;
                    } else {
                        commands::list_times_with_selection(&api_client, &logger).await?;
                    }
                }
                TimeAction::Total { project } => {
                    if let Some(project_slug) = project {
                        commands::show_total(&api_client, &logger, &project_slug).await?;
                    } else {
                        commands::show_total_with_selection(&api_client, &logger).await?;
                    }
                }
                TimeAction::Edit { project } => {
                    if let Some(project_slug) = project {
                        commands::edit_time_entry(&api_client, &logger, &project_slug).await?;
                    } else {
                        commands::edit_time_entry_with_selection(&api_client, &logger).await?;
                    }
                }
                TimeAction::Delete { project, timestamp, all } => {
                    if let Some(project_slug) = project {
                        commands::delete_times(&api_client, &logger, &project_slug, timestamp, all).await?;
                    } else {
                        commands::delete_times_with_selection(&api_client, &logger, timestamp, all).await?;
                    }
                }
            }
        }
        Commands::Export { output_dir, filename_template } => {
            commands::export_data(&api_client, &logger, &output_dir, &filename_template).await?;
        }
    }

    Ok(())
} 