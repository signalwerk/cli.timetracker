use crate::api::{ApiClient, Project, TimeEntry};
use crate::logger::Logger;
use anyhow::Result;
use chrono::{DateTime, Utc, Local};
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use std::cmp::Reverse;

pub async fn add_project(
    api_client: &ApiClient,
    logger: &Logger,
    slug: &str,
    name: Option<String>,
    description: Option<String>,
) -> Result<()> {
    let project_name = name.unwrap_or_else(|| slug.to_string());
    let project_description = description.unwrap_or_else(|| format!("Project {}", slug));
    
    let project = Project {
        name: project_name.clone(),
        slug: slug.to_string(),
        description: project_description.clone(),
    };

    match api_client.add_project(project).await {
        Ok(_) => {
            println!("✅ Project '{}' added successfully", slug);
            logger.log(&format!("Added project: {} ({})", slug, project_name)).await?;
        }
        Err(e) => {
            eprintln!("❌ Failed to add project: {}", e);
            logger.log(&format!("Failed to add project {}: {}", slug, e)).await?;
        }
    }

    Ok(())
}

pub async fn start_tracking(
    api_client: &ApiClient,
    logger: &Logger,
    project_slug: &str,
    description: Option<String>,
) -> Result<()> {
    // Check current status before starting
    match api_client.get_time_entries(project_slug).await {
        Ok(entries) => {
            if is_project_running(&entries) {
                eprintln!("❌ Project '{}' is already running!", project_slug);
                eprintln!("   💡 Use 'timetracker end {}' to stop tracking first", project_slug);
                logger.log(&format!("Attempted to start already running project: {}", project_slug)).await?;
                return Ok(());
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to check project status: {}", e);
            logger.log(&format!("Failed to check status before starting {}: {}", project_slug, e)).await?;
            return Ok(());
        }
    }

    let timestamp = Utc::now().timestamp();
    
    let entry = TimeEntry {
        timestamp,
        entry_type: "start".to_string(),
        description: description.clone(),
    };

    match api_client.add_time_entry(project_slug, entry).await {
        Ok(_) => {
            println!("⏱️  Started tracking time for project '{}'", project_slug);
            if let Some(desc) = &description {
                println!("   Description: {}", desc);
            }
            let log_msg = if let Some(desc) = description {
                format!("Started tracking time for project '{}' with description: {}", project_slug, desc)
            } else {
                format!("Started tracking time for project '{}'", project_slug)
            };
            logger.log(&log_msg).await?;
        }
        Err(e) => {
            eprintln!("❌ Failed to start tracking: {}", e);
            logger.log(&format!("Failed to start tracking for {}: {}", project_slug, e)).await?;
        }
    }

    Ok(())
}

pub async fn end_tracking(
    api_client: &ApiClient,
    logger: &Logger,
    project_slug: &str,
    description: String,
) -> Result<()> {
    // Check current status before stopping
    match api_client.get_time_entries(project_slug).await {
        Ok(entries) => {
            if entries.is_empty() {
                eprintln!("❌ No time entries found for project '{}'!", project_slug);
                eprintln!("   💡 Use 'timetracker start {}' to start tracking first", project_slug);
                logger.log(&format!("Attempted to stop project with no entries: {}", project_slug)).await?;
                return Ok(());
            }
            
            if !is_project_running(&entries) {
                eprintln!("❌ Project '{}' is not currently running!", project_slug);
                eprintln!("   💡 Use 'timetracker start {}' to start tracking first", project_slug);
                logger.log(&format!("Attempted to stop already stopped project: {}", project_slug)).await?;
                return Ok(());
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to check project status: {}", e);
            logger.log(&format!("Failed to check status before stopping {}: {}", project_slug, e)).await?;
            return Ok(());
        }
    }

    let timestamp = Utc::now().timestamp();
    
    let entry = TimeEntry {
        timestamp,
        entry_type: "end".to_string(),
        description: Some(description.clone()),
    };

    match api_client.add_time_entry(project_slug, entry).await {
        Ok(_) => {
            println!("⏹️  Stopped tracking time for project '{}'", project_slug);
            println!("   What was done: {}", description);
            let log_msg = format!("Stopped tracking time for project '{}' with description: {}", project_slug, description);
            logger.log(&log_msg).await?;
        }
        Err(e) => {
            eprintln!("❌ Failed to stop tracking: {}", e);
            logger.log(&format!("Failed to stop tracking for {}: {}", project_slug, e)).await?;
        }
    }

    Ok(())
}

pub async fn list_projects(api_client: &ApiClient, logger: &Logger) -> Result<()> {
    logger.log("Listed all projects").await?;
    
    match api_client.get_projects().await {
        Ok(projects) => {
            if projects.is_empty() {
                println!("📋 No projects found");
            } else {
                println!("📋 Projects:");
                for project in projects {
                    println!("  • {} ({}) - {}", project.name, project.slug, project.description);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to list projects: {}", e);
            logger.log(&format!("Failed to list projects: {}", e)).await?;
        }
    }

    Ok(())
}

pub async fn list_times(api_client: &ApiClient, logger: &Logger, project_slug: &str) -> Result<()> {
    logger.log(&format!("Listed times for project '{}'", project_slug)).await?;
    
    match api_client.get_time_entries(project_slug).await {
        Ok(entries) => {
            if entries.is_empty() {
                println!("⏱️  No time entries found for project '{}'", project_slug);
            } else {
                println!("⏱️  Time entries for project '{}':", project_slug);
                for entry in entries {
                    let utc_datetime = DateTime::from_timestamp(entry.timestamp, 0)
                        .unwrap_or_else(|| Utc::now());
                    let local_datetime = utc_datetime.with_timezone(&Local);
                    let type_icon = if entry.entry_type == "start" { "▶️" } else { "⏹️" };
                    
                    print!("  {} {} {} [ts:{}]", 
                           type_icon, 
                           entry.entry_type.to_uppercase(), 
                           local_datetime.format("%Y-%m-%d %H:%M:%S %Z"),
                           entry.timestamp);
                    if let Some(desc) = &entry.description {
                        print!(" - {}", desc);
                    }
                    println!();
                }
                println!("");
                println!("💡 To delete a specific entry: timetracker delete times {} --timestamp <ts>", project_slug);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to list times: {}", e);
            logger.log(&format!("Failed to list times for {}: {}", project_slug, e)).await?;
        }
    }

    Ok(())
}

pub async fn show_total(api_client: &ApiClient, logger: &Logger, project_slug: &str) -> Result<()> {
    logger.log(&format!("Calculated total time for project '{}'", project_slug)).await?;
    
    match api_client.get_time_entries(project_slug).await {
        Ok(entries) => {
            let total_seconds = calculate_total_time(&entries);
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            
            println!("📊 Total time for project '{}': {}h {}m {}s", 
                     project_slug, hours, minutes, seconds);
        }
        Err(e) => {
            eprintln!("❌ Failed to calculate total time: {}", e);
            logger.log(&format!("Failed to calculate total time for {}: {}", project_slug, e)).await?;
        }
    }

    Ok(())
}

pub async fn show_status(api_client: &ApiClient, logger: &Logger, project_slug: &str) -> Result<()> {
    logger.log(&format!("Checked status for project '{}'", project_slug)).await?;
    
    match api_client.get_time_entries(project_slug).await {
        Ok(entries) => {
            let is_running = is_project_running(&entries);
            
            if is_running {
                println!("🟢 Project '{}' is currently running", project_slug);
                // Find the last start entry
                if let Some(last_start) = entries.iter()
                    .filter(|e| e.entry_type == "start")
                    .max_by_key(|e| e.timestamp) {
                    let utc_start_time = DateTime::from_timestamp(last_start.timestamp, 0)
                        .unwrap_or_else(|| Utc::now());
                    let local_start_time = utc_start_time.with_timezone(&Local);
                    let duration = Utc::now().timestamp() - last_start.timestamp;
                    let hours = duration / 3600;
                    let minutes = (duration % 3600) / 60;
                    println!("   Started at: {}", local_start_time.format("%Y-%m-%d %H:%M:%S %Z"));
                    println!("   Running for: {}h {}m", hours, minutes);
                }
            } else {
                println!("🔴 Project '{}' is not currently running", project_slug);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to check status: {}", e);
            logger.log(&format!("Failed to check status for {}: {}", project_slug, e)).await?;
        }
    }

    Ok(())
}

fn calculate_total_time(entries: &[TimeEntry]) -> i64 {
    let mut total = 0i64;
    let mut start_time: Option<i64> = None;
    
    // Sort entries by timestamp
    let mut sorted_entries = entries.to_vec();
    sorted_entries.sort_by_key(|e| e.timestamp);
    
    for entry in sorted_entries {
        match entry.entry_type.as_str() {
            "start" => {
                start_time = Some(entry.timestamp);
            }
            "end" => {
                if let Some(start) = start_time {
                    total += entry.timestamp - start;
                    start_time = None;
                }
            }
            _ => {} // Ignore unknown types
        }
    }
    
    total
}

fn is_project_running(entries: &[TimeEntry]) -> bool {
    if entries.is_empty() {
        return false;
    }
    
    // Sort entries by timestamp and get the last one
    let mut sorted_entries = entries.to_vec();
    sorted_entries.sort_by_key(|e| e.timestamp);
    
    if let Some(last_entry) = sorted_entries.last() {
        last_entry.entry_type == "start"
    } else {
        false
    }
}

pub async fn export_data(api_client: &ApiClient, logger: &Logger, output_dir: &str, filename_template: &str) -> Result<()> {
    logger.log(&format!("Exporting data to directory: {} with template: {}", output_dir, filename_template)).await?;
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;
    
    // Generate export timestamp for filename templates
    let export_timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    
    match api_client.get_all_keys().await {
        Ok(keys) => {
            let keys_count = keys.len();
            println!("📁 Exporting {} keys to {} using template '{}'", keys_count, output_dir, filename_template);
            
            for key_data in keys {
                // Generate filename from template
                let filename = generate_filename_from_template(
                    filename_template, 
                    &key_data.key, 
                    &export_timestamp
                );
                let file_path = Path::new(output_dir).join(filename);
                
                // Parse the value (which is stored as a JSON string) and pretty print it
                let value = match serde_json::from_str::<serde_json::Value>(&key_data.value.as_str().unwrap_or("{}")) {
                    Ok(parsed) => parsed,
                    Err(_) => key_data.value.clone(),
                };
                
                let pretty_json = serde_json::to_string_pretty(&value)?;
                fs::write(&file_path, pretty_json)?;
                
                println!("  ✅ Exported: {} -> {}", key_data.key, file_path.display());
            }
            
            logger.log(&format!("Successfully exported {} keys", keys_count)).await?;
        }
        Err(e) => {
            eprintln!("❌ Failed to export data: {}", e);
            logger.log(&format!("Failed to export data: {}", e)).await?;
        }
    }
    
    Ok(())
}

fn generate_filename_from_template(template: &str, key: &str, timestamp: &str) -> String {
    let mut filename = template.to_string();
    
    // Replace {key-name} placeholder
    let safe_key_name = key.replace("/", "_");
    filename = filename.replace("{key-name}", &safe_key_name);
    
    // Replace {timestamp} placeholder
    filename = filename.replace("{timestamp}", timestamp);
    
    // Replace {project-name} placeholder
    let project_name = extract_project_name_from_key(key);
    filename = filename.replace("{project-name}", &project_name);
    
    filename
}

fn extract_project_name_from_key(key: &str) -> String {
    // For keys like "projects/TypeRoof", extract "TypeRoof"
    // For keys like "projects", return "all_projects"
    // For other keys, return "general"
    
    if key.starts_with("projects/") {
        if let Some(project_slug) = key.strip_prefix("projects/") {
            if !project_slug.is_empty() {
                return project_slug.to_string();
            }
        }
    } else if key == "projects" {
        return "all_projects".to_string();
    }
    
    "general".to_string()
}

pub async fn delete_project(api_client: &ApiClient, logger: &Logger, slug: &str) -> Result<()> {
    logger.log(&format!("Deleting project: {}", slug)).await?;
    
    match api_client.delete_project(slug).await {
        Ok(_) => {
            println!("🗑️  Successfully deleted project '{}' and all its time entries", slug);
            logger.log(&format!("Successfully deleted project: {}", slug)).await?;
        }
        Err(e) => {
            eprintln!("❌ Failed to delete project: {}", e);
            logger.log(&format!("Failed to delete project {}: {}", slug, e)).await?;
        }
    }
    
    Ok(())
}

pub async fn delete_times(
    api_client: &ApiClient, 
    logger: &Logger, 
    project_slug: &str, 
    timestamp: Option<i64>, 
    all: bool
) -> Result<()> {
    if let Some(ts) = timestamp {
        // Delete specific timestamp - this is safer
        logger.log(&format!("Deleting time entry with timestamp {} for project: {}", ts, project_slug)).await?;
        
        match api_client.delete_time_entry_by_timestamp(project_slug, ts).await {
            Ok(_) => {
                let utc_datetime = DateTime::from_timestamp(ts, 0)
                    .unwrap_or_else(|| Utc::now());
                let local_datetime = utc_datetime.with_timezone(&Local);
                println!("🗑️  Successfully deleted time entry from {} for project '{}'", 
                         local_datetime.format("%Y-%m-%d %H:%M:%S %Z"), project_slug);
                logger.log(&format!("Successfully deleted time entry {} for project: {}", ts, project_slug)).await?;
            }
            Err(e) => {
                eprintln!("❌ Failed to delete time entry: {}", e);
                logger.log(&format!("Failed to delete time entry {} for {}: {}", ts, project_slug, e)).await?;
            }
        }
    } else if all {
        // Delete ALL entries - this is DANGEROUS!
        show_danger_warning_and_confirm(project_slug).await?;
        
        logger.log(&format!("⚠️ DANGER: Deleting ALL time entries for project: {}", project_slug)).await?;
        
        match api_client.delete_project_times(project_slug).await {
            Ok(_) => {
                println!("🗑️  Successfully deleted ALL time entries for project '{}'", project_slug);
                logger.log(&format!("⚠️ Successfully deleted ALL time entries for project: {}", project_slug)).await?;
            }
            Err(e) => {
                eprintln!("❌ Failed to delete time entries: {}", e);
                logger.log(&format!("Failed to delete all time entries for {}: {}", project_slug, e)).await?;
            }
        }
    } else {
        // No timestamp provided and --all not specified
        eprintln!("❌ Safety Error: You must specify either:");
        eprintln!("   • A specific timestamp to delete: --timestamp <unix_timestamp>");
        eprintln!("   • Use --all flag to delete ALL entries (DANGEROUS!)");
        eprintln!("");
        eprintln!("💡 Tip: Use 'timetracker times {}' to see all timestamps first", project_slug);
        return Ok(());
    }
    
    Ok(())
}

async fn show_danger_warning_and_confirm(project_slug: &str) -> Result<()> {
    println!("");
    println!("🚨 ⚠️  DANGER WARNING ⚠️  🚨");
    println!("═══════════════════════════════════════════════════════════════");
    println!("  You are about to DELETE ALL TIME ENTRIES for project:");
    println!("  📁 '{}'", project_slug);
    println!("");
    println!("  ❌ This action CANNOT be undone!");
    println!("  ❌ All tracking history will be permanently lost!");
    println!("  ❌ This includes start/stop times and descriptions!");
    println!("");
    println!("  💡 Consider using --timestamp to delete specific entries instead");
    println!("  💡 Use 'timetracker export' to backup data first");
    println!("═══════════════════════════════════════════════════════════════");
    println!("");
    
    print!("Are you absolutely sure? Type 'DELETE ALL' to confirm: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    
    if input != "DELETE ALL" {
        println!("❌ Operation cancelled. Data is safe.");
        return Err(anyhow::anyhow!("User cancelled dangerous operation"));
    }
    
    println!("⚠️  Proceeding with deletion...");
    Ok(())
}

pub async fn edit_time_entry(api_client: &ApiClient, logger: &Logger, project_slug: &str) -> Result<()> {
    logger.log(&format!("Editing time entry for project '{}'", project_slug)).await?;
    
    // Get time entries for the project
    let entries = match api_client.get_time_entries(project_slug).await {
        Ok(entries) => {
            if entries.is_empty() {
                println!("❌ No time entries found for project '{}'", project_slug);
                return Ok(());
            }
            entries
        }
        Err(e) => {
            eprintln!("❌ Failed to get time entries: {}", e);
            logger.log(&format!("Failed to get time entries for {}: {}", project_slug, e)).await?;
            return Ok(());
        }
    };
    
    // Sort entries by timestamp (newest first) and take last 5
    let mut sorted_entries = entries.clone();
    sorted_entries.sort_by_key(|e| Reverse(e.timestamp));
    let recent_entries: Vec<_> = sorted_entries.into_iter().take(5).collect();
    
    // Display the recent entries
    println!("📝 Recent time entries for project '{}':", project_slug);
    println!("");
    for (index, entry) in recent_entries.iter().enumerate() {
        let utc_datetime = DateTime::from_timestamp(entry.timestamp, 0)
            .unwrap_or_else(|| Utc::now());
        let local_datetime = utc_datetime.with_timezone(&Local);
        let type_icon = if entry.entry_type == "start" { "▶️" } else { "⏹️" };
        let description = entry.description.as_ref()
            .map(|d| format!(" - {}", d))
            .unwrap_or_else(|| " - (no description)".to_string());
        
                 println!("  {}. {} {} {}{}",
                 index + 1,
                 type_icon,
                 entry.entry_type.to_uppercase(),
                 local_datetime.format("%Y-%m-%d %H:%M:%S %Z"),
                 description);
    }
    
    println!("");
    print!("Select entry to edit (1-{}), or 'q' to quit: ", recent_entries.len());
    io::stdout().flush()?;
    
    // Get user selection
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    
    if input.eq_ignore_ascii_case("q") {
        println!("❌ Edit cancelled");
        return Ok(());
    }
    
    let selection: usize = match input.parse::<usize>() {
        Ok(num) if num >= 1 && num <= recent_entries.len() => num - 1,
        _ => {
            println!("❌ Invalid selection. Please enter a number between 1 and {}", recent_entries.len());
            return Ok(());
        }
    };
    
    let selected_entry = &recent_entries[selection];
    
    // Show current description and allow editing
    println!("");
    println!("Selected entry:");
    let utc_datetime = DateTime::from_timestamp(selected_entry.timestamp, 0)
        .unwrap_or_else(|| Utc::now());
    let local_datetime = utc_datetime.with_timezone(&Local);
    let type_icon = if selected_entry.entry_type == "start" { "▶️" } else { "⏹️" };
    println!("  {} {} {}", type_icon, selected_entry.entry_type.to_uppercase(), local_datetime.format("%Y-%m-%d %H:%M:%S %Z"));
    
    let current_desc = selected_entry.description.as_ref()
        .map(|d| d.as_str())
        .unwrap_or("(no description)");
    println!("  Current description: {}", current_desc);
    println!("");
    
    print!("Enter new description (press Enter to keep current, or type 'CLEAR' to remove): ");
    io::stdout().flush()?;
    
    let mut new_description = String::new();
    io::stdin().read_line(&mut new_description)?;
    let new_description = new_description.trim();
    
    let updated_description = if new_description.is_empty() {
        // Keep current description
        selected_entry.description.clone()
    } else if new_description.eq_ignore_ascii_case("CLEAR") {
        // Clear description
        None
    } else {
        // Set new description
        Some(new_description.to_string())
    };
    
    // Update the entry via API
    match api_client.update_time_entry_by_timestamp(project_slug, selected_entry.timestamp, updated_description.clone()).await {
        Ok(_) => {
            let desc_text = updated_description.as_ref()
                .map(|d| format!("'{}'", d))
                .unwrap_or_else(|| "(no description)".to_string());
            println!("✅ Successfully updated description to: {}", desc_text);
            logger.log(&format!("Updated time entry {} description for project {}", selected_entry.timestamp, project_slug)).await?;
        }
        Err(e) => {
            eprintln!("❌ Failed to update description: {}", e);
            logger.log(&format!("Failed to update time entry {} for {}: {}", selected_entry.timestamp, project_slug, e)).await?;
        }
    }
    
    Ok(())
} 