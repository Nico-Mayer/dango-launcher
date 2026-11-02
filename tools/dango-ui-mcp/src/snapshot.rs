//! Shrinks Slint's element tree to the elements an agent acts on: interactive
//! ones and those with an accessible label or value.

use serde_json::Value;

const INTERACTIVE: &[&str] = &[
    "Button",
    "Checkbox",
    "Combobox",
    "List",
    "ListItem",
    "Slider",
    "Spinbox",
    "Switch",
    "Tab",
    "TabList",
    "TextInput",
    "RadioButton",
];

pub fn render(elements: &[Value], window_height: f64, truncated: bool) -> String {
    let mut lines = Vec::new();
    for element in elements {
        if let Some(line) = line(element, window_height) {
            lines.push(line);
        }
    }
    if truncated {
        lines
            .push("(tree cut at 1000 elements; use query_element_descendants for the rest)".into());
    }
    lines.join("\n")
}

fn line(element: &Value, window_height: f64) -> Option<String> {
    let text = |key: &str| element[key].as_str().filter(|s| !s.is_empty());
    let flag = |key: &str| element[key].as_bool().unwrap_or(false);

    let role = text("accessibleRole").filter(|r| *r != "Unknown");
    let label = text("accessibleLabel");
    let value = text("accessibleValue");
    let interactive = role.is_some_and(|r| INTERACTIVE.contains(&r));
    if label.is_none() && value.is_none() && !interactive {
        return None;
    }
    if element["computedOpacity"].as_f64() == Some(0.0) {
        return None;
    }

    let handle = &element["handle"];
    let mut line = format!(
        "#{}/{} {}",
        handle["index"].as_str().unwrap_or("?"),
        handle["generation"].as_str().unwrap_or("?"),
        role.unwrap_or("-"),
    );
    if let Some(label) = label {
        line.push_str(&format!(" {label:?}"));
    }
    if let Some(value) = value {
        line.push_str(&format!(" value={value:?}"));
    }
    for (key, name) in [
        ("accessibleItemSelected", "selected"),
        ("accessibleChecked", "checked"),
        ("accessibleReadOnly", "read-only"),
    ] {
        if flag(key) {
            line.push(' ');
            line.push_str(name);
        }
    }
    let y = element["absolutePosition"]["y"].as_f64().unwrap_or(0.0);
    let height = element["size"]["height"].as_f64().unwrap_or(0.0);
    if y >= window_height || (height > 0.0 && y + height <= 0.0) {
        line.push_str(" offscreen");
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn keeps_labelled_and_interactive_elements_only() {
        let elements = [
            json!({"handle": {"index": "1", "generation": "1"}, "typeNamesAndIds": []}),
            json!({
                "handle": {"index": "2", "generation": "1"},
                "accessibleRole": "TextInput",
                "accessibleLabel": "Search apps and commands",
                "accessibleValue": "conf",
            }),
            json!({
                "handle": {"index": "23", "generation": "1"},
                "accessibleRole": "ListItem",
                "accessibleLabel": "Clipboard History",
                "accessibleItemSelected": true,
                "absolutePosition": {"y": 90.0},
                "size": {"height": 40.0},
            }),
            json!({
                "handle": {"index": "29", "generation": "1"},
                "accessibleRole": "Image",
            }),
            json!({
                "handle": {"index": "90", "generation": "1"},
                "accessibleRole": "ListItem",
                "accessibleLabel": "Far down",
                "absolutePosition": {"y": 900.0},
                "size": {"height": 40.0},
            }),
        ];
        assert_eq!(
            render(&elements, 480.0, false),
            "#2/1 TextInput \"Search apps and commands\" value=\"conf\"\n\
             #23/1 ListItem \"Clipboard History\" selected\n\
             #90/1 ListItem \"Far down\" offscreen"
        );
    }

    #[test]
    fn hides_transparent_elements() {
        let elements = [json!({
            "handle": {"index": "5", "generation": "1"},
            "accessibleLabel": "Actions",
            "computedOpacity": 0.0,
        })];
        assert_eq!(render(&elements, 480.0, false), "");
    }
}
