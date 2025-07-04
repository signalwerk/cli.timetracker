use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::env;
use std::fs;
use chrono::{DateTime, Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenCache {
    token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyValueRequest {
    key: String,
    value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyValueResponse {
    data: KeyValueData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyValueData {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyValueListResponse {
    data: Vec<KeyValueData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRequest {
    value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub name: String,
    pub slug: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeEntry {
    pub timestamp: i64,
    #[serde(rename = "type")]
    pub entry_type: String, // "start" or "end"
    pub description: Option<String>,
}

pub struct ApiClient {
    client: Client,
    token: Option<String>,
    login_url: String,
    data_base_url: String,
    username: String,
    password: String,
    token_cache_file: String,
}

impl ApiClient {
    pub fn new() -> Result<Self> {
        // Load environment variables from .env file
        dotenv::dotenv().ok(); // Don't fail if .env doesn't exist
        
        let api_domain = env::var("API_DOMAIN")
            .unwrap_or_else(|_| "https://kv.srv.signalwerk.ch".to_string());
        let api_project = env::var("API_PROJECT")
            .unwrap_or_else(|_| "timetracker".to_string());
        let username = env::var("API_USERNAME")
            .map_err(|_| anyhow!("API_USERNAME not found in environment"))?;
        let password = env::var("API_PASSWORD")
            .map_err(|_| anyhow!("API_PASSWORD not found in environment"))?;
        let token_cache_file = env::var("TOKEN_CACHE_FILE")
            .unwrap_or_else(|_| ".token_cache.json".to_string());

        let login_url = format!("{}/login", api_domain);
        let data_base_url = format!("{}/{}", api_domain, api_project);

        Ok(Self {
            client: Client::new(),
            token: None,
            login_url,
            data_base_url,
            username,
            password,
            token_cache_file,
        })
    }

    fn load_cached_token(&self) -> Option<String> {
        if let Ok(content) = fs::read_to_string(&self.token_cache_file) {
            if let Ok(cache) = serde_json::from_str::<TokenCache>(&content) {
                // Check if token is still valid (not expired)
                if cache.expires_at > Utc::now() {
                    return Some(cache.token);
                }
            }
        }
        None
    }

    fn save_token_to_cache(&self, token: &str) -> Result<()> {
        // Set token to expire in 23 hours (assuming 24h validity, with 1h buffer)
        let expires_at = Utc::now() + Duration::hours(23);
        let cache = TokenCache {
            token: token.to_string(),
            expires_at,
        };
        
        let content = serde_json::to_string_pretty(&cache)?;
        fs::write(&self.token_cache_file, content)?;
        Ok(())
    }

    async fn is_token_valid(&self, token: &str) -> bool {
        // Test the token by making a simple API call
        let response = self
            .client
            .get(&format!("{}/data", self.data_base_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;
        
        match response {
            Ok(resp) => resp.status().is_success() || resp.status().as_u16() == 404, // 404 is also valid (empty data)
            Err(_) => false,
        }
    }

    pub async fn authenticate(&mut self) -> Result<()> {
        // First, try to load cached token
        if let Some(cached_token) = self.load_cached_token() {
            // Verify the cached token is still valid
            if self.is_token_valid(&cached_token).await {
                self.token = Some(cached_token);
                return Ok(());
            }
        }

        // If no valid cached token, perform fresh authentication
        let login_request = LoginRequest {
            username: self.username.clone(),
            password: self.password.clone(),
        };

        let response = self
            .client
            .post(&self.login_url)
            .json(&login_request)
            .send()
            .await?;

        if response.status().is_success() {
            let login_response: LoginResponse = response.json().await?;
            
            // Save token to cache
            self.save_token_to_cache(&login_response.token)?;
            
            self.token = Some(login_response.token);
            Ok(())
        } else {
            Err(anyhow!("Authentication failed: {}", response.status()))
        }
    }

    async fn get_auth_header(&self) -> Result<String> {
        match &self.token {
            Some(token) => Ok(format!("Bearer {}", token)),
            None => Err(anyhow!("Not authenticated")),
        }
    }

    pub async fn get_key(&self, key: &str) -> Result<serde_json::Value> {
        let auth_header = self.get_auth_header().await?;
        
        let encoded_key = urlencoding::encode(key);
        let response = self
            .client
            .get(&format!("{}/data/{}", self.data_base_url, encoded_key))
            .header("Authorization", auth_header)
            .send()
            .await?;

        if response.status().is_success() {
            let kv_response: KeyValueResponse = response.json().await?;
            
            // The API returns values as JSON strings, so we need to parse them
            match &kv_response.data.value {
                serde_json::Value::String(s) => {
                    // Try to parse the string as JSON
                    match serde_json::from_str(s) {
                        Ok(parsed) => Ok(parsed),
                        Err(_) => Ok(kv_response.data.value) // Return as-is if not valid JSON
                    }
                }
                _ => Ok(kv_response.data.value)
            }
        } else if response.status().as_u16() == 404 {
            // Key doesn't exist, return empty array for lists
            Ok(serde_json::json!([]))
        } else {
            Err(anyhow!("Failed to get key: {}", response.status()))
        }
    }

    pub async fn set_key(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let auth_header = self.get_auth_header().await?;
        
        // Serialize the value to a JSON string since the API expects string values
        let value_string = serde_json::to_string(&value)?;
        let request = KeyValueRequest {
            key: key.to_string(),
            value: serde_json::Value::String(value_string),
        };

        let response = self
            .client
            .post(&format!("{}/data", self.data_base_url))
            .header("Authorization", auth_header)
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to set key: {}", response.status()))
        }
    }

    pub async fn update_key(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let auth_header = self.get_auth_header().await?;
        
        // Serialize the value to a JSON string since the API expects string values
        let value_string = serde_json::to_string(&value)?;
        let request = UpdateRequest { 
            value: serde_json::Value::String(value_string) 
        };

        let encoded_key = urlencoding::encode(key);
        let response = self
            .client
            .put(&format!("{}/data/{}", self.data_base_url, encoded_key))
            .header("Authorization", auth_header)
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to update key: {}", response.status()))
        }
    }

    pub async fn get_projects(&self) -> Result<Vec<Project>> {
        let projects_value = self.get_key("projects").await?;
        let projects: Vec<Project> = serde_json::from_value(projects_value)?;
        Ok(projects)
    }

    pub async fn get_project(&self, slug: &str) -> Result<Project> {
        let projects = self.get_projects().await?;
        projects
            .into_iter()
            .find(|p| p.slug == slug)
            .ok_or_else(|| anyhow!("Project with slug '{}' not found", slug))
    }

    pub async fn add_project(&self, project: Project) -> Result<()> {
        let mut projects = self.get_projects().await.unwrap_or_default();
        
        // Check if project already exists
        if projects.iter().any(|p| p.slug == project.slug) {
            return Err(anyhow!("Project with slug '{}' already exists", project.slug));
        }
        
        projects.push(project);
        let is_first_project = projects.len() == 1;
        let value = serde_json::to_value(projects)?;
        
        // Use set_key for first time, or update_key if projects already exist
        if is_first_project {
            self.set_key("projects", value).await
        } else {
            self.update_key("projects", value).await
        }
    }

    pub async fn update_project(&self, old_slug: &str, updated_project: Project) -> Result<()> {
        let mut projects = self.get_projects().await.unwrap_or_default();
        
        // Find the project to update
        let project_index = projects.iter().position(|p| p.slug == old_slug)
            .ok_or_else(|| anyhow!("Project with slug '{}' not found", old_slug))?;
        
        // If slug is changing, check if new slug already exists (but ignore the current project)
        if old_slug != updated_project.slug {
            if projects.iter().enumerate().any(|(i, p)| i != project_index && p.slug == updated_project.slug) {
                return Err(anyhow!("Project with slug '{}' already exists", updated_project.slug));
            }
            
            // If slug is changing, we need to move the time entries to the new key
            let old_time_key = format!("projects/{}", old_slug);
            let new_time_key = format!("projects/{}", updated_project.slug);
            
            // Get existing time entries for the old slug
            if let Ok(time_entries) = self.get_time_entries(old_slug).await {
                if !time_entries.is_empty() {
                    // Save time entries under new slug
                    let value = serde_json::to_value(time_entries)?;
                    self.set_key(&new_time_key, value).await?;
                    
                    // Delete old time entries
                    if let Err(e) = self.delete_key(&old_time_key).await {
                        // Only fail if it's not a 404 (key doesn't exist)
                        if !e.to_string().contains("404") {
                            return Err(anyhow!("Failed to delete old time entries: {}", e));
                        }
                    }
                }
            }
        }
        
        // Update the project in the projects list
        projects[project_index] = updated_project;
        let value = serde_json::to_value(projects)?;
        self.update_key("projects", value).await
    }

    pub async fn get_time_entries(&self, project_slug: &str) -> Result<Vec<TimeEntry>> {
        let key = format!("projects/{}", project_slug);
        let value = self.get_key(&key).await?;
        let entries: Vec<TimeEntry> = serde_json::from_value(value)?;
        Ok(entries)
    }

    pub async fn add_time_entry(&self, project_slug: &str, entry: TimeEntry) -> Result<()> {
        let key = format!("projects/{}", project_slug);
        let mut entries = self.get_time_entries(project_slug).await.unwrap_or_default();
        entries.push(entry);
        let is_first_entry = entries.len() == 1;
        let value = serde_json::to_value(entries)?;
        
        // Use set_key for first time, or update_key if entries already exist
        if is_first_entry {
            self.set_key(&key, value).await
        } else {
            self.update_key(&key, value).await
        }
    }

    pub async fn get_all_keys(&self) -> Result<Vec<KeyValueData>> {
        let auth_header = self.get_auth_header().await?;
        
        let response = self
            .client
            .get(&format!("{}/data", self.data_base_url))
            .header("Authorization", auth_header)
            .send()
            .await?;

        if response.status().is_success() {
            let list_response: KeyValueListResponse = response.json().await?;
            Ok(list_response.data)
        } else {
            Err(anyhow!("Failed to get all keys: {}", response.status()))
        }
    }

    pub async fn delete_key(&self, key: &str) -> Result<()> {
        let auth_header = self.get_auth_header().await?;
        let encoded_key = urlencoding::encode(key);
        
        let response = self
            .client
            .delete(&format!("{}/data/{}", self.data_base_url, encoded_key))
            .header("Authorization", auth_header)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to delete key: {}", response.status()))
        }
    }

    pub async fn delete_project(&self, project_slug: &str) -> Result<()> {
        let mut projects = self.get_projects().await.unwrap_or_default();
        
        // Find and remove the project
        let original_len = projects.len();
        projects.retain(|p| p.slug != project_slug);
        
        if projects.len() == original_len {
            return Err(anyhow!("Project with slug '{}' not found", project_slug));
        }
        
        // First, delete the time entries for this project
        let time_key = format!("projects/{}", project_slug);
        if let Err(e) = self.delete_key(&time_key).await {
            // Only fail if it's not a 404 (key doesn't exist)
            if !e.to_string().contains("404") {
                return Err(anyhow!("Failed to delete time entries for project '{}': {}", project_slug, e));
            }
            // If 404, it just means no time entries exist, which is fine
        }
        
        // Then update the projects list
        let value = serde_json::to_value(projects)?;
        self.update_key("projects", value).await?;
        
        Ok(())
    }

    pub async fn delete_project_times(&self, project_slug: &str) -> Result<()> {
        let key = format!("projects/{}", project_slug);
        self.delete_key(&key).await
    }

    pub async fn delete_time_entry_by_timestamp(&self, project_slug: &str, timestamp: i64) -> Result<()> {
        let key = format!("projects/{}", project_slug);
        let mut entries = self.get_time_entries(project_slug).await.unwrap_or_default();
        
        // Find and remove the entry with the specified timestamp
        let original_len = entries.len();
        entries.retain(|entry| entry.timestamp != timestamp);
        
        if entries.len() == original_len {
            return Err(anyhow!("Time entry with timestamp {} not found for project '{}'", timestamp, project_slug));
        }
        
        // Update the entries list
        let value = serde_json::to_value(entries)?;
        self.update_key(&key, value).await
    }

    pub async fn update_time_entry_by_timestamp(&self, project_slug: &str, timestamp: i64, new_description: Option<String>) -> Result<()> {
        let key = format!("projects/{}", project_slug);
        let mut entries = self.get_time_entries(project_slug).await.unwrap_or_default();
        
        // Find the entry with the specified timestamp and update its description
        let mut found = false;
        for entry in &mut entries {
            if entry.timestamp == timestamp {
                entry.description = new_description.clone();
                found = true;
                break;
            }
        }
        
        if !found {
            return Err(anyhow!("Time entry with timestamp {} not found for project '{}'", timestamp, project_slug));
        }
        
        // Update the entries list
        let value = serde_json::to_value(entries)?;
        self.update_key(&key, value).await
    }
} 