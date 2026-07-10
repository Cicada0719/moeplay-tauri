use crate::ai::contracts::{
    AiCapabilities, AiProviderConfig, StructuredRequest, StructuredResponse,
};
use crate::ai::endpoint::ValidatedEndpoint;
use crate::ai::error::AiResult;
use crate::ai::governance::CancellationGuard;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialRequirement {
    None,
    BearerSecret,
}

/// A secret-free transport DTO. Credential material is injected by the HTTP
/// transport after endpoint-origin authorization and is never serialized here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
    pub credential: CredentialRequirement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterHttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

pub trait AiProviderAdapter: Send + Sync {
    fn config(&self) -> &AiProviderConfig;
    fn capabilities(&self) -> AiCapabilities;
    fn endpoint(&self) -> &ValidatedEndpoint;

    fn build_request(
        &self,
        request: &StructuredRequest,
        cancellation: &CancellationGuard,
    ) -> AiResult<AdapterHttpRequest>;

    fn parse_response(
        &self,
        response: &AdapterHttpResponse,
        cancellation: &CancellationGuard,
    ) -> AiResult<StructuredResponse>;
}
