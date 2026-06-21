use super::pool::DescriptorPool;
use super::symbols::SymbolError;
use url::Url;

pub struct Client {}

#[derive(Debug)]
pub enum RunnerError {
    /// URL path isn't of the form `/pkg.Service/Method`.
    InvalidUrl,
    /// Building the symbol table failed (malformed descriptor set).
    SymbolBuild(SymbolError),
    /// No service registered under this FQN.
    UnknownService { fqn: String },
    /// Service exists but has no method with this local name.
    UnknownMethod { service: String, method: String },
    /// A method's `input_type` / `output_type` couldn't be resolved.
    UnresolvedType { fqn: String },
}

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    /// Run a gRPC request, given an `url` and a descriptor.
    pub fn run(&self, descriptor_pool: DescriptorPool, url: Url) -> Result<(), RunnerError> {
        // Parse the URL into (service FQN, method name).
        let (svc_fqn, method_name) = parse_grpc_path(&url)?;

        // Get the descriptor set.
        let fds = descriptor_pool.descriptor_set();
        println!("proto:");
        println!("{:#?}", fds);

        // Get the symbols table.
        let symbols = descriptor_pool
            .symbols()
            .map_err(RunnerError::SymbolBuild)?;
        println!("symbols:");
        println!("{symbols}");

        // Get service, method, input and output message type.
        let svc = symbols
            .find_service(&svc_fqn)
            .ok_or_else(|| RunnerError::UnknownService {
                fqn: svc_fqn.clone(),
            })?;
        println!("service:");
        println!("{:#?}", svc);

        let method =
            symbols
                .find_method(svc, &method_name)
                .ok_or_else(|| RunnerError::UnknownMethod {
                    service: svc_fqn.clone(),
                    method: method_name.clone(),
                })?;
        println!("method:");
        println!("{:#?}", method);

        let input_msg =
            symbols
                .resolve_method_input(method)
                .ok_or_else(|| RunnerError::UnresolvedType {
                    fqn: method.input_type.clone().unwrap_or_default(),
                })?;
        let output_msg =
            symbols
                .resolve_method_output(method)
                .ok_or_else(|| RunnerError::UnresolvedType {
                    fqn: method.output_type.clone().unwrap_or_default(),
                })?;
        println!("input message:");
        println!("{:#?}", input_msg);
        println!("output message:");
        println!("{:#?}", output_msg);

        Ok(())
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
        return Err(RunnerError::InvalidUrl);
    }
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 {
        return Err(RunnerError::InvalidUrl);
    }
    let method_name = parts.last().unwrap().to_string();
    let svc_fqn = parts[..parts.len() - 1].join(".");
    Ok((svc_fqn, method_name))
}
