use url::Url;
use crate::client::RunnerError;
use crate::pool::DescriptorPool;
use super::descriptor::{DescriptorProto, MethodDescriptorProto, ServiceDescriptorProto};

/// Represents a gRPC request.
#[derive(Debug)]
pub struct Request<'fds> {
    service: &'fds ServiceDescriptorProto,
    method: &'fds MethodDescriptorProto,
    input_message: &'fds DescriptorProto,
    output_message: &'fds DescriptorProto,
}

impl <'fds> Request<'fds> {

    /// Build a gRPC request given a set of descriptor, a URL and a body.
    pub fn try_from(
        descriptor_pool: &'fds DescriptorPool, url: Url, _body: &[u8]
    ) -> Result<Self, RunnerError> {
        // Parse the URL into (service FQN, method name).
        let (svc_fqn, method_name) = parse_grpc_path(&url)?;

        // Get the descriptor set.
        let fds = descriptor_pool.descriptor_set();

        // Get the symbols table.
        let symbols = descriptor_pool
            .symbols()
            .map_err(RunnerError::SymbolBuild)?;

        // Get service, method, input and output message type.
        let service = symbols
            .find_service(&svc_fqn)
            .ok_or_else(|| RunnerError::UnknownService {
                service: svc_fqn.clone(),
                method: method_name.clone(),
            })?;

        let method =
            symbols
                .find_method(service, &method_name)
                .ok_or_else(|| RunnerError::UnknownMethod {
                    service: svc_fqn.clone(),
                    method: method_name.clone(),
                })?;

        let input_message =
            symbols
                .resolve_method_input(method)
                .ok_or_else(|| RunnerError::UnresolvedType {
                    service: svc_fqn.clone(),
                    method: method_name.clone(),
                    type_name: method.input_type.clone().unwrap_or_default(),
                })?;
        let output_message =
            symbols
                .resolve_method_output(method)
                .ok_or_else(|| RunnerError::UnresolvedType {
                    service: svc_fqn.clone(),
                    method: method_name.clone(),
                    type_name: method.output_type.clone().unwrap_or_default(),
                })?;
        let request = Request {
            service,
            method,
            input_message,
            output_message,
        };
        println!("{:?}", request);
        Ok(request)
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