/*
 * Copyright Cedar Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Performance benchmarks for multi-valued authorization decisions
//!
//! This benchmark suite validates the performance characteristics of the
//! multi-valued decision system against the following targets:
//!
//! - Binary authorization overhead: < 5%
//! - Multi-valued authorization overhead: < 15%
//! - Throughput: > 10,000 requests/second
//!
//! Run with: cargo bench --bench multi_decision_bench

use cedar_policy_core::{
    ast::{Context, EntityUID, PolicySet, Request},
    authorizer::Authorizer,
    config::{DecisionConfig, DecisionTypeConfig},
    entities::{decision_registry::DecisionTypeRegistry, Entities},
    evaluator::DecisionSet,
    extensions::Extensions,
    parser,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Setup common test data
fn setup_simple_policy_set() -> PolicySet {
    let mut pset = PolicySet::new();

    let permit_src = r#"
        permit(principal, action == Action::"read", resource);
    "#;

    pset.add_static(
        parser::parse_policy(
            Some(cedar_policy_core::ast::PolicyID::from_string("permit1")),
            permit_src,
        )
        .expect("Failed to parse policy"),
    )
    .expect("Failed to add policy");

    pset
}

fn setup_multi_policy_set() -> PolicySet {
    let mut pset = PolicySet::new();

    let permit_src = r#"
        permit(principal, action == Action::"read", resource);
    "#;

    let forbid_src = r#"
        forbid(principal, action == Action::"delete", resource)
        when { resource.archived == true };
    "#;

    pset.add_static(
        parser::parse_policy(
            Some(cedar_policy_core::ast::PolicyID::from_string("permit1")),
            permit_src,
        )
        .expect("Failed to parse policy"),
    )
    .expect("Failed to add policy");

    pset.add_static(
        parser::parse_policy(
            Some(cedar_policy_core::ast::PolicyID::from_string("forbid1")),
            forbid_src,
        )
        .expect("Failed to parse policy"),
    )
    .expect("Failed to add policy");

    pset
}

fn create_request() -> Request {
    use cedar_policy_core::ast::RequestSchemaAllPass;
    Request::new(
        (
            EntityUID::with_eid_and_type("User", "alice").expect("Failed to create principal"),
            None,
        ),
        (
            EntityUID::with_eid_and_type("Action", "read").expect("Failed to create action"),
            None,
        ),
        (
            EntityUID::with_eid_and_type("Resource", "doc1").expect("Failed to create resource"),
            None,
        ),
        Context::empty(),
        None::<&RequestSchemaAllPass>,
        Extensions::none(),
    )
    .expect("Failed to create request")
}

/// Benchmark binary authorization (baseline)
fn bench_binary_authorization(c: &mut Criterion) {
    let authorizer = Authorizer::new();
    let policy_set = setup_simple_policy_set();
    let entities = Entities::new();
    let request = create_request();

    let mut group = c.benchmark_group("binary_authorization");
    group.throughput(Throughput::Elements(1));

    group.bench_function("is_authorized", |b| {
        b.iter(|| {
            let response = authorizer.is_authorized(
                black_box(request.clone()),
                black_box(&policy_set),
                black_box(&entities),
            );
            black_box(response);
        });
    });

    group.finish();
}

/// Benchmark multi-valued authorization
fn bench_multi_valued_authorization(c: &mut Criterion) {
    let authorizer = Authorizer::new();
    let policy_set = setup_simple_policy_set();
    let entities = Entities::new();
    let request = create_request();

    let mut group = c.benchmark_group("multi_valued_authorization");
    group.throughput(Throughput::Elements(1));

    group.bench_function("decisions", |b| {
        b.iter(|| {
            let response = authorizer.decisions(
                black_box(request.clone()),
                black_box(&policy_set),
                black_box(&entities),
            );
            black_box(response);
        });
    });

    // Test with legacy conversion
    group.bench_function("decisions_with_legacy", |b| {
        b.iter(|| {
            let response = authorizer.decisions(
                black_box(request.clone()),
                black_box(&policy_set),
                black_box(&entities),
            );
            let legacy = response.into_legacy();
            black_box(legacy);
        });
    });

    group.finish();
}

/// Benchmark authorization with multiple policies
fn bench_multi_policy_authorization(c: &mut Criterion) {
    let authorizer = Authorizer::new();
    let policy_set = setup_multi_policy_set();
    let entities = Entities::new();
    let request = create_request();

    let mut group = c.benchmark_group("multi_policy_authorization");
    group.throughput(Throughput::Elements(1));

    group.bench_function("binary_multi_policy", |b| {
        b.iter(|| {
            let response = authorizer.is_authorized(
                black_box(request.clone()),
                black_box(&policy_set),
                black_box(&entities),
            );
            black_box(response);
        });
    });

    group.bench_function("multi_valued_multi_policy", |b| {
        b.iter(|| {
            let response = authorizer.decisions(
                black_box(request.clone()),
                black_box(&policy_set),
                black_box(&entities),
            );
            black_box(response);
        });
    });

    group.finish();
}

/// Benchmark decision registry operations
fn bench_registry_operations(c: &mut Criterion) {
    let config = DecisionConfig {
        decision_types: vec![
            DecisionTypeConfig {
                name: "allow".to_string(),
                precedence: 100,
                combinable: true,
                exclusive: false,
            },
            DecisionTypeConfig {
                name: "deny".to_string(),
                precedence: 200,
                combinable: false,
                exclusive: true,
            },
            DecisionTypeConfig {
                name: "alert".to_string(),
                precedence: 50,
                combinable: true,
                exclusive: false,
            },
            DecisionTypeConfig {
                name: "validate".to_string(),
                precedence: 60,
                combinable: true,
                exclusive: false,
            },
            DecisionTypeConfig {
                name: "audit".to_string(),
                precedence: 40,
                combinable: true,
                exclusive: false,
            },
        ],
        combination_rules: vec![],
        conflict_resolution: "precedence".to_string(),
    };

    let mut group = c.benchmark_group("registry_operations");

    group.bench_function("create_registry", |b| {
        b.iter(|| {
            let registry = DecisionTypeRegistry::from_config(black_box(&config));
            black_box(registry);
        });
    });

    let registry = DecisionTypeRegistry::from_config(&config);

    group.bench_function("lookup_by_name", |b| {
        b.iter(|| {
            let id = registry.get_id(black_box("allow"));
            black_box(id);
        });
    });

    group.bench_function("lookup_by_id", |b| {
        b.iter(|| {
            let name = registry.get_name(black_box(
                cedar_policy_core::entities::decision_registry::DecisionTypeId::ALLOW,
            ));
            black_box(name);
        });
    });

    group.finish();
}

/// Benchmark decision set operations
fn bench_decision_set_operations(c: &mut Criterion) {
    use cedar_policy_core::ast::PolicyID;
    use cedar_policy_core::entities::decision_registry::DecisionTypeId;

    let config = DecisionConfig {
        decision_types: vec![
            DecisionTypeConfig {
                name: "allow".to_string(),
                precedence: 100,
                combinable: true,
                exclusive: false,
            },
            DecisionTypeConfig {
                name: "deny".to_string(),
                precedence: 200,
                combinable: false,
                exclusive: true,
            },
            DecisionTypeConfig {
                name: "alert".to_string(),
                precedence: 50,
                combinable: true,
                exclusive: false,
            },
        ],
        combination_rules: vec![],
        conflict_resolution: "precedence".to_string(),
    };

    let registry = DecisionTypeRegistry::from_config(&config);
    let mut group = c.benchmark_group("decision_set_operations");

    let alert_id = registry.get_id("alert").unwrap();

    group.bench_function("create_and_add", |b| {
        b.iter(|| {
            let mut set = DecisionSet::new(black_box(registry.clone()));
            set.add(
                black_box(DecisionTypeId::ALLOW),
                black_box(PolicyID::from_string("p1")),
            );
            set.add(
                black_box(alert_id),
                black_box(PolicyID::from_string("p2")),
            );
            black_box(set);
        });
    });

    let mut set = DecisionSet::new(registry.clone());
    set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p1"));
    set.add(alert_id, PolicyID::from_string("p2"));

    group.bench_function("query_has", |b| {
        b.iter(|| {
            let has_allow = set.has(black_box(DecisionTypeId::ALLOW));
            black_box(has_allow);
        });
    });

    group.bench_function("to_decision", |b| {
        b.iter(|| {
            let decision = set.to_decision();
            black_box(decision);
        });
    });

    group.finish();
}

/// Benchmark configuration loading and validation
fn bench_config_loading(c: &mut Criterion) {
    let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
    combinable: true
    exclusive: false
  - name: deny
    precedence: 200
    combinable: false
    exclusive: true
  - name: alert
    precedence: 50
    combinable: true
    exclusive: false
  - name: validate
    precedence: 60
    combinable: true
    exclusive: false
  - name: audit
    precedence: 40
    combinable: true
    exclusive: false

combination_rules:
  - when: [deny, "*"]
    then: exclusive
    result: [deny]
  - when: [allow, alert]
    then: merge

conflict_resolution: precedence
"#;

    let mut group = c.benchmark_group("config_loading");

    group.bench_function("parse_yaml", |b| {
        b.iter(|| {
            let config = DecisionConfig::from_str(black_box(yaml)).expect("Failed to parse");
            black_box(config);
        });
    });

    group.bench_function("parse_and_create_registry", |b| {
        b.iter(|| {
            let config = DecisionConfig::from_str(black_box(yaml)).expect("Failed to parse");
            let registry = DecisionTypeRegistry::from_config(&config);
            black_box(registry);
        });
    });

    group.finish();
}

/// Benchmark varying number of decision types
fn bench_decision_type_scaling(c: &mut Criterion) {
    use cedar_policy_core::ast::PolicyID;
    use cedar_policy_core::entities::decision_registry::DecisionTypeId;

    let mut group = c.benchmark_group("decision_type_scaling");

    for num_types in [2, 3, 5, 10].iter() {
        let mut decision_types = vec![
            DecisionTypeConfig {
                name: "allow".to_string(),
                precedence: 100,
                combinable: true,
                exclusive: false,
            },
            DecisionTypeConfig {
                name: "deny".to_string(),
                precedence: 200,
                combinable: false,
                exclusive: true,
            },
        ];

        for i in 0..(*num_types - 2) {
            decision_types.push(DecisionTypeConfig {
                name: format!("custom{}", i),
                precedence: 50 - i as u32,
                combinable: true,
                exclusive: false,
            });
        }

        let config = DecisionConfig {
            decision_types,
            combination_rules: vec![],
            conflict_resolution: "precedence".to_string(),
        };

        let registry = DecisionTypeRegistry::from_config(&config);

        // Collect custom decision type IDs
        let custom_ids: Vec<_> = (0..(*num_types - 2))
            .filter_map(|i| registry.get_id(&format!("custom{}", i)))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("decision_set_operations", num_types),
            num_types,
            |b, _| {
                b.iter(|| {
                    let mut set = DecisionSet::new(registry.clone());
                    set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p1"));
                    for (i, custom_id) in custom_ids.iter().enumerate() {
                        set.add(*custom_id, PolicyID::from_string(&format!("p{}", i + 2)));
                    }
                    let decision = set.to_decision();
                    black_box(decision);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_binary_authorization,
    bench_multi_valued_authorization,
    bench_multi_policy_authorization,
    bench_registry_operations,
    bench_decision_set_operations,
    bench_config_loading,
    bench_decision_type_scaling,
);

criterion_main!(benches);
