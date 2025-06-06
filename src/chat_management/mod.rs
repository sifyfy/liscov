pub mod local_moderation;
pub mod message_classifier;
pub mod message_filter;
pub mod question_detector;
pub mod search_engine;

pub use local_moderation::*;
pub use message_classifier::*;
pub use message_filter::*;
pub use question_detector::*;
pub use search_engine::*;

use serde::{Deserialize, Serialize};

/// 質問のカテゴリ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QuestionCategory {
    Technical,
    General,
    Request,
    Feedback,
    Other,
}

impl QuestionCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestionCategory::Technical => "technical",
            QuestionCategory::General => "general",
            QuestionCategory::Request => "request",
            QuestionCategory::Feedback => "feedback",
            QuestionCategory::Other => "other",
        }
    }
}

/// 質問の優先度
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    High = 3,
    Medium = 2,
    Low = 1,
}

/// 回答方法
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnswerMethod {
    LiveResponse,
    TemplateResponse(String),
    Ignored,
    Deferred,
}
