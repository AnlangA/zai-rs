//! Public Agent v1 request serialization not covered by module unit tests.

use zai_rs::agent::{
    AgentCustomVariables, AgentId, AgentInvokeRequest, AgentMessage, NonStreaming,
};

#[test]
fn custom_variables_is_open_map() {
    let mut variables = AgentCustomVariables::new();
    variables.insert("k", serde_json::json!(42));
    let request = AgentInvokeRequest::<NonStreaming>::builder(AgentId::GeneralTranslation)
        .message(AgentMessage::user("hi"))
        .custom_variables(variables)
        .build()
        .unwrap();
    let json = serde_json::to_value(request).unwrap();
    assert_eq!(json["custom_variables"]["k"], 42);
}
