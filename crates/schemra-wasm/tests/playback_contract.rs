use schemra_wasm::ScenarioSampler;
use serde_json::{Value, json};

const VISUAL: &str = r#"visual network "Network" {
  node client "Client" {}
  node server "Server" {}
  client -> server
}
"#;

const SCENARIO: &str = r#"scenario requests for visual network {
  token packet { icon: "mail" }
  move packet from client -> server { duration: 100ms }
  wait 25ms
  highlight server 50ms trace
}
"#;

fn project() -> String {
    json!({
        "sources": [
            { "id": "visual", "path": "visual.schemra", "source": VISUAL },
            { "id": "scenario", "path": "scenario.schemra", "source": SCENARIO }
        ]
    })
    .to_string()
}

fn frame(sampler: &ScenarioSampler, time_ms: u32) -> Value {
    serde_json::from_str(&sampler.sample(time_ms)).expect("sample is JSON")
}

#[test]
fn sampler_compiles_once_and_returns_scene_coordinates_for_playback() {
    let sampler = ScenarioSampler::new(&project(), "network", "requests").unwrap();

    assert_eq!(sampler.duration_ms(), 175);
    let moving = frame(&sampler, 50);
    assert_eq!(moving["time_ms"], 50);
    assert_eq!(moving["tokens"].as_array().map(Vec::len), Some(1));
    assert_eq!(moving["tokens"][0]["token"], "packet");
    assert_eq!(moving["tokens"][0]["traveling"], true);
    assert_eq!(moving["active_edges"].as_array().map(Vec::len), Some(1));

    let arrived = frame(&sampler, 100);
    assert!(arrived["tokens"].as_array().unwrap().is_empty());
    assert!(arrived["active_edges"].as_array().unwrap().is_empty());

    let highlighted = frame(&sampler, 150);
    assert_eq!(highlighted["highlights"].as_array().map(Vec::len), Some(1));
    assert_eq!(highlighted["highlights"][0]["node_id"], "server");
    assert!(
        frame(&sampler, sampler.duration_ms())["highlights"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn sampler_rejects_unknown_visuals_scenarios_and_invalid_projects() {
    assert!(ScenarioSampler::new(&project(), "missing", "requests").is_err());
    assert!(ScenarioSampler::new(&project(), "network", "missing").is_err());
    assert!(ScenarioSampler::new("{", "network", "requests").is_err());
}

#[test]
fn repeated_sampling_is_independent_of_seek_order() {
    let sampler = ScenarioSampler::new(&project(), "network", "requests").unwrap();
    let expected = frame(&sampler, 50);

    for time_ms in [125, 0, 100, 25, 175, 50, 75, 50] {
        let _ = frame(&sampler, time_ms);
    }

    assert_eq!(frame(&sampler, 50), expected);
}
