use std::{error::Error, io::Read};

use schemra_wasm::{
    ScenarioSampler, compile_project_schemra, compile_schemra, complete_schemra,
    complete_schemra_with_nodes, format_schemra, rename_project_schemra,
};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn Error>> {
    let mut source = String::new();
    std::io::stdin().read_to_string(&mut source)?;
    if std::env::args().any(|argument| argument == "--compile-only") {
        println!("{}", compile_schemra(&source));
        return Ok(());
    }

    let project = json!({
        "sources": [{
            "id": "main.schemra",
            "path": "main.schemra",
            "source": source
        }]
    });
    let project_json = serde_json::to_string(&project)?;
    let oversized_source = " ".repeat(2 * 1024 * 1024 + 1);
    let completion_source = "visual network {\n client -> ";
    let completion_context = json!({"node_ids": ["client", "server"]});
    let rename_offset = source.find("client").ok_or("fixture has no client node")?;
    let rename_request = json!({
        "sources": [{
            "id": "main.schemra",
            "path": "main.schemra",
            "source": source
        }],
        "path": "main.schemra",
        "offset": rename_offset
    });
    let rename_apply_request = json!({
        "sources": [{
            "id": "main.schemra",
            "path": "main.schemra",
            "source": source
        }],
        "path": "main.schemra",
        "offset": rename_offset,
        "new_name": "frontend"
    });
    let sampler = ScenarioSampler::new(&project_json, "network", "request")?;

    let outputs = json!({
        "single": parse_response(compile_schemra(&source))?,
        "project": parse_response(compile_project_schemra(&project_json))?,
        "invalid": parse_response(compile_schemra("visual network { node client {"))?,
        "oversized": parse_response(compile_schemra(&oversized_source))?,
        "invalid_project": parse_response(compile_project_schemra("{"))?,
        "formatted": format_schemra(&source),
        "completion": parse_response(complete_schemra(completion_source, completion_source.len(), true))?,
        "completion_with_nodes": parse_response(complete_schemra_with_nodes(
            completion_source,
            completion_source.len(),
            true,
            &completion_context.to_string()
        ))?,
        "rename_prepared": parse_response(rename_project_schemra(&rename_request.to_string()))?,
        "rename_applied": parse_response(rename_project_schemra(&rename_apply_request.to_string()))?,
        "playback_duration_ms": sampler.duration_ms(),
        "playback_mid": parse_response(sampler.sample(50))?,
        "playback_end": parse_response(sampler.sample(sampler.duration_ms()))?
    });
    println!("{}", serde_json::to_string(&outputs)?);
    Ok(())
}

fn parse_response(response: String) -> Result<Value, serde_json::Error> {
    serde_json::from_str(&response)
}
