//! Local JSON-lines companion protocol. No network listener or shell endpoint.
use crate::{detection, model::Result, service::Service, ssh, storage};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub(crate) fn synchronize(service: &mut Service) -> Result<()> {
    let current = storage::load(&service.root)?;
    if serde_json::to_value(&current).ok() != serde_json::to_value(&service.data).ok() {
        service.data = current;
        service.plans.clear();
    }
    Ok(())
}
fn field<'a>(request: &'a Value, name: &str) -> Result<&'a str> {
    request.get(name).and_then(Value::as_str).ok_or_else(|| format!("Missing {name}"))
}
fn dispatch(service: &mut Service, request: &Value) -> Result<Value> {
    let _lock = storage::ConfigLock::acquire(&service.root.join("state.json"))?;
    synchronize(service)?;
    let value = match field(request, "method")? {
        "snapshot" => json!(service.data),
        "detect" => json!(detection::detect()),
        "plan" => json!(service.plan_activation(field(request, "profileId")?)?),
        "apply" => json!(service.apply(field(request, "planId")?)?),
        "verify" => json!(ssh::verify(service.profile(field(request, "profileId")?)?)?),
        "create" => json!(service.create_profile(serde_json::from_value(request.get("profile").cloned().ok_or("Missing profile")?).map_err(|_| "Invalid profile")?)?),
        "rename" => json!(service.rename_profile(field(request, "profileId")?, field(request, "name")?)?),
        "remove" => json!(service.remove_profile(field(request, "profileId")?)?),
        "history" => json!(service.journals()?),
        "undo" => json!(service.undo(field(request, "transactionId")?)?),
        _ => return Err("Unsupported operation".into()),
    };
    Ok(value)
}
pub fn run() -> Result<()> {
    let root = dirs::data_dir().ok_or("Application data directory unavailable")?.join("dev.gitcontext.desktop");
    storage::private_dir(&root)?;
    let mut service = {
        let _lock = storage::ConfigLock::acquire(&root.join("state.json"))?;
        Service::new(root)?
    };
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        let mut bytes = Vec::new();
        let count = std::io::Read::take(&mut input, 1_048_577).read_until(b'\n', &mut bytes).map_err(|_| "Protocol read failed")?;
        if count == 0 { break; }
        if count > 1_048_576 { return Err("Request too large".into()); }
        let response = match serde_json::from_slice::<Value>(&bytes) {
            Ok(request) => {
                let id = request.get("id").cloned().unwrap_or(Value::Null);
                match dispatch(&mut service, &request) {
                    Ok(result) => json!({"id":id,"result":result}),
                    Err(error) => json!({"id":id,"error":error}),
                }
            }
            Err(_) => json!({"id":null,"error":"Invalid JSON request"}),
        };
        serde_json::to_writer(&mut output, &response).map_err(|_| "Protocol write failed")?;
        writeln!(output).and_then(|_| output.flush()).map_err(|_| "Protocol write failed")?;
    }
    Ok(())
}
