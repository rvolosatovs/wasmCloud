#![cfg(target_arch = "wasm32")]

wit_bindgen::generate!({
    world: "wasmcloud",
    path: "../../../wit",
});

struct Actor;

impl actor::Actor for Actor {
    fn guest_call(operation: String, payload: Option<Vec<u8>>) -> Result<Option<Vec<u8>>, String> {
        host::host_call("test", "test", "test", None)?;
        Ok(None)
    }
}

export_wasmcloud!(Actor);
