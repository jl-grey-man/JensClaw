use async_trait::async_trait;
use chrono::{Datelike, TimeZone};
use serde_json::json;

use crate::claude::ToolDefinition;

use super::{schema_object, Tool, ToolResult};

/// Parse natural language date/time and convert to ISO 8601 format
/// Examples: "tomorrow at 13:30", "in 5 minutes", "next Monday at 9am"
pub struct ParseDateTimeTool;

impl ParseDateTimeTool {
    pub fn new() -> Self {
        ParseDateTimeTool
    }

    fn parse_natural_datetime(&self, input: &str, tz_name: &str) -> Result<String, String> {
        let tz: chrono_tz::Tz = tz_name
            .parse()
            .map_err(|_| format!("Invalid timezone: {}", tz_name))?;
        
        let now = chrono::Local::now().with_timezone(&tz);
        let input_lower = input.to_lowercase();
        
        // Helper to convert to UTC RFC 3339 for consistent storage/comparison
        let to_utc_rfc3339 = |dt: chrono::DateTime<chrono_tz::Tz>| -> String {
            dt.with_timezone(&chrono::Utc).to_rfc3339()
        };
        
        // Handle "tomorrow at HH:MM" or "tomorrow at H:MM"
        if input_lower.contains("tomorrow") {
            let tomorrow = now + chrono::Duration::days(1);
            let time_part = self.extract_time(&input_lower);
            
            if let Some((hour, minute)) = time_part {
                let scheduled = tz
                    .with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), hour, minute, 0)
                    .single()
                    .ok_or("Invalid date/time")?;
                return Ok(to_utc_rfc3339(scheduled));
            } else {
                // Default to 9:00 AM if no time specified
                let scheduled = tz
                    .with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), 9, 0, 0)
                    .single()
                    .ok_or("Invalid date/time")?;
                return Ok(to_utc_rfc3339(scheduled));
            }
        }
        
        // Handle "in X minutes/hours/days"
        if input_lower.starts_with("in ") {
            let duration = self.parse_duration(&input_lower[3..])?;
            let scheduled = now + duration;
            return Ok(to_utc_rfc3339(scheduled));
        }
        
        // Handle "today at HH:MM"
        if input_lower.contains("today") {
            let time_part = self.extract_time(&input_lower);
            if let Some((hour, minute)) = time_part {
                let scheduled = tz
                    .with_ymd_and_hms(now.year(), now.month(), now.day(), hour, minute, 0)
                    .single()
                    .ok_or("Invalid date/time")?;
                return Ok(to_utc_rfc3339(scheduled));
            }
        }
        
        // Handle day names like "Monday", "next Monday"
        let days = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
        for (i, day) in days.iter().enumerate() {
            if input_lower.contains(day) {
                let target_weekday = i as i64 + 1; // chrono uses 1-7 for Mon-Sun
                let current_weekday = now.weekday().number_from_monday() as i64;
                let days_until = (target_weekday - current_weekday + 7) % 7;
                let days_until = if days_until == 0 { 7 } else { days_until }; // Next week if today
                
                let target_date = now + chrono::Duration::days(days_until);
                let time_part = self.extract_time(&input_lower);
                
                if let Some((hour, minute)) = time_part {
                    let scheduled = tz
                        .with_ymd_and_hms(target_date.year(), target_date.month(), target_date.day(), hour, minute, 0)
                        .single()
                        .ok_or("Invalid date/time")?;
                    return Ok(to_utc_rfc3339(scheduled));
                } else {
                    let scheduled = tz
                        .with_ymd_and_hms(target_date.year(), target_date.month(), target_date.day(), 9, 0, 0)
                        .single()
                        .ok_or("Invalid date/time")?;
                    return Ok(to_utc_rfc3339(scheduled));
                }
            }
        }
        
        Err(format!("Could not parse date/time: {}", input))
    }

    fn extract_time(&self, input: &str) -> Option<(u32, u32)> {
        // Try to find patterns like "13:30", "13.30", "1:30pm", "1pm"
        use regex::Regex;
        
        // Pattern for 24h format: "13:30" or "13.30"
        let re_24h = Regex::new(r"(\d{1,2})[:\.](\d{2})").ok()?;
        if let Some(caps) = re_24h.captures(input) {
            let hour: u32 = caps[1].parse().ok()?;
            let minute: u32 = caps[2].parse().ok()?;
            if hour < 24 && minute < 60 {
                return Some((hour, minute));
            }
        }
        
        // Pattern for 12h format with am/pm: "1:30pm" or "1pm"
        let re_12h = Regex::new(r"(\d{1,2})(?::(\d{2}))?\s*(am|pm)").ok()?;
        if let Some(caps) = re_12h.captures(input) {
            let mut hour: u32 = caps[1].parse().ok()?;
            let minute: u32 = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let period = caps[3].to_lowercase();
            
            if period == "pm" && hour != 12 {
                hour += 12;
            } else if period == "am" && hour == 12 {
                hour = 0;
            }
            
            if hour < 24 && minute < 60 {
                return Some((hour, minute));
            }
        }
        
        None
    }

    fn parse_duration(&self, input: &str) -> Result<chrono::Duration, String> {
        use regex::Regex;
        
        let re = Regex::new(r"^(\d+)\s*(minute|minutes|min|hour|hours|hr|day|days)\s*$").unwrap();
        
        if let Some(caps) = re.captures(input.trim()) {
            let amount: i64 = caps[1].parse().map_err(|_| "Invalid number")?;
            let unit = &caps[2];
            
            match unit {
                "minute" | "minutes" | "min" => Ok(chrono::Duration::minutes(amount)),
                "hour" | "hours" | "hr" => Ok(chrono::Duration::hours(amount)),
                "day" | "days" => Ok(chrono::Duration::days(amount)),
                _ => Err("Unknown time unit".to_string()),
            }
        } else {
            Err(format!("Could not parse duration: {}", input))
        }
    }
}

#[async_trait]
impl Tool for ParseDateTimeTool {
    fn name(&self) -> &str {
        "parse_datetime"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "parse_datetime".into(),
            description: "Convert natural language date/time to ISO 8601 format. Use this when scheduling tasks to ensure correct timestamp format. Supports: 'tomorrow at 13:30', 'in 5 minutes', 'in 2 hours', 'next Monday at 9am', 'today at 3pm'".into(),
            input_schema: schema_object(
                json!({
                    "input": {
                        "type": "string",
                        "description": "Natural language date/time description"
                    },
                    "timezone": {
                        "type": "string",
                        "description": "IANA timezone name (default: Europe/Stockholm)"
                    }
                }),
                &["input"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let text = match input.get("input").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return ToolResult::error("Missing 'input' parameter".into()),
        };

        let tz = input
            .get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("Europe/Stockholm");

        match self.parse_natural_datetime(text, tz) {
            Ok(iso_timestamp) => ToolResult::success(format!(
                "Parsed '{}' to ISO 8601: {}",
                text, iso_timestamp
            )),
            Err(e) => ToolResult::error(format!("Failed to parse datetime: {}", e)),
        }
    }
}
