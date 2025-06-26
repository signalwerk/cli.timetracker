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
    /// Add a new project
    #[command(name = "project")]
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// Start tracking time for a project
    Start {
        /// Project slug
        project_slug: String,
        /// Optional description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// End tracking time for a project
    End {
        /// Project slug
        project_slug: String,
        /// Optional description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List all projects
    List,
    /// List times for a project
    Times {
        /// Project slug
        project_slug: String,
    },
    /// Show total time for a project
    Total {
        /// Project slug
        project_slug: String,
    },
    /// Check if a project is currently running
    Status {
        /// Project slug
        project_slug: String,
    },
    /// Export all data as JSON files
    Export {
        /// Output directory
        #[arg(short, long, default_value = "./DATA")]
        output_dir: String,
    },
    /// Delete projects or time entries
    Delete {
        #[command(subcommand)]
        target: DeleteTarget,
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
}

#[derive(Subcommand)]
enum DeleteTarget {
    /// Delete a project
    Project {
        /// Project slug
        slug: String,
    },
    /// Delete time entries for a project
    Times {
        /// Project slug
        project_slug: String,
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
            }
        }
        Commands::Start { project_slug, description } => {
            commands::start_tracking(&api_client, &logger, &project_slug, description).await?;
        }
        Commands::End { project_slug, description } => {
            commands::end_tracking(&api_client, &logger, &project_slug, description).await?;
        }
        Commands::List => {
            commands::list_projects(&api_client, &logger).await?;
        }
        Commands::Times { project_slug } => {
            commands::list_times(&api_client, &logger, &project_slug).await?;
        }
        Commands::Total { project_slug } => {
            commands::show_total(&api_client, &logger, &project_slug).await?;
        }
        Commands::Status { project_slug } => {
            commands::show_status(&api_client, &logger, &project_slug).await?;
        }
        Commands::Export { output_dir } => {
            commands::export_data(&api_client, &logger, &output_dir).await?;
        }
        Commands::Delete { target } => {
            match target {
                DeleteTarget::Project { slug } => {
                    commands::delete_project(&api_client, &logger, &slug).await?;
                }
                DeleteTarget::Times { project_slug, timestamp, all } => {
                    commands::delete_times(&api_client, &logger, &project_slug, timestamp, all).await?;
                }
            }
        }
    }

    Ok(())
} 