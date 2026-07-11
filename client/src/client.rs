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
use std::fmt::Formatter;
use std::path::Path;
use std::{fmt, fs};

use hurl::http::{Header, HeaderVec, RequestedHttpVersion::Http2PriorKnowledge};
use hurl::runner;
use hurl::runner::{Input, RunnerOptionsBuilder, VariableSet};
use hurl::util::logger::{LoggerOptionsBuilder, Verbosity};
use url::Url;

use crate::request::Request;
use crate::request::body::RequestBodyError;
use crate::schema::pool::DescriptorPool;
use crate::schema::symbols::SymbolError;

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
    InvalidRequestBody {
        service: String,
        method: String,
        error: RequestBodyError,
    },
    /// A runtime error
    Runtime {
        service: String,
        method: String,
        error: String,
    },
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RunnerError::InvalidUrl { url } => {
                let path = url.path().trim_start_matches('/');
                write!(
                    f,
                    "Error running method: URL path '{path}' is not in expected format 'service/method'"
                )
            }
            RunnerError::SymbolBuild(_) => write!(f, "RunnerError::SymbolBuild"),
            RunnerError::UnknownService { service, method } => write!(
                f,
                "Error running method '{service}/{method}': service '{service}' not found"
            ),
            RunnerError::UnknownMethod { service, method } => write!(
                f,
                "Error running method '{service}/{method}': service '{service}' does not include a method '{method}'"
            ),
            RunnerError::UnresolvedType {
                service,
                method,
                type_name,
            } => write!(
                f,
                "Error running method '{service}/{method}': type '{type_name}' not found"
            ),

            RunnerError::Runtime {
                service,
                method,
                error,
            } => write!(f, "Error running method '{service}/{method}': {error}"),
            RunnerError::InvalidRequestBody {
                service,
                method,
                error,
            } => write!(f, "Error running method '{service}/{method}': {error}"),
        }
    }
}

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    /// Run a gRPC request, given an `url` and a descriptor.
    pub fn run(
        &self,
        descriptor_pool: DescriptorPool,
        url: Url,
        body: &[u8],
    ) -> Result<(), RunnerError> {
        // Constructs the gRPC request
        let request = Request::try_from(&descriptor_pool, &url, body)?;

        // Write the request body to file so we can inject it in Hurl
        let body_path = Path::new("build/body.in");
        fs::write(body_path, request.request_body()).unwrap();

        let content = format!(
            r#"#
            POST {url}
            file,build/body.in;
        "#
        );
        let filename = Input::new("sample.hurl");
        let variables = VariableSet::new();
        let mut headers = HeaderVec::new();
        headers.push(Header::new("Content-Type", "application/grpc"));
        headers.push(Header::new("TE", "trailers"));

        let runner_opts = RunnerOptionsBuilder::new()
            .http_version(Http2PriorKnowledge)
            .headers(headers)
            .build();
        let logger_opts = LoggerOptionsBuilder::new()
            .verbosity(Some(Verbosity::Verbose))
            .build();

        // Run the Hurl sample
        let result = runner::run(
            &content,
            Some(&filename),
            &runner_opts,
            &variables,
            &logger_opts,
        )
        .map_err(|e| RunnerError::Runtime {
            service: request.service_name().to_string(),
            method: request.method_name().to_string(),
            error: e.clone(),
        })?;

        let entry = &result.entries[0];
        let response = &result.entries[0].calls[0].response;
        let response_headers = &response.headers;
        let grpc_status = response_headers
            .get("grpc-status")
            .unwrap()
            .value
            .parse::<u32>()
            .unwrap();
        let grpc_message = &response_headers.get("grpc-message").unwrap().value;

        // println!("{result:#?}");
        println!("curl cmd:     {}", entry.curl_cmd);
        println!("grpc-status:  {grpc_status}");
        println!("grpc-message: {grpc_message}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::descriptor::{
        DescriptorProto, FileDescriptorProto, FileDescriptorSet, MethodDescriptorProto,
        ServiceDescriptorProto,
    };
    use crate::schema::resolve::resolve_fqns;

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
            ..Default::default()
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
        let mut ds = FileDescriptorSet { files };
        resolve_fqns(&mut ds);
        DescriptorPool::from_descriptor_set(ds)
    }

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn body() -> Vec<u8> {
        vec![]
    }

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
            "Error running method: URL path 'pkg.Greeter/' is not in expected format 'service/method'"
        );

        let u = url("http://localhost");
        let err = Client::new().run(p, u, &b).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method: URL path '' is not in expected format 'service/method'"
        );
    }

    #[test]
    fn run_reports_empty_services() {
        // Empty descriptor set: no service registered under the FQN.
        let files = vec![file("pkg", vec![], vec![])];
        let p = pool(files);
        let b = body();
        let u = url("http://localhost/pkg.Greeter/SayHello");
        let err = Client::new().run(p, u, &b).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHello': service 'pkg.Greeter' not found"
        )
    }

    #[test]
    fn run_reports_unknown_service() {
        let p = greeter_pool();
        let u = url("http://localhost/pkg.Foo/GetFoo");
        let b = body();

        let err = Client::new().run(p, u, &b).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Foo/GetFoo': service 'pkg.Foo' not found"
        )
    }

    #[test]
    fn run_reports_method_not_found() {
        let p = greeter_pool();
        let u = url("http://localhost/pkg.Greeter/SayHi");
        let b = body();

        let err = Client::new().run(p, u, &b).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHi': service 'pkg.Greeter' does not include a method 'SayHi'"
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

        let err = Client::new().run(p, u, &b).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHello': type '.pkg.Missing' not found"
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

        let err = Client::new().run(p, u, &b).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Error running method 'pkg.Greeter/SayHello': type '.pkg.Missing' not found"
        )
    }
}
