pub mod data {
    use serde::Deserialize;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    #[derive(Deserialize, Debug, Clone)]
    pub struct ClaudeHistoryRow {
        #[serde(default)]
        pub display: String,
        #[serde(default)]
        pub timestamp: u64,
        #[serde(default)]
        pub project: String,
    }

    pub fn load_history() -> Vec<ClaudeHistoryRow> {
        let Some(user_dirs) = directories::UserDirs::new() else {
            return vec![];
        };
        let mut path = user_dirs.home_dir().to_path_buf();
        path.push(".claude");
        path.push("history.jsonl");

        let Ok(file) = File::open(path) else {
            return vec![];
        };

        let mut history = vec![];
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if let Ok(row) = serde_json::from_str::<ClaudeHistoryRow>(&line) {
                history.push(row);
            }
        }
        history.reverse();
        history
    }

    use std::collections::HashMap;

    #[derive(Deserialize, Debug, Clone, Default)]
    pub struct ModelUsage {
        pub inputTokens: u64,
        pub outputTokens: u64,
        pub cacheReadInputTokens: u64,
        pub cacheCreationInputTokens: u64,
    }

    #[derive(Deserialize, Debug, Clone, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct ClaudeStats {
        pub model_usage: HashMap<String, ModelUsage>,
        pub total_sessions: u64,
        pub total_messages: u64,
        pub first_session_date: String,
    }

    pub fn load_stats() -> Option<ClaudeStats> {
        let user_dirs = directories::UserDirs::new()?;
        let mut path = user_dirs.home_dir().to_path_buf();
        path.push(".claude");
        path.push("stats-cache.json");

        let file = File::open(path).ok()?;
        serde_json::from_reader(BufReader::new(file)).ok()
    }
}
