use core::fmt;
use std::sync::Arc;

use google_generative_ai_rs::v1::{api, errors, gemini};

#[derive(Debug, thiserror::Error)]
pub enum GeminiError {
    #[error("No Response Error")]
    NoResponseError,

    #[error("{0}")]
    ApiError(errors::GoogleAPIError),
}

impl From<errors::GoogleAPIError> for GeminiError {
    fn from(value: errors::GoogleAPIError) -> Self {
        Self::ApiError(value)
    }
}

#[derive(Clone)]
pub struct GeminiClient {
    api_key: String,
    client: Arc<api::Client>,
}

impl fmt::Debug for GeminiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GeminiClient {{ client: <...> }}")
    }
}

impl GeminiClient {
    pub fn new(api_key: &str) -> Self {
        let client = api::Client::new_from_model(gemini::Model::GeminiPro, api_key.to_string());
        Self {
            api_key: api_key.to_string(),
            client: Arc::new(client),
        }
    }

    pub fn model(&self) -> String {
        self.client.model.to_string()
    }

    pub async fn text_request(&self, prompt: &str) -> Result<String, GeminiError> {
        let request = gemini::request::Request {
            contents: vec![gemini::Content {
                role: gemini::Role::User,
                parts: vec![
                    gemini::Part {
                        text: Some(format!("Answer the following using less than 3500 characters into total, including whitespace and new line characters. {prompt}")),
                        file_data: None,
                        inline_data: None,
                        video_metadata: None,
                    }
                ]
            }],
            tools: vec![],
            safety_settings: vec![],
            generation_config: Some(gemini::request::GenerationConfig {
                temperature: None,
                top_p: None,
                top_k: None,
                candidate_count: Some(1),
                max_output_tokens: None,
                stop_sequences: None,
            }),
        };

        self.client
            .post(30, &request)
            .await?
            .rest()
            .as_ref()
            .and_then(|res| res.candidates.first())
            .and_then(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .filter_map(|p| p.text.clone())
                    .reduce(|mut accum, curr| {
                        accum.extend(curr.chars());
                        accum
                    })
            })
            .ok_or_else(|| GeminiError::NoResponseError)
    }
}
