use std::fmt;
use std::fmt::Formatter;

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
    UnknownService { fqn: String, method: String },
    /// Service exists but has no method with this local name.
    UnknownMethod { service: String, method: String },
    /// A method's `input_type` / `output_type` couldn't be resolved.
    UnresolvedType { fqn: String },
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RunnerError::InvalidUrl => write!(f, "RunnerError::InvalidUrl"),
            RunnerError::SymbolBuild(_) => write!(f, "RunnerError::SymbolBuild"),
            RunnerError::UnknownService { fqn, method } => write!(
                f,
                "Error running method '{fqn}/{method}', service '{fqn}' not found"
            ),
            RunnerError::UnknownMethod { service, method } => write!(
                f,
                "Error running method '{service}/{method}', service '{service}' does not include a method '{method}'"
            ),
            RunnerError::UnresolvedType { .. } => write!(f, "RunnerError::UnresolvedType"),
        }
    }
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
                method: method_name.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{
        DescriptorProto, FileDescriptorProto, FileDescriptorSet, MethodDescriptorProto,
        ServiceDescriptorProto,
    };

    // --- Builders to construct a `FileDescriptorSet` by hand. ---

    fn message(name: &str) -> DescriptorProto {
        DescriptorProto {
            name: Some(name.to_string()),
            ..Default::default()
        }
    }

    fn method(name: &str, input_type: &str, output_type: &str) -> MethodDescriptorProto {
        MethodDescriptorProto {
            name: Some(name.to_string()),
            input_type: Some(input_type.to_string()),
            output_type: Some(output_type.to_string()),
            ..Default::default()
        }
    }

    fn service(name: &str, methods: Vec<MethodDescriptorProto>) -> ServiceDescriptorProto {
        ServiceDescriptorProto {
            name: Some(name.to_string()),
            methods,
        }
    }

    fn file(
        package: &str,
        message_types: Vec<DescriptorProto>,
        services: Vec<ServiceDescriptorProto>,
    ) -> FileDescriptorProto {
        FileDescriptorProto {
            name: Some("test.proto".to_string()),
            package: Some(package.to_string()),
            message_types,
            services,
            ..Default::default()
        }
    }

    fn pool(files: Vec<FileDescriptorProto>) -> DescriptorPool {
        DescriptorPool::from_descriptor_set(FileDescriptorSet { files })
    }

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    /// A descriptor set with one fully-wired service: `pkg.Greeter/SayHello`
    /// taking `pkg.HelloRequest` and returning `pkg.HelloReply`.
    fn greeter_pool() -> DescriptorPool {
        pool(vec![file(
            "pkg",
            vec![message("HelloRequest"), message("HelloReply")],
            vec![service(
                "Greeter",
                vec![method("SayHello", ".pkg.HelloRequest", ".pkg.HelloReply")],
            )],
        )])
    }

    #[test]
    fn run_reports_empty_services() {
        // Empty descriptor set: no service registered under the FQN.
        let files = vec![file("pkg", vec![], vec![])];
        let p = pool(files);
        let err = Client::new()
            .run(p, url("http://localhost/pkg.Greeter/SayHello"))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHello', service 'pkg.Greeter' not found"
        )
    }

    #[test]
    fn run_reports_unknown_service() {
        let p = greeter_pool();
        let err = Client::new()
            .run(p, url("http://localhost/pkg.Foo/GetFoo"))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Foo/GetFoo', service 'pkg.Foo' not found"
        )
    }

    #[test]
    fn run_reports_method_not_found() {
        let p = greeter_pool();
        let err = Client::new()
            .run(p, url("http://localhost/pkg.Greeter/SayHi"))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHi', service 'pkg.Greeter' does not include a method 'SayHi'"
        )
    }

    //
    //
    //
    // #[test]
    // fn run_rejects_root_path() {
    //     let err = Client::new()
    //         .run(greeter_pool(), url("http://localhost/"))
    //         .unwrap_err();
    //     assert!(matches!(err, RunnerError::InvalidUrl));
    // }
    //
    // #[test]
    // fn run_rejects_path_without_method() {
    //     // A single path segment can't be split into service + method.
    //     let err = Client::new()
    //         .run(greeter_pool(), url("http://localhost/pkg.Greeter"))
    //         .unwrap_err();
    //     assert!(matches!(err, RunnerError::InvalidUrl));
    // }
    //
    // #[test]
    // fn run_reports_duplicate_symbols() {
    //     // Two messages sharing an FQN make `SymbolTable::build` fail.
    //     let p = pool(vec![file(
    //         "pkg",
    //         vec![message("Dup"), message("Dup")],
    //         vec![],
    //     )]);
    //     let err = Client::new()
    //         .run(p, url("http://localhost/pkg.Greeter/SayHello"))
    //         .unwrap_err();
    //     assert!(matches!(err, RunnerError::SymbolBuild(_)));
    // }
    //
    //
    //
    // #[test]
    // fn run_reports_unknown_method() {
    //     // Service exists but has no method with this local name.
    //     let err = Client::new()
    //         .run(greeter_pool(), url("http://localhost/pkg.Greeter/Missing"))
    //         .unwrap_err();
    //
    //     match err {
    //         RunnerError::UnknownMethod { service, method } => {
    //             assert_eq!(service, "pkg.Greeter");
    //             assert_eq!(method, "Missing");
    //         }
    //         other => panic!("expected UnknownMethod, got {other:?}"),
    //     }
    // }
    //
    // #[test]
    // fn run_reports_unresolved_input_type() {
    //     // Method's `input_type` points at a message that isn't in the set.
    //     let p = pool(vec![file(
    //         "pkg",
    //         vec![message("HelloReply")],
    //         vec![service(
    //             "Greeter",
    //             vec![method("SayHello", ".pkg.Missing", ".pkg.HelloReply")],
    //         )],
    //     )]);
    //     let err = Client::new()
    //         .run(p, url("http://localhost/pkg.Greeter/SayHello"))
    //         .unwrap_err();
    //     match err {
    //         RunnerError::UnresolvedType { fqn } => assert_eq!(fqn, ".pkg.Missing"),
    //         other => panic!("expected UnresolvedType, got {other:?}"),
    //     }
    // }
    //
    // #[test]
    // fn run_reports_unresolved_output_type() {
    //     // Input resolves, but `output_type` points at a missing message.
    //     let p = pool(vec![file(
    //         "pkg",
    //         vec![message("HelloRequest")],
    //         vec![service(
    //             "Greeter",
    //             vec![method("SayHello", ".pkg.HelloRequest", ".pkg.Missing")],
    //         )],
    //     )]);
    //     let err = Client::new()
    //         .run(p, url("http://localhost/pkg.Greeter/SayHello"))
    //         .unwrap_err();
    //     match err {
    //         RunnerError::UnresolvedType { fqn } => assert_eq!(fqn, ".pkg.Missing"),
    //         other => panic!("expected UnresolvedType, got {other:?}"),
    //     }
    // }
}
