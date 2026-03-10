use crate::commands::CommandResult;
use crate::features::settings::load_working_hours_expression;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_common::with_running_gui_usage_agent_mut;
use crate::features::tasks::TaskManager;
use chrono::{Duration, Local, TimeZone};
use opening_hours::OpeningHours;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub rate_limits: Option<Value>,
    pub working_periods: Vec<WorkingPeriod>,
    pub window_duration_hours: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkingPeriod {
    pub start_at: String,
    pub end_at: String,
}

#[tauri::command]
pub async fn task_agent_gui_usage(
    app: tauri::AppHandle,
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    with_running_gui_usage_agent_mut(&manager, req.task_id, |gui_agent| {
        let rate_limits = gui_agent
            .refresh_rate_limits()
            .map_err(|error| error.to_string())?;

        let usage_window = rate_limits.as_ref().and_then(select_usage_window);
        let window_duration_hours = usage_window
            .as_ref()
            .map(|window| window.window_duration_mins as f64 / 60.0);
        let working_periods = usage_window
            .as_ref()
            .map(|window| build_working_periods(&app, window))
            .transpose()
            .map_err(|error| error.to_string())?
            .unwrap_or_default();

        Ok(Response {
            rate_limits,
            working_periods,
            window_duration_hours,
        })
    })
}

#[derive(Debug, Clone, Copy)]
struct UsageWindow {
    window_duration_mins: i64,
    resets_at_unix_s: i64,
}

fn select_usage_window(rate_limits: &Value) -> Option<UsageWindow> {
    let root = rate_limits.as_object()?;
    let envelope = root
        .get("rateLimits")
        .and_then(Value::as_object)
        .unwrap_or(root);
    let primary = envelope.get("primary").and_then(Value::as_object);
    let secondary = envelope.get("secondary").and_then(Value::as_object);

    let primary_window = primary.and_then(parse_usage_window);
    let secondary_window = secondary.and_then(parse_usage_window);
    match (primary_window, secondary_window) {
        (Some(left), Some(right)) => {
            if right.window_duration_mins >= left.window_duration_mins {
                Some(right)
            } else {
                Some(left)
            }
        }
        (Some(window), None) | (None, Some(window)) => Some(window),
        (None, None) => None,
    }
}

fn parse_usage_window(window: &serde_json::Map<String, Value>) -> Option<UsageWindow> {
    let window_duration_mins = window.get("windowDurationMins")?.as_i64()?;
    let resets_at_unix_s = window.get("resetsAt")?.as_i64()?;
    if window_duration_mins <= 0 {
        return None;
    }
    Some(UsageWindow {
        window_duration_mins,
        resets_at_unix_s,
    })
}

fn build_working_periods(
    app: &tauri::AppHandle,
    usage_window: &UsageWindow,
) -> anyhow::Result<Vec<WorkingPeriod>> {
    let configured_expression = load_working_hours_expression(app)?;
    let opening_hours = parse_working_hours_with_fallback(&configured_expression);

    let end = Local
        .timestamp_opt(usage_window.resets_at_unix_s, 0)
        .single()
        .ok_or_else(|| anyhow::anyhow!("invalid reset timestamp"))?;
    let start = end - Duration::minutes(usage_window.window_duration_mins);

    let mut periods = Vec::new();
    let mut cursor = start.naive_local();
    let end_naive = end.naive_local();
    while cursor < end_naive {
        let next_change = opening_hours.next_change(cursor).unwrap_or(end_naive);
        let segment_end = next_change.min(end_naive);
        if segment_end <= cursor {
            break;
        }
        if opening_hours.is_open(cursor) {
            periods.push(WorkingPeriod {
                start_at: format_local_datetime(cursor)?,
                end_at: format_local_datetime(segment_end)?,
            });
        }
        cursor = segment_end;
    }

    Ok(periods)
}

fn parse_working_hours_with_fallback(expression: &str) -> OpeningHours {
    match OpeningHours::parse(expression) {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "invalid working_hours setting {:?}; falling back to default schedule: {}",
                expression,
                error
            );
            OpeningHours::parse("Mo-Fr 09:00-17:00")
                .expect("default working hours must be a valid opening_hours expression")
        }
    }
}

fn format_local_datetime(value: chrono::NaiveDateTime) -> anyhow::Result<String> {
    let local = Local
        .from_local_datetime(&value)
        .earliest()
        .or_else(|| Local.from_local_datetime(&value).latest())
        .ok_or_else(|| anyhow::anyhow!("failed to resolve local datetime"))?;
    Ok(local.to_rfc3339())
}
