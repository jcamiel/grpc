use std::fmt;
use std::fmt::Formatter;

use url::Url;

use super::pool::DescriptorPool;
use super::symbols::SymbolError;
use super::request::Request;

pub struct Client {}



#[derive(Debug)]
pub enum RunnerError {
    /// URL path isn't of the form `/pkg.Service/Method`.
    InvalidUrl { url: Url },
    /// Building the symbol table failed (malformed descriptor set).
    SymbolBuild(SymbolError),
    /// No service registered under this FQN.
    UnknownService { service: String, method: String },
    /// Service exists but has no method with this local name.
    UnknownMethod { service: String, method: String },
    /// A method's `input_type` / `output_type` couldn't be resolved.
    UnresolvedType {
        service: String,
        method: String,
        type_name: String,
    },
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RunnerError::InvalidUrl { url } => {
                let path = url.path().trim_start_matches('/');
                write!(
                    f,
                    "Error running method, URL path '{path}' is not in expected format 'service/method'"
                )
            }
            RunnerError::SymbolBuild(_) => write!(f, "RunnerError::SymbolBuild"),
            RunnerError::UnknownService { service, method } => write!(
                f,
                "Error running method '{service}/{method}', service '{service}' not found"
            ),
            RunnerError::UnknownMethod { service, method } => write!(
                f,
                "Error running method '{service}/{method}', service '{service}' does not include a method '{method}'"
            ),
            RunnerError::UnresolvedType {
                service,
                method,
                type_name,
            } => write!(
                f,
                "Error running method '{service}/{method}', type '{type_name}' not found"
            ),
        }
    }
}

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    /// Run a gRPC request, given an `url` and a descriptor.
    pub fn run(&self, descriptor_pool: DescriptorPool, url: Url, body: &[u8]) -> Result<(), RunnerError> {
        let _request = Request::try_from(&descriptor_pool, url, body)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{
        DescriptorProto, FileDescriptorProto, FileDescriptorSet, MethodDescriptorProto,
        ServiceDescriptorProto,
    };

    // Builders to construct a `FileDescriptorSet` by hand for tests.
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

    fn body() -> Vec<u8> {
        vec![]
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
    fn run_reports_invalid_url() {
        let files = vec![];
        let p = pool(files);
        let u = url("http://localhost/pkg.Greeter/");
        let b = body();
        let err = Client::new().run(p.clone(), u, &b).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method, URL path 'pkg.Greeter/' is not in expected format 'service/method'"
        );

        let u = url("http://localhost");
        let err = Client::new().run(p, u, &b).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method, URL path '' is not in expected format 'service/method'"
        );
    }

    #[test]
    fn run_reports_empty_services() {
        // Empty descriptor set: no service registered under the FQN.
        let files = vec![file("pkg", vec![], vec![])];
        let p = pool(files);
        let b = body();
        let u = url("http://localhost/pkg.Greeter/SayHello");
        let err = Client::new()
            .run(p, u, &b)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHello', service 'pkg.Greeter' not found"
        )
    }

    #[test]
    fn run_reports_unknown_service() {
        let p = greeter_pool();
        let u = url("http://localhost/pkg.Foo/GetFoo");
        let b = body();

        let err = Client::new()
            .run(p, u, &b)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Foo/GetFoo', service 'pkg.Foo' not found"
        )
    }

    #[test]
    fn run_reports_method_not_found() {
        let p = greeter_pool();
        let u = url("http://localhost/pkg.Greeter/SayHi");
        let b = body();

        let err = Client::new()
            .run(p, u, &b)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHi', service 'pkg.Greeter' does not include a method 'SayHi'"
        )
    }

    #[test]
    fn run_reports_unresolved_input_type() {
        // Method's `input_type` points at a message that isn't in the set.
        let p = pool(vec![file(
            "pkg",
            vec![message("HelloReply")],
            vec![service(
                "Greeter",
                vec![method("SayHello", ".pkg.Missing", ".pkg.HelloReply")],
            )],
        )]);
        let u = url("http://localhost/pkg.Greeter/SayHello");
        let b = body();

        let err = Client::new()
            .run(p, u, &b)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHello', type '.pkg.Missing' not found"
        )
    }

    #[test]
    fn run_reports_unresolved_output_type() {
        // Input resolves, but `output_type` points at a missing message.
        let p = pool(vec![file(
            "pkg",
            vec![message("HelloRequest")],
            vec![service(
                "Greeter",
                vec![method("SayHello", ".pkg.HelloRequest", ".pkg.Missing")],
            )],
        )]);
        let u = url("http://localhost/pkg.Greeter/SayHello");
        let b = body();

        let err = Client::new()
            .run(p, u, &b)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHello', type '.pkg.Missing' not found"
        )
    }
}
