use async_trait::async_trait;
use nebo_sdk::error::NeboError;
use nebo_sdk::schema::SchemaBuilder;
use nebo_sdk::tool::ToolHandler;
use nebo_sdk::NeboApp;
use serde::Deserialize;
use serde_json::Value;

struct Calculator;

#[derive(Deserialize)]
struct CalcInput {
    action: String,
    a: f64,
    b: f64,
}

#[async_trait]
impl ToolHandler for Calculator {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Performs arithmetic calculations."
    }

    fn schema(&self) -> Value {
        SchemaBuilder::new(&["add", "subtract", "multiply", "divide"])
            .number("a", "First operand", true)
            .number("b", "Second operand", true)
            .build()
    }

    async fn execute(&self, input: Value) -> Result<String, NeboError> {
        let inp: CalcInput =
            serde_json::from_value(input).map_err(|e| NeboError::Execution(e.to_string()))?;

        let result = match inp.action.as_str() {
            "add" => inp.a + inp.b,
            "subtract" => inp.a - inp.b,
            "multiply" => inp.a * inp.b,
            "divide" => {
                if inp.b == 0.0 {
                    return Err(NeboError::Execution("division by zero".into()));
                }
                inp.a / inp.b
            }
            other => return Err(NeboError::Execution(format!("unknown action: {other}"))),
        };

        Ok(format!("{} {} {} = {}", inp.a, inp.action, inp.b, result))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = NeboApp::new()?.register_tool(Calculator);
    app.run().await?;
    Ok(())
}
