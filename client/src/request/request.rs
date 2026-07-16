/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2026 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
use url::Url;

use super::body::RequestBody;
use crate::client::RunnerError;
use crate::schema::pool::DescriptorPool;
use crate::wire::writer::Writer;

/// Represents a gRPC request.
#[derive(Debug)]
pub struct Request {
    /// Fully qualified service name
    service_name: String,
    method_name: String,
    request_body: RequestBody,
}

impl Request {
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    /// Build a gRPC request given a set of descriptor, a URL and a body.
    pub fn try_from(
        descriptor_pool: &DescriptorPool,
        url: &Url,
        body: &[u8],
    ) -> Result<Self, RunnerError> {
        // Parse the URL into (service FQN, method name).
        let (svc_fqn, method_name) = parse_grpc_path(url)?;

        // Get the symbols table.
        let symbols = descriptor_pool
            .symbols()
            .map_err(RunnerError::SymbolBuild)?;

        // Get service, method, input and output message type.
        let service = symbols
            .find_service(&svc_fqn)
            .ok_or(RunnerError::UnknownService {
                service: svc_fqn.clone(),
                method: method_name.clone(),
            })?;

        let method =
            symbols
                .find_method(service, &method_name)
                .ok_or(RunnerError::UnknownMethod {
                    service: svc_fqn.clone(),
                    method: method_name.clone(),
                })?;

        let input_message =
            symbols
                .resolve_method_input(method)
                .ok_or(RunnerError::UnresolvedType {
                    service: svc_fqn.clone(),
                    method: method_name.clone(),
                    type_name: method.input_type.clone().unwrap_or_default(),
                })?;
        let _output_message =
            symbols
                .resolve_method_output(method)
                .ok_or(RunnerError::UnresolvedType {
                    service: svc_fqn.clone(),
                    method: method_name.clone(),
                    type_name: method.output_type.clone().unwrap_or_default(),
                })?;

        // Parse the body
        let request_body =
            RequestBody::try_new(body, input_message, &symbols).map_err(|error| {
                RunnerError::InvalidRequestBody {
                    service: svc_fqn.clone(),
                    method: method_name.clone(),
                    error,
                }
            })?;

        let request = Request {
            service_name: svc_fqn,
            method_name,
            request_body,
        };
        Ok(request)
    }
}

impl Request {
    pub fn encode(&self, writer: &mut Writer) {
        self.request_body.encode(writer);
    }
}

/// Parse a gRPC URL path into `(service FQN, method name)`.
///
/// gRPC paths take the form `/pkg.Service/Method` — a single slash-separated service+method pair.
/// We accept additional leading path segments and treat everything except the last segment as the
/// service name (joined with `.`), which matches the older curl-style convention of typing the
/// service path directly into the URL.
fn parse_grpc_path(url: &Url) -> Result<(String, String), RunnerError> {
    let path = url.path();
    if path.is_empty() || path == "/" {
        return Err(RunnerError::InvalidUrl { url: url.clone() });
    }
    let parts = path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(RunnerError::InvalidUrl { url: url.clone() });
    }
    let method_name = parts.last().unwrap().to_string();
    let svc_fqn = parts[..parts.len() - 1].join(".");
    Ok((svc_fqn, method_name))
}
