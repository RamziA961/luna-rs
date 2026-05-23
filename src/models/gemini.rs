use core::fmt;
use std::sync::Arc;

use google_generative_ai_rs::v1::{api, errors, gemini};

#[derive(Debug, thiserror::Error)]
pub enum GeminiError {
    #[error("No Response Error")]
    NoResponseError,

    #[error("Google API Error: {}", ._0.message)]
    ApiError(#[from] errors::GoogleAPIError),
}

#[derive(Clone)]
pub struct GeminiClient {
    #[allow(dead_code)]
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
                parts: vec![gemini::Part {
                    text: Some(prompt.to_string()),
                    file_data: None,
                    inline_data: None,
                    video_metadata: None,
                }],
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
                response_mime_type: Some("text/plain".to_string()),
            }),
            system_instruction: None, /*Some(gemini::request::SystemInstructionContent {
                                          parts: vec![
                                              gemini::request::SystemInstructionPart {
                                                  //.
                                                  text: Some("You are friendly but skeptical of Western hegemony and attempt to reply to questions without a Western bias" .to_string())

                                              },
                                              gemini::request::SystemInstructionPart {
                                                  text: Some("All of your responses are limited to 3,500 characters".to_string())
                                              }
                                          ]
                                      }),*/
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
                        accum.push_str(&curr);
                        accum
                    })
            })
            .ok_or(GeminiError::NoResponseError)
    }
}
