# Timetracker CLI

A minimal CLI tool for time tracking using a REST API backend.

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/timetracker`.

## Configuration

The tool uses hardcoded credentials for the API. In a production environment, you would want to implement proper credential management.

- API Base URL: `https://kv.srv.signalwerk.ch/timetracker`
- Default credentials: username: "user", password: "pass"

## Logging

All actions are logged for debugging purposes:
- **Development mode**: `timetracker.log` in the current directory
- **Production mode**: `~/.timetracker.log` in the user's home directory

Development mode is automatically detected by the presence of `Cargo.toml` in the current directory.

## Usage

### Add a Project

```bash
timetracker project add <project-slug> --name "Project Name" --description "Project Description"
```

Example:
```bash
timetracker project add my-website --name "My Website" --description "Personal site"
```

### Start Time Tracking

```bash
timetracker start <project-slug> [--description "What you're working on"]
```

Example:
```bash
timetracker start my-website --description "Working on homepage"
```

### Stop Time Tracking

```bash
timetracker end <project-slug> [--description "What you completed"]
```

Example:
```bash
timetracker end my-website
```

### List All Projects

```bash
timetracker list
```

### List Time Entries for a Project

```bash
timetracker times <project-slug>
```

Example:
```bash
timetracker times my-website
```

### Show Total Time for a Project

```bash
timetracker total <project-slug>
```

Example:
```bash
timetracker total my-website
```

### Check Project Status

```bash
timetracker status <project-slug>
```

Example:
```bash
timetracker status my-website
```

### Export All Data

Export all keys and values from the server as pretty-formatted JSON files.

```bash
timetracker export [--output-dir <directory>]
```

Default output directory is `./DATA`. Examples:
```bash
timetracker export                    # Export to ./DATA/
timetracker export -o ./backup       # Export to ./backup/
```

### Delete Projects

Delete a project and all its associated time entries.

```bash
timetracker delete project <project-slug>
```

Example:
```bash
timetracker delete project my-website
```

### Delete Time Entries

**⚠️ SAFETY FEATURES**: To prevent accidental data loss, deletion commands now have built-in safety checks.

#### Delete specific time entry by timestamp (RECOMMENDED)
```bash
# First, list times to see timestamps
timetracker times <project-slug>

# Delete specific entry by timestamp
timetracker delete times <project-slug> --timestamp <unix-timestamp>
```

#### Delete ALL time entries for a project (DANGEROUS!)
```bash
# This requires explicit confirmation with --all flag
# You'll be prompted to type "DELETE ALL" to proceed
timetracker delete times <project-slug> --all
```

Examples:
```bash
# Safe: Delete specific entry by timestamp
timetracker times my-website  # Shows: [ts:1234567890]
timetracker delete times my-website --timestamp 1234567890

# Dangerous: Delete all entries (requires confirmation)
timetracker delete times my-website --all
```

**💡 Pro Tips:**
- Use `timetracker times <project>` first to see timestamps in format `[ts:1234567890]`
- Consider using `timetracker export` to backup data before deletion
- Timestamp deletion is much safer than deleting all entries at once

## API Structure

The tool interacts with a key-value store REST API with the following structure:

### Keys

- `projects`: Array of project objects
- `projects/<project-slug>`: Array of time entry objects

### Project Object

```json
{
  "name": "Project Name",
  "slug": "project-slug",
  "description": "Project description"
}
```

### Time Entry Object

```json
{
  "timestamp": 1234567890,
  "type": "start", // or "end"
  "description": "Optional description"
}
```

## Examples

```bash
# Add a new project
timetracker project add website --name "Company Website" --description "Main company website"

# Start working
timetracker start website --description "Updating homepage"

# Check status
timetracker status website

# Stop working
timetracker end website --description "Homepage updates complete"

# View all time entries
timetracker times website

# View total time spent
timetracker total website

# List all projects
timetracker list

# Export all data for backup
timetracker export --output-dir ./backup

# Delete specific time entry (safe) - first check timestamps
timetracker times website  # Shows timestamps like [ts:1234567890]
timetracker delete times website --timestamp 1234567890

# Delete ALL time entries (dangerous - requires "DELETE ALL" confirmation)
timetracker delete times website --all

# Delete entire project and all its data
timetracker delete project website
```

## Error Handling

- If authentication fails, the tool will show a warning but continue to operate in local mode
- All errors are logged to the log file for debugging
- Network errors are handled gracefully with user-friendly error messages 

# Build the project
cargo build --release

# Add a project
./target/release/timetracker project add my-website --name "My Website" --description "Personal site"

# Start tracking
./target/release/timetracker start my-website --description "Working on homepage"

# Check status
./target/release/timetracker status my-website

# Stop tracking
./target/release/timetracker end my-website

# View total time
./target/release/timetracker total my-website 