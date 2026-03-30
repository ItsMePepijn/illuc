pub mod commands;

use crate::error::Result;
use crate::features::tasks::git::get_head_branch;
use crate::features::tasks::worktree::{format_title_from_branch, managed_worktree_root};
use crate::utils::path::normalize_path_string;
#[cfg(target_os = "windows")]
use crate::utils::windows::resolve_wsl_home_dir;
use anyhow::Context;
use chrono::{DateTime, Local};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tauri::Manager;

const RESPONSE_VERSION: u32 = 1;
const PRICING_FILE_NAME: &str = "openai-pricing.json";
const BUNDLED_PRICING: &str = include_str!("openai-pricing.json");
const COST_NOTE: &str =
    "Cost estimates use API pricing and exclude subscriptions, bundled allowances, and custom pricing.";

static SESSION_PARSE_CACHE: LazyLock<Mutex<HashMap<PathBuf, CachedSessionFile>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageBreakdown {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub input_cost: f64,
    pub cached_input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
}

impl UsageBreakdown {
    fn merge(&mut self, other: &UsageBreakdown) {
        self.input_tokens += other.input_tokens;
        self.cached_input_tokens += other.cached_input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
        self.input_cost += other.input_cost;
        self.cached_input_cost += other.cached_input_cost;
        self.output_cost += other.output_cost;
        self.total_cost += other.total_cost;
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayUsageBucket {
    pub date: String,
    pub session_count: u32,
    pub usage: UsageBreakdown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskUsageBucket {
    pub key: String,
    pub label: String,
    pub subtitle: String,
    pub path: String,
    pub is_workspace: bool,
    pub session_count: u32,
    pub last_active_at: String,
    pub usage: UsageBreakdown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageResponse {
    pub version: u32,
    pub currency: String,
    pub pricing_version: u32,
    pub pricing_source_url: String,
    pub pricing_published_at: String,
    pub note: String,
    pub scopes: TokenUsageScopes,
    pub by_task: Vec<TaskUsageBucket>,
    pub unknown_priced_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageScopes {
    pub global: ScopedTokenUsage,
    pub workspace: ScopedTokenUsage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopedTokenUsage {
    pub totals: UsageBreakdown,
    pub session_count: u32,
    pub by_day: Vec<DayUsageBucket>,
    pub by_month_session_counts: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PricingCatalog {
    version: u32,
    currency: String,
    source_url: String,
    published_at: String,
    models: HashMap<String, ModelPricing>,
    #[serde(default)]
    aliases: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelPricing {
    input_per_million: f64,
    cached_input_per_million: Option<f64>,
    output_per_million: f64,
}

impl PricingCatalog {
    fn resolve_pricing(&self, model: &str) -> Option<&ModelPricing> {
        if let Some(pricing) = self.models.get(model) {
            return Some(pricing);
        }
        self.aliases
            .get(model)
            .and_then(|canonical| self.models.get(canonical))
    }

    fn price_usage(
        &self,
        model: Option<&str>,
        usage: &UsageBreakdown,
    ) -> (UsageBreakdown, Option<String>) {
        let Some(model_name) = model.map(str::trim).filter(|value| !value.is_empty()) else {
            return (usage.clone(), Some("unknown".to_string()));
        };
        let Some(pricing) = self.resolve_pricing(model_name) else {
            return (usage.clone(), Some(model_name.to_string()));
        };
        let mut priced = usage.clone();
        priced.input_cost = (priced.input_tokens as f64 / 1_000_000.0) * pricing.input_per_million;
        priced.cached_input_cost = (priced.cached_input_tokens as f64 / 1_000_000.0)
            * pricing
                .cached_input_per_million
                .unwrap_or(pricing.input_per_million);
        priced.output_cost =
            (priced.output_tokens as f64 / 1_000_000.0) * pricing.output_per_million;
        priced.total_cost = priced.input_cost + priced.cached_input_cost + priced.output_cost;
        (priced, None)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SessionMetaLine {
    #[serde(rename = "type")]
    kind: String,
    payload: SessionMetaPayload,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionMetaPayload {
    cwd: String,
    #[serde(default)]
    originator: Option<String>,
    #[serde(default)]
    source: Option<SessionSource>,
}

impl SessionMetaPayload {
    fn is_supported_codex_source(&self) -> bool {
        let _ = &self.originator;
        let _ = &self.source;
        true
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum SessionSource {
    CliString(String),
    Subagent {
        #[serde(rename = "subagent")]
        _subagent: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct TurnContextLine {
    #[serde(rename = "type")]
    kind: String,
    payload: TurnContextPayload,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TurnContextPayload {
    model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenCountLine {
    timestamp: String,
    #[serde(rename = "type")]
    kind: String,
    payload: TokenCountPayload,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenCountPayload {
    #[serde(rename = "type")]
    kind: String,
    info: Option<TokenCountInfo>,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenCountInfo {
    total_token_usage: Option<TokenTotals>,
    last_token_usage: Option<TokenTotals>,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenTotals {
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Clone, Default)]
struct SessionAggregation {
    usage: UsageBreakdown,
    by_day: BTreeMap<String, UsageBreakdown>,
    day_sessions: HashSet<String>,
    unknown_priced_models: HashSet<String>,
}

#[derive(Debug, Clone, Default)]
struct TaskAggregate {
    label: String,
    subtitle: String,
    path: String,
    is_workspace: bool,
    session_count: u32,
    last_active_at: String,
    usage: UsageBreakdown,
}

#[derive(Debug, Clone)]
struct TaskMeta {
    key: String,
    label: String,
    subtitle: String,
    path: String,
    is_workspace: bool,
}

#[derive(Debug, Clone)]
struct CachedSessionFile {
    file_size: u64,
    modified_unix_ms: u128,
    pricing_version: u32,
    parsed: Option<ParsedSessionFile>,
}

#[derive(Debug, Clone)]
struct ParsedSessionFile {
    cwd: String,
    aggregation: SessionAggregation,
}

pub fn ensure_pricing_file(app: &tauri::AppHandle) -> Result<PathBuf> {
    let bundled: PricingCatalog = serde_json::from_str(BUNDLED_PRICING)
        .with_context(|| "failed to parse bundled pricing catalog")?;
    let config_dir = app
        .path()
        .app_config_dir()
        .with_context(|| "failed to resolve app config dir")?;
    std::fs::create_dir_all(&config_dir)
        .with_context(|| format!("failed to create {}", config_dir.display()))?;
    let pricing_path = config_dir.join(PRICING_FILE_NAME);

    let should_overwrite = match std::fs::read_to_string(&pricing_path) {
        Ok(contents) => match serde_json::from_str::<PricingCatalog>(&contents) {
            Ok(existing) => existing.version < bundled.version,
            Err(_) => true,
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => return Err(error.into()),
    };

    if should_overwrite {
        std::fs::write(&pricing_path, BUNDLED_PRICING)
            .with_context(|| format!("failed to write {}", pricing_path.display()))?;
    }

    Ok(pricing_path)
}

pub fn load_token_usage(app: &tauri::AppHandle, repo_root: &Path) -> Result<TokenUsageResponse> {
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    let repo_root_display = normalize_path_string(&repo_root);
    let worktree_root = managed_worktree_root(&repo_root)?;
    let worktree_root_display = normalize_path_string(&worktree_root);
    let pricing = load_pricing_catalog(app)?;
    let sessions_dir = resolve_codex_sessions_dir(app)?;

    let mut global_totals = UsageBreakdown::default();
    let mut workspace_totals = UsageBreakdown::default();
    let mut global_day_totals: BTreeMap<String, UsageBreakdown> = BTreeMap::new();
    let mut global_day_session_counts: HashMap<String, u32> = HashMap::new();
    let mut global_month_session_counts: HashMap<String, u32> = HashMap::new();
    let mut workspace_day_totals: BTreeMap<String, UsageBreakdown> = BTreeMap::new();
    let mut workspace_day_session_counts: HashMap<String, u32> = HashMap::new();
    let mut workspace_month_session_counts: HashMap<String, u32> = HashMap::new();
    let mut task_totals: HashMap<String, TaskAggregate> = HashMap::new();
    let mut unknown_priced_models = BTreeSet::new();
    let mut global_session_count = 0u32;
    let mut workspace_session_count = 0u32;

    if sessions_dir.exists() {
        let session_files = collect_session_files(&sessions_dir)?;
        for session_path in session_files {
            let parsed = get_or_parse_session_file(&session_path, &pricing);
            let Some(parsed) = (match parsed {
                Ok(value) => value,
                Err(error) => {
                    log::warn!(
                        "failed to parse codex session {}: {}",
                        session_path.display(),
                        error
                    );
                    None
                }
            }) else {
                continue;
            };
            let task_meta = classify_task_meta(
                &parsed.session.cwd,
                &repo_root_display,
                &worktree_root_display,
            );
            let session = parsed.session.aggregation;

            if session.usage.total_tokens == 0 {
                continue;
            }

            global_session_count += 1;
            global_totals.merge(&session.usage);
            for (date, usage) in &session.by_day {
                global_day_totals
                    .entry(date.clone())
                    .or_default()
                    .merge(usage);
            }
            for date in &session.day_sessions {
                *global_day_session_counts.entry(date.clone()).or_insert(0) += 1;
            }
            for month_key in session
                .day_sessions
                .iter()
                .map(|date| date[..7].to_string())
                .collect::<HashSet<_>>()
            {
                *global_month_session_counts.entry(month_key).or_insert(0) += 1;
            }

            if let Some(task_meta) = task_meta {
                workspace_session_count += 1;
                workspace_totals.merge(&session.usage);
                for (date, usage) in &session.by_day {
                    workspace_day_totals
                        .entry(date.clone())
                        .or_default()
                        .merge(usage);
                }
                for date in &session.day_sessions {
                    *workspace_day_session_counts
                        .entry(date.clone())
                        .or_insert(0) += 1;
                }
                for month_key in session
                    .day_sessions
                    .iter()
                    .map(|date| date[..7].to_string())
                    .collect::<HashSet<_>>()
                {
                    *workspace_month_session_counts.entry(month_key).or_insert(0) += 1;
                }
                let task =
                    task_totals
                        .entry(task_meta.key.clone())
                        .or_insert_with(|| TaskAggregate {
                            label: task_meta.label.clone(),
                            subtitle: task_meta.subtitle.clone(),
                            path: task_meta.path.clone(),
                            is_workspace: task_meta.is_workspace,
                            session_count: 0,
                            last_active_at: String::new(),
                            usage: UsageBreakdown::default(),
                        });
                task.session_count += 1;
                if let Some(last_active_at) = session.day_sessions.iter().max() {
                    if task.last_active_at.as_str() < last_active_at.as_str() {
                        task.last_active_at = last_active_at.clone();
                    }
                }
                task.usage.merge(&session.usage);
            }

            unknown_priced_models.extend(session.unknown_priced_models.into_iter());
        }
    }

    let mut by_day = global_day_totals
        .into_iter()
        .map(|(date, usage)| DayUsageBucket {
            session_count: global_day_session_counts.get(&date).copied().unwrap_or(0),
            date,
            usage,
        })
        .collect::<Vec<_>>();
    by_day.sort_by(|left, right| left.date.cmp(&right.date));

    let mut workspace_by_day = workspace_day_totals
        .into_iter()
        .map(|(date, usage)| DayUsageBucket {
            session_count: workspace_day_session_counts
                .get(&date)
                .copied()
                .unwrap_or(0),
            date,
            usage,
        })
        .collect::<Vec<_>>();
    workspace_by_day.sort_by(|left, right| left.date.cmp(&right.date));

    let mut by_task = task_totals
        .into_iter()
        .map(|(key, task)| TaskUsageBucket {
            key,
            label: task.label,
            subtitle: task.subtitle,
            path: task.path,
            is_workspace: task.is_workspace,
            session_count: task.session_count,
            last_active_at: task.last_active_at,
            usage: task.usage,
        })
        .collect::<Vec<_>>();
    by_task.sort_by(|left, right| {
        right
            .last_active_at
            .cmp(&left.last_active_at)
            .then_with(|| left.label.cmp(&right.label))
    });

    Ok(TokenUsageResponse {
        version: RESPONSE_VERSION,
        currency: pricing.currency.clone(),
        pricing_version: pricing.version,
        pricing_source_url: pricing.source_url.clone(),
        pricing_published_at: pricing.published_at.clone(),
        note: COST_NOTE.to_string(),
        scopes: TokenUsageScopes {
            global: ScopedTokenUsage {
                totals: global_totals,
                session_count: global_session_count,
                by_day,
                by_month_session_counts: global_month_session_counts.into_iter().collect(),
            },
            workspace: ScopedTokenUsage {
                totals: workspace_totals,
                session_count: workspace_session_count,
                by_day: workspace_by_day,
                by_month_session_counts: workspace_month_session_counts.into_iter().collect(),
            },
        },
        by_task,
        unknown_priced_models: unknown_priced_models.into_iter().collect(),
    })
}

fn load_pricing_catalog(app: &tauri::AppHandle) -> Result<PricingCatalog> {
    let pricing_path = ensure_pricing_file(app)?;
    let contents = std::fs::read_to_string(&pricing_path)
        .with_context(|| format!("failed to read {}", pricing_path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", pricing_path.display()))
        .map_err(Into::into)
}

fn resolve_codex_sessions_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    let home_dir = resolve_wsl_home_dir().with_context(|| "failed to resolve WSL home directory")?;

    #[cfg(not(target_os = "windows"))]
    let home_dir = app
        .path()
        .home_dir()
        .with_context(|| "failed to resolve user home directory")?;

    Ok(home_dir.join(".codex").join("sessions"))
}

fn collect_session_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_session_files_recursive(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_session_files_recursive(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_session_files_recursive(&path, files)?;
            continue;
        }
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("jsonl"))
        {
            files.push(path);
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct LoadedParsedSession {
    session: ParsedSessionFile,
}

fn get_or_parse_session_file(
    path: &Path,
    pricing: &PricingCatalog,
) -> Result<Option<LoadedParsedSession>> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("failed to read metadata for {}", path.display()))?;
    let file_size = metadata.len();
    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_millis())
        .unwrap_or_default();

    {
        let cache = SESSION_PARSE_CACHE.lock();
        if let Some(cached) = cache.get(path) {
            if cached.file_size == file_size
                && cached.modified_unix_ms == modified_unix_ms
                && cached.pricing_version == pricing.version
            {
                return Ok(cached
                    .parsed
                    .clone()
                    .map(|session| LoadedParsedSession { session }));
            }
        }
    }

    let parsed = parse_session_file(path, pricing)?;
    let mut cache = SESSION_PARSE_CACHE.lock();
    cache.insert(
        path.to_path_buf(),
        CachedSessionFile {
            file_size,
            modified_unix_ms,
            pricing_version: pricing.version,
            parsed: parsed.clone(),
        },
    );

    Ok(parsed.map(|session| LoadedParsedSession { session }))
}

fn parse_session_file(path: &Path, pricing: &PricingCatalog) -> Result<Option<ParsedSessionFile>> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    if reader.read_line(&mut first_line)? == 0 {
        return Ok(None);
    }
    let first_line = first_line.trim();
    if first_line.is_empty() {
        return Ok(None);
    }

    let session_meta: SessionMetaLine = match serde_json::from_str(first_line) {
        Ok(parsed) => parsed,
        Err(error) => {
            log::warn!(
                "failed to parse session meta for {}: {}",
                path.display(),
                error
            );
            return Ok(None);
        }
    };
    if session_meta.kind != "session_meta" {
        return Ok(None);
    }
    if !session_meta.payload.is_supported_codex_source() {
        return Ok(None);
    }

    let cwd = normalize_path_string(Path::new(&session_meta.payload.cwd));
    let mut aggregation = SessionAggregation::default();
    let mut active_model: Option<String> = None;
    let mut previous_totals: Option<TokenTotals> = None;

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let is_turn_context = trimmed.contains(r#""type":"turn_context""#);
        let is_token_count = trimmed.contains(r#""token_count""#);
        if !is_turn_context && !is_token_count {
            continue;
        }

        if is_turn_context {
            if let Ok(turn_context) = serde_json::from_str::<TurnContextLine>(trimmed) {
                if turn_context.kind == "turn_context" {
                    if let Some(model) = turn_context.payload.model {
                        active_model = Some(model);
                    }
                    continue;
                }
            }
        }

        let token_line: TokenCountLine = match serde_json::from_str(trimmed) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        if token_line.kind != "event_msg" || token_line.payload.kind != "token_count" {
            continue;
        }
        let Some(info) = token_line.payload.info else {
            continue;
        };
        let delta = if let Some(current_totals) = info.total_token_usage {
            let delta = token_delta(previous_totals.as_ref(), &current_totals);
            previous_totals = Some(current_totals);
            delta
        } else if let Some(last_totals) = info.last_token_usage {
            token_delta(None, &last_totals)
        } else {
            continue;
        };
        if delta.total_tokens == 0 {
            continue;
        }

        let (priced_delta, unknown_model) = pricing.price_usage(active_model.as_deref(), &delta);
        if let Some(model) = unknown_model {
            aggregation.unknown_priced_models.insert(model);
        }

        let date = match local_date_key(&token_line.timestamp) {
            Ok(value) => value,
            Err(error) => {
                log::warn!(
                    "failed to parse token-count timestamp in {}: {}",
                    path.display(),
                    error
                );
                continue;
            }
        };
        aggregation.day_sessions.insert(date.clone());
        aggregation.usage.merge(&priced_delta);
        aggregation
            .by_day
            .entry(date)
            .or_default()
            .merge(&priced_delta);
    }

    if aggregation.usage.total_tokens == 0 {
        return Ok(None);
    }

    Ok(Some(ParsedSessionFile { cwd, aggregation }))
}

fn classify_task_meta(cwd: &str, repo_root: &str, worktree_root: &str) -> Option<TaskMeta> {
    let is_repo_session = cwd == repo_root;
    let worktree_prefix = format!("{}{}", worktree_root, std::path::MAIN_SEPARATOR);
    let is_worktree_session = cwd == worktree_root || cwd.starts_with(&worktree_prefix);
    if !is_repo_session && !is_worktree_session {
        return None;
    }

    Some(resolve_task_meta(cwd, repo_root))
}

fn resolve_task_meta(cwd: &str, repo_root: &str) -> TaskMeta {
    if cwd == repo_root {
        let repo_name = Path::new(repo_root)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Workspace");
        return TaskMeta {
            key: "workspace".to_string(),
            label: "Workspace".to_string(),
            subtitle: repo_name.to_string(),
            path: cwd.to_string(),
            is_workspace: true,
        };
    }

    let path = PathBuf::from(cwd);
    if path.exists() {
        if let Ok(branch_name) = get_head_branch(&path) {
            return TaskMeta {
                key: cwd.to_string(),
                label: format_title_from_branch(&branch_name),
                subtitle: branch_name,
                path: cwd.to_string(),
                is_workspace: false,
            };
        }
    }

    let fallback = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("task")
        .to_string();
    let short = fallback.chars().take(8).collect::<String>();
    TaskMeta {
        key: cwd.to_string(),
        label: format!("Task {}", short),
        subtitle: "Discarded worktree".to_string(),
        path: cwd.to_string(),
        is_workspace: false,
    }
}

fn local_date_key(timestamp: &str) -> Result<String> {
    let instant = DateTime::parse_from_rfc3339(timestamp)
        .with_context(|| format!("failed to parse session timestamp {timestamp}"))?;
    Ok(instant.with_timezone(&Local).format("%Y-%m-%d").to_string())
}

fn token_delta(previous: Option<&TokenTotals>, current: &TokenTotals) -> UsageBreakdown {
    let previous_input = previous
        .map(|value| value.input_tokens.saturating_sub(value.cached_input_tokens))
        .unwrap_or_default();
    let previous_cached = previous
        .map(|value| value.cached_input_tokens)
        .unwrap_or_default();
    let previous_output = previous
        .map(|value| value.output_tokens)
        .unwrap_or_default();
    let current_input = current
        .input_tokens
        .saturating_sub(current.cached_input_tokens);

    let input_tokens = current_input.saturating_sub(previous_input);
    let cached_input_tokens = current.cached_input_tokens.saturating_sub(previous_cached);
    let output_tokens = current.output_tokens.saturating_sub(previous_output);
    let total_tokens = input_tokens + cached_input_tokens + output_tokens;

    UsageBreakdown {
        input_tokens,
        cached_input_tokens,
        output_tokens,
        total_tokens,
        input_cost: 0.0,
        cached_input_cost: 0.0,
        output_cost: 0.0,
        total_cost: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        token_delta, PricingCatalog, SessionMetaPayload, SessionSource, TokenCountLine,
        UsageBreakdown,
    };
    use std::collections::HashMap;

    #[derive(Debug, Clone)]
    struct Totals {
        input_tokens: u64,
        cached_input_tokens: u64,
        output_tokens: u64,
    }

    impl From<Totals> for super::TokenTotals {
        fn from(value: Totals) -> Self {
            Self {
                input_tokens: value.input_tokens,
                cached_input_tokens: value.cached_input_tokens,
                output_tokens: value.output_tokens,
            }
        }
    }

    #[test]
    fn token_delta_splits_cached_tokens_from_input_tokens() {
        let previous = Totals {
            input_tokens: 1_000,
            cached_input_tokens: 400,
            output_tokens: 200,
        };
        let current = Totals {
            input_tokens: 1_800,
            cached_input_tokens: 900,
            output_tokens: 350,
        };

        let previous_totals: super::TokenTotals = previous.into();
        let current_totals: super::TokenTotals = current.into();
        let delta = token_delta(Some(&previous_totals), &current_totals);
        assert_eq!(delta.input_tokens, 300);
        assert_eq!(delta.cached_input_tokens, 500);
        assert_eq!(delta.output_tokens, 150);
        assert_eq!(delta.total_tokens, 950);
    }

    #[test]
    fn pricing_aliases_resolve_to_canonical_models() {
        let catalog = PricingCatalog {
            version: 1,
            currency: "USD".to_string(),
            source_url: "https://example.com".to_string(),
            published_at: "2026-03-30".to_string(),
            models: HashMap::from([(
                "gpt-5.1-codex-mini".to_string(),
                super::ModelPricing {
                    input_per_million: 0.25,
                    cached_input_per_million: Some(0.025),
                    output_per_million: 2.0,
                },
            )]),
            aliases: HashMap::from([(
                "gpt-5-codex-mini".to_string(),
                "gpt-5.1-codex-mini".to_string(),
            )]),
        };

        let usage = UsageBreakdown {
            input_tokens: 100_000,
            cached_input_tokens: 50_000,
            output_tokens: 25_000,
            total_tokens: 175_000,
            input_cost: 0.0,
            cached_input_cost: 0.0,
            output_cost: 0.0,
            total_cost: 0.0,
        };
        let (priced, unknown) = catalog.price_usage(Some("gpt-5-codex-mini"), &usage);
        assert!(unknown.is_none());
        assert!((priced.input_cost - 0.025).abs() < 0.000_001);
        assert!((priced.cached_input_cost - 0.00125).abs() < 0.000_001);
        assert!((priced.output_cost - 0.05).abs() < 0.000_001);
        assert!((priced.total_cost - 0.07625).abs() < 0.000_001);
    }

    #[test]
    fn codex_source_filter_accepts_cli_gui_and_subagents() {
        let cli_payload = SessionMetaPayload {
            cwd: "/tmp/project".to_string(),
            originator: Some("codex_cli_rs".to_string()),
            source: Some(SessionSource::CliString("cli".to_string())),
        };
        assert!(cli_payload.is_supported_codex_source());

        let gui_payload = SessionMetaPayload {
            cwd: "/tmp/project".to_string(),
            originator: Some("illuc-codex-gui".to_string()),
            source: Some(SessionSource::CliString("vscode".to_string())),
        };
        assert!(gui_payload.is_supported_codex_source());

        let subagent_payload = SessionMetaPayload {
            cwd: "/tmp/project".to_string(),
            originator: Some("codex_cli_rs".to_string()),
            source: Some(SessionSource::Subagent {
                _subagent: "review".to_string(),
            }),
        };
        assert!(subagent_payload.is_supported_codex_source());

        let legacy_cli_payload = SessionMetaPayload {
            cwd: "/tmp/project".to_string(),
            originator: Some("codex_cli_rs".to_string()),
            source: None,
        };
        assert!(legacy_cli_payload.is_supported_codex_source());

        let other_payload = SessionMetaPayload {
            cwd: "/tmp/project".to_string(),
            originator: Some("something_else".to_string()),
            source: Some(SessionSource::CliString("editor".to_string())),
        };
        assert!(other_payload.is_supported_codex_source());
    }

    #[test]
    fn token_count_line_deserializes_snake_case_totals() {
        let json = r#"{
            "timestamp":"2026-03-21T10:43:25.070Z",
            "type":"event_msg",
            "payload":{
                "type":"token_count",
                "info":{
                    "total_token_usage":{
                        "input_tokens":9226,
                        "cached_input_tokens":7552,
                        "output_tokens":141
                    }
                }
            }
        }"#;

        let parsed: TokenCountLine = serde_json::from_str(json).expect("token_count should parse");
        let totals = parsed
            .payload
            .info
            .and_then(|info| info.total_token_usage)
            .expect("totals should exist");

        assert_eq!(totals.input_tokens, 9_226);
        assert_eq!(totals.cached_input_tokens, 7_552);
        assert_eq!(totals.output_tokens, 141);
    }

    #[test]
    fn token_count_line_deserializes_last_token_usage_fallback() {
        let json = r#"{
            "timestamp":"2026-03-21T10:43:25.070Z",
            "type":"event_msg",
            "payload":{
                "type":"token_count",
                "info":{
                    "last_token_usage":{
                        "input_tokens":9226,
                        "cached_input_tokens":7552,
                        "output_tokens":141
                    }
                }
            }
        }"#;

        let parsed: TokenCountLine = serde_json::from_str(json).expect("token_count should parse");
        let totals = parsed
            .payload
            .info
            .and_then(|info| info.last_token_usage)
            .expect("last token usage should exist");

        assert_eq!(totals.input_tokens, 9_226);
        assert_eq!(totals.cached_input_tokens, 7_552);
        assert_eq!(totals.output_tokens, 141);
    }

    #[test]
    fn token_delta_uses_last_totals_as_direct_usage_when_no_cumulative_totals_exist() {
        let last_totals = super::TokenTotals {
            input_tokens: 9_226,
            cached_input_tokens: 7_552,
            output_tokens: 141,
        };

        let delta = token_delta(None, &last_totals);
        assert_eq!(delta.input_tokens, 1_674);
        assert_eq!(delta.cached_input_tokens, 7_552);
        assert_eq!(delta.output_tokens, 141);
        assert_eq!(delta.total_tokens, 9_367);
    }
}
