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

//! This module processes a raw [`FileDescriptorSet`] (the AST produced from the wire) and resolves
//! the fully qualified name and other field resolutions. We do it here, as post-processing, after
//! parsing so the `.protoset` parsing is independant of the resolution.

use super::descriptor::{
    DescriptorProto, EnumDescriptorProto, FileDescriptorSet, ServiceDescriptorProto,
};

/// Top-down walk that fills in `fqn` on every message, enum, service, and method in the set.
pub fn resolve_fqns(set: &mut FileDescriptorSet) {
    for file in &mut set.files {
        let pkg = file.package.as_deref().unwrap_or("").to_string();
        for msg in &mut file.message_types {
            resolve_message(msg, &pkg);
        }
        for en in &mut file.enum_types {
            resolve_enum(en, &pkg);
        }
        for svc in &mut file.services {
            resolve_service(svc, &pkg);
        }
    }
}

fn resolve_message(msg: &mut DescriptorProto, parent_fqn: &str) {
    msg.fqn = join(parent_fqn, msg.name.as_deref().unwrap_or(""));
    // Clone once for the recursion; message's own fqn is now the parent path
    // for everything nested inside it.
    let fqn = msg.fqn.clone();
    for nested in &mut msg.nested_types {
        resolve_message(nested, &fqn);
    }
    for en in &mut msg.enum_types {
        resolve_enum(en, &fqn);
    }
}

fn resolve_enum(en: &mut EnumDescriptorProto, parent_fqn: &str) {
    en.fqn = join(parent_fqn, en.name.as_deref().unwrap_or(""));
}

fn resolve_service(svc: &mut ServiceDescriptorProto, parent_fqn: &str) {
    svc.fqn = join(parent_fqn, svc.name.as_deref().unwrap_or(""));
    let fqn = svc.fqn.clone();
    for m in &mut svc.methods {
        m.fqn = join(&fqn, m.name.as_deref().unwrap_or(""));
    }
}

/// Join a parent FQN with a local name, dropping the dot when parent is the root scope.
fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}.{name}")
    }
}
