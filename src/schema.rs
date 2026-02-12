use serde_json::{json, Value};

/// Builder for JSON Schema following the STRAP pattern.
pub struct SchemaBuilder {
    actions: Vec<String>,
    properties: Vec<(String, Value)>,
    required: Vec<String>,
}

impl SchemaBuilder {
    /// Create a new SchemaBuilder with the given action names.
    pub fn new(actions: &[&str]) -> Self {
        Self {
            actions: actions.iter().map(|s| s.to_string()).collect(),
            properties: Vec::new(),
            required: Vec::new(),
        }
    }

    /// Add a string parameter.
    pub fn string(mut self, name: &str, description: &str, required: bool) -> Self {
        self.properties.push((
            name.to_string(),
            json!({"type": "string", "description": description}),
        ));
        if required {
            self.required.push(name.to_string());
        }
        self
    }

    /// Add a number parameter.
    pub fn number(mut self, name: &str, description: &str, required: bool) -> Self {
        self.properties.push((
            name.to_string(),
            json!({"type": "number", "description": description}),
        ));
        if required {
            self.required.push(name.to_string());
        }
        self
    }

    /// Add a boolean parameter.
    pub fn boolean(mut self, name: &str, description: &str, required: bool) -> Self {
        self.properties.push((
            name.to_string(),
            json!({"type": "boolean", "description": description}),
        ));
        if required {
            self.required.push(name.to_string());
        }
        self
    }

    /// Add a string enum parameter.
    pub fn enum_field(mut self, name: &str, description: &str, required: bool, values: &[&str]) -> Self {
        self.properties.push((
            name.to_string(),
            json!({"type": "string", "enum": values, "description": description}),
        ));
        if required {
            self.required.push(name.to_string());
        }
        self
    }

    /// Build the JSON Schema as a serde_json::Value.
    pub fn build(self) -> Value {
        let action_desc = self.actions.join(", ");
        let mut props = serde_json::Map::new();
        props.insert(
            "action".to_string(),
            json!({
                "type": "string",
                "enum": self.actions,
                "description": format!("Action to perform: {}", action_desc),
            }),
        );

        for (name, schema) in self.properties {
            props.insert(name, schema);
        }

        let mut req: Vec<Value> = vec![json!("action")];
        for r in self.required {
            req.push(json!(r));
        }

        json!({
            "type": "object",
            "properties": props,
            "required": req,
        })
    }
}
