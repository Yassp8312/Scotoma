use std::{collections::BTreeMap, process::Command};

const ORDERS: [&str; 6] = ["1,2,3", "1,3,2", "2,1,3", "2,3,1", "3,1,2", "3,2,1"];

fn run(mode: &str, order: &str) -> BTreeMap<String, String> {
    let output = Command::new(env!("CARGO_BIN_EXE_v3_adversarial_relay"))
        .args(["--mode", mode, "--order", order])
        .output()
        .expect("adversarial relay process starts");
    assert!(
        output.status.success(),
        "process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let line = String::from_utf8(output.stdout).expect("ASCII JSON report");
    parse_flat_json(line.trim())
}

fn parse_flat_json(line: &str) -> BTreeMap<String, String> {
    assert!(line.starts_with('{') && line.ends_with('}'));
    line[1..line.len() - 1]
        .split(',')
        .map(|field| {
            let (key, value) = field.split_once(':').expect("key/value report field");
            (
                key.trim_matches('"').to_owned(),
                value.trim_matches('"').to_owned(),
            )
        })
        .collect()
}

#[test]
fn honest_duplicate_and_reordered_schedules_never_confirm_a_pair() {
    for order in ORDERS {
        for mode in ["honest", "duplicate", "reorder"] {
            let report = run(mode, order);
            assert_eq!(report["ambiguous_pair_confirmed"], "false");
            assert_eq!(report["confirmed"], "0");
            assert_eq!(report["terminal"], "3");
        }
    }
}

#[test]
fn hidden_or_delayed_collision_is_an_indistinguishability_counterexample() {
    for order in ORDERS {
        for mode in ["hide", "delay"] {
            let report = run(mode, order);
            assert_eq!(report["ambiguous_pair_confirmed"], "true");
            assert_eq!(report["confirmed"], "2");
            assert_eq!(report["terminal"], "1");
        }
    }
}

#[test]
fn invalid_process_arguments_fail_closed() {
    for args in [
        vec!["--mode", "unknown", "--order", "1,2,3"],
        vec!["--mode", "honest", "--order", "1,1,2"],
        vec!["--mode", "honest", "--order", "1,2"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_v3_adversarial_relay"))
            .args(args)
            .output()
            .expect("adversarial relay process starts");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}
