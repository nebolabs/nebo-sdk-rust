# Nebo Rust SDK

Official Rust SDK for building [Nebo](https://neboloop.com) apps.

## Install

```toml
[dependencies]
nebo-sdk = "0.1"
```

## Quick Start

```rust
use async_trait::async_trait;
use nebo_sdk::{NeboApp, NeboError, SchemaBuilder};
use nebo_sdk::tool::ToolHandler;
use serde::Deserialize;
use serde_json::Value;

struct Calculator;

#[derive(Deserialize)]
struct Input { action: String, a: f64, b: f64 }

#[async_trait]
impl ToolHandler for Calculator {
    fn name(&self) -> &str { "calculator" }
    fn description(&self) -> &str { "Arithmetic calculator." }
    fn schema(&self) -> Value {
        SchemaBuilder::new(&["add", "subtract", "multiply", "divide"])
            .number("a", "First operand", true)
            .number("b", "Second operand", true)
            .build()
    }
    async fn execute(&self, input: Value) -> Result<String, NeboError> {
        let i: Input = serde_json::from_value(input).map_err(|e| NeboError::Execution(e.to_string()))?;
        let r = match i.action.as_str() {
            "add" => i.a + i.b,
            "subtract" => i.a - i.b,
            "multiply" => i.a * i.b,
            "divide" => i.a / i.b,
            _ => return Err(NeboError::Execution("unknown action".into())),
        };
        Ok(format!("{r}"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    NeboApp::new()?.register_tool(Calculator).run().await?;
    Ok(())
}
```

## Documentation

See [Creating Nebo Apps](https://neboloop.com/developers) for the full guide.
