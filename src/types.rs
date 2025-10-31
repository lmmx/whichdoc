use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Coordinate {
    pub reason: String,
    pub message: Option<Message>,
    pub package_id: Option<String>,
    pub manifest_path: Option<String>,
    pub target: Option<Target>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    #[serde(rename = "$message_type")]
    pub message_type: String,
    pub children: Vec<Child>,
    pub code: Code,
    pub level: String,
    pub message: String,
    pub rendered: String,
    pub spans: Vec<Span>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Child {
    pub children: Vec<Option<serde_json::Value>>,
    pub code: Option<serde_json::Value>,
    pub level: String,
    pub message: String,
    pub rendered: Option<serde_json::Value>,
    pub spans: Vec<Option<serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Code {
    pub code: String,
    pub explanation: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Span {
    pub byte_end: i64,
    pub byte_start: i64,
    pub column_end: i64,
    pub column_start: i64,
    pub expansion: Option<serde_json::Value>,
    pub file_name: String,
    pub is_primary: bool,
    pub label: Option<serde_json::Value>,
    pub line_end: i64,
    pub line_start: i64,
    pub suggested_replacement: Option<serde_json::Value>,
    pub suggestion_applicability: Option<serde_json::Value>,
    pub text: Vec<Text>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Text {
    pub highlight_end: i64,
    pub highlight_start: i64,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Target {
    pub crate_types: Vec<String>,
    pub doc: bool,
    pub doctest: bool,
    pub edition: String,
    pub kind: Vec<String>,
    pub name: String,
    pub src_path: String,
    pub test: bool,
}
