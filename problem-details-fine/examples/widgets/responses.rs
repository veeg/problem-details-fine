//! Renders the responses of a problem set as HTML that looks like the responses of Redoc 3.1.0. The crate
//! documentation includes the output.
//!
//! Redoc shows the response status codes as tabs, and the schema of the selected response below them. A `oneOf`
//! has a switcher: a segmented control, or a dropdown for more than 5 options. The output gives the same
//! interaction without JavaScript. A group of radio inputs holds each selection, and CSS shows the selected tab and
//! schema. The dropdown is a `<details>` element.
//!
//! The markup, the styles and the colors come from the Redoc source (`src/components/Schema/styled.ts`,
//! `ResponseCodeTabs.tsx`, `ResponsesSection.tsx`, `BodyMimeTypeField.tsx`) and the variables of `@redocly/theme`
//! 0.72. With the dark rustdoc themes, the output uses the dark colors of Redoc. These parts of Redoc need
//! JavaScript, and the output does not have them: deep links, "Expand all", the search field of the dropdown, and
//! closing the dropdown after a selection. The output has no operation and no "Responses" heading; it starts with
//! the tabs.
//!
//! The renderer reads only the parts of a schema that `problem-details-fine` writes: `oneOf`, `allOf` of an object
//! and a `oneOf`, objects with `properties` and `required`, `enum`, `format`, `items`, `minimum` and `examples`.
//! The output has no blank lines, because a blank line ends an HTML block in Markdown.

use serde_json::{Map, Value};

/// The media type of a problem details document.
const PROBLEM_JSON: &str = "application/problem+json";

/// A `oneOf` with more options than this has a dropdown instead of a segmented control, as in Redoc.
const MAX_SEGMENTS: usize = 5;

/// The styles that do not depend on the number of tabs or options. Each value comes from Redoc 3.1.0.
const STYLE: &str = r#"
.rd {
  --rd-text-primary: #1a1c21; --rd-text-secondary: #3b3c45; --rd-text-description: #555761;
  --rd-border-primary: #dcdde5; --rd-border-secondary: #ededf2; --rd-bg: #ffffff; --rd-bg-inner: #ffffff;
  --rd-bg-hover: #f3f3f6; --rd-tag-basic: #2a2b33; --rd-required: #e20c0c;
  --rd-font: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell',
    'Fira Sans', 'Droid Sans', 'Helvetica Neue', Arial, sans-serif;
  --rd-mono: ui-monospace, Menlo, Monaco, 'Cascadia Mono', 'Segoe UI Mono', 'Roboto Mono', 'Oxygen Mono',
    'Ubuntu Mono', 'Fira Mono', 'Droid Sans Mono', 'Courier New', monospace;
  font-family: var(--rd-font); font-size: 14px; line-height: 22px; color: var(--rd-text-primary);
  background: var(--rd-bg); border-radius: 8px; padding: 24px; margin: 16px 0;
}
/* rustdoc gives each child of the documentation `overflow-x: auto`, which would clip the dropdown menu. */
.docblock > div.rd.rd { overflow: visible; }
:root[data-theme="dark"] .rd, :root[data-theme="ayu"] .rd {
  --rd-text-primary: #fbfbfc; --rd-text-secondary: #c4c6d1; --rd-text-description: #9b9ca8;
  --rd-border-primary: #555761; --rd-border-secondary: #3b3c45; --rd-bg: #22242b; --rd-bg-inner: #2a2b33;
  --rd-bg-hover: #2a2b33; --rd-tag-basic: #dcdde5; --rd-required: #f56c6c;
}
.rd .rd-radio { position: absolute; opacity: 0; width: 1px; height: 1px; margin: 0; pointer-events: none; }
.rd .rd-tabs { display: flex; align-items: center; flex-wrap: wrap; margin: 0 0 16px; padding: 0; }
.rd .rd-tab { display: inline-flex; align-items: center; padding: 0 12px; height: 24px; border-radius: 6px;
  cursor: pointer; user-select: none; font-size: 14px; line-height: 22px; font-weight: 400;
  color: var(--rd-text-secondary); background: transparent; }
.rd .rd-tab:hover { color: var(--rd-text-primary); }
.rd .rd-tab::before { content: ""; display: inline-block; width: 6px; height: 6px; border-radius: 50%;
  margin-right: 4px; border: 1px solid var(--rd-bg); background-color: #1f7cff; }
.rd .rd-tab.rd-2xx::before { background-color: #1cb854; }
.rd .rd-tab.rd-3xx::before { background-color: #ffa02e; }
.rd .rd-tab.rd-4xx::before, .rd .rd-tab.rd-5xx::before { background-color: #f9316d; }
.rd .rd-panel, .rd .rd-option, .rd .rd-dd-label { display: none; }
.rd .rd-panel { flex-direction: column; gap: 8px; }
.rd .rd-content { display: flex; flex-direction: column; }
.rd .rd-md { margin: 0; overflow-wrap: break-word; color: var(--rd-text-secondary); font-size: 14px;
  line-height: 22px; }
.rd .rd-md p { margin: 14px 0 0; }
/* `:root` outranks the inline code styles of rustdoc, including the color of the ayu theme. */
:root .rd .rd-md code { display: inline; white-space: normal; color: var(--rd-text-secondary); background-color: var(--rd-border-secondary);
  border-radius: 4px; padding: 0.19rem 4px; font-weight: 400; word-break: break-word; font-size: 12px;
  font-family: var(--rd-mono); line-height: 20px; }
.rd .rd-response-description .rd-md p:first-child { margin-top: 0; }
.rd .rd-body-title { position: relative; display: flex; align-items: center; font-weight: 600;
  color: var(--rd-text-primary); margin-top: 8px; font-size: 18px; line-height: 24px; padding: 0 0 4px; }
.rd .rd-media { margin: 0 4px; cursor: default; color: var(--rd-text-primary); font-weight: 400; font-size: 16px;
  line-height: 24px; }
.rd .rd-props { display: flex; flex-direction: column; }
.rd .rd-prop { width: 100%; padding: 16px 0; border-bottom: 1px solid var(--rd-border-primary); }
.rd .rd-prop:first-child { padding-top: 12px; }
.rd .rd-prop p { margin-bottom: 0; }
.rd .rd-row { display: flex; align-items: center; gap: 1px 4px; flex-wrap: wrap; margin-bottom: 2px;
  position: relative; }
.rd .rd-name { position: relative; font-family: var(--rd-mono); font-size: 14px; line-height: 22px;
  color: var(--rd-text-description); white-space: pre-wrap; overflow-wrap: break-word; max-width: 100%; }
.rd .rd-pname { font-family: var(--rd-mono); font-weight: 600; font-size: 14px; line-height: 22px;
  color: var(--rd-text-primary); }
.rd .rd-type { font-style: normal; color: var(--rd-text-description); font-size: 14px; line-height: 22px;
  vertical-align: middle; overflow-wrap: anywhere; }
.rd .rd-type:not(:last-of-type)::after { content: ','; }
.rd .rd-required { color: var(--rd-required); font-size: 14px; line-height: 22px; vertical-align: middle; }
.rd .rd-detail { display: flex; flex-wrap: wrap; align-items: center; gap: 4px; margin-top: 4px; }
.rd .rd-detail-label { font-size: 14px; line-height: 22px; color: var(--rd-text-secondary); }
.rd .rd-tag { padding: 0 4px; font-family: var(--rd-mono); font-size: 12px; line-height: 20px; border-radius: 6px;
  color: var(--rd-text-secondary); background: var(--rd-border-secondary); display: inline-flex;
  word-break: break-word; }
.rd .rd-example { border: 1px solid var(--rd-border-primary); white-space: pre-wrap; overflow-wrap: anywhere;
  width: fit-content; }
.rd .rd-oneof { display: flex; flex-direction: column; gap: 8px; }
.rd .rd-option { flex-direction: column; gap: 8px; }
.rd .rd-switcher { display: flex; flex-direction: column; align-items: flex-start; }
.rd .rd-badge { font-size: 12px; line-height: 20px; color: var(--rd-tag-basic); padding: 0 8px;
  border: 1px solid var(--rd-border-secondary); border-radius: 6px; margin: 4px 0 8px; position: relative;
  width: fit-content; }
.rd .rd-badge::before { content: ' '; display: block; position: absolute; width: 1px; height: 8px;
  background: var(--rd-border-secondary); bottom: -8px; left: 8px; }
.rd .rd-segmented { display: flex; background: var(--rd-border-secondary); padding: 2px; gap: 2px;
  border-radius: 6px; max-width: 100%; width: fit-content; }
.rd .rd-segment { font-size: 12px; line-height: 20px; border-radius: 4px; padding: 0 12px; white-space: nowrap;
  text-overflow: ellipsis; overflow: hidden; font-weight: 400; color: var(--rd-text-secondary);
  cursor: pointer; user-select: none; }
.rd .rd-dd { position: relative; }
.rd .rd-dd-trigger { display: inline-flex; align-items: center; gap: 4px; list-style: none; cursor: pointer;
  user-select: none; border: 1px solid var(--rd-border-primary); border-radius: 6px; padding: 1px 12px;
  font-size: 14px; line-height: 22px; color: var(--rd-text-primary); background: var(--rd-bg); }
.rd .rd-dd-trigger::-webkit-details-marker { display: none; }
.rd .rd-dd-trigger::after { content: ''; width: 6px; height: 6px; margin: 0 2px 3px 4px;
  border-right: 1.5px solid currentColor; border-bottom: 1.5px solid currentColor; transform: rotate(45deg); }
.rd .rd-dd[open] .rd-dd-trigger::after { margin-bottom: -3px; transform: rotate(-135deg); }
.rd .rd-dd-menu { position: absolute; z-index: 10; top: calc(100% + 4px); left: 0; margin: 0; min-width: 100px;
  max-height: 300px; padding: 4px; list-style: none; white-space: nowrap; overflow-x: hidden; overflow-y: auto;
  display: flex; flex-direction: column; background: var(--rd-bg); color: var(--rd-text-primary);
  border: 1px solid var(--rd-border-secondary); border-radius: 12px;
  box-shadow: 0 8px 24px 8px #0000000a, 0 4px 12px 0 #00000014; font-size: 14px; font-weight: 400;
  line-height: 22px; }
.rd .rd-dd-menu li { margin: 0; }
.rd .rd-dd-item { display: flex; align-items: center; justify-content: space-between; gap: 20px; width: 100%;
  box-sizing: border-box; padding: 4px 12px; border-radius: 8px; cursor: pointer; user-select: none; }
.rd .rd-dd-item:hover { background-color: var(--rd-bg-hover); }
.rd .rd-dd-item::after { content: ''; width: 4px; height: 9px; margin: 0 5px 3px auto; flex-shrink: 0;
  border-right: 1.5px solid transparent; border-bottom: 1.5px solid transparent; transform: rotate(45deg); }
"#;

/// Returns the HTML for `responses`, the responses of a problem set by status code.
pub fn render(responses: &Map<String, Value>) -> String {
    let mut renderer = Renderer::default();
    let mut body = String::new();
    renderer.responses(&mut body, responses);
    format!(
        "<div class=\"rd\"><style>{}{}</style>\n{body}</div>\n",
        STYLE.trim_start().replace("\n\n", "\n"),
        renderer.selection_style()
    )
}

/// The state of the renderer: the next identifier of a radio group, and the largest groups that the CSS must
/// cover.
#[derive(Default)]
struct Renderer {
    groups: usize,
    max_tabs: usize,
    max_options: usize,
}

/// A schema after the `allOf` of an object and a `oneOf` is merged, as Redoc merges it: the properties of the
/// object, the options of the `oneOf` and the description of either.
struct Node<'a> {
    properties: Vec<(&'a str, &'a Value)>,
    required: Vec<&'a str>,
    one_of: &'a [Value],
    description: Option<&'a str>,
}

impl<'a> Node<'a> {
    fn new(schema: &'a Value) -> Self {
        let (base, rest) = match schema["allOf"].as_array() {
            Some(parts) => (&parts[0], parts.get(1)),
            None => (schema, None),
        };
        let one_of = rest.unwrap_or(schema)["oneOf"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        Node {
            properties: properties(base),
            required: required(base),
            one_of,
            description: rest
                .and_then(|rest| rest["description"].as_str())
                .or_else(|| base["description"].as_str()),
        }
    }

    /// Adds the properties of a `oneOf` option to the properties of this node. As in Redoc, the properties of this
    /// node come first, and a property of the option replaces the property with the same name.
    fn with_option(&self, option: &'a Value) -> Node<'a> {
        let child = Node::new(option);
        let mut properties = self.properties.clone();
        for (name, property) in child.properties {
            match properties
                .iter_mut()
                .find(|(existing, _)| *existing == name)
            {
                Some(existing) => existing.1 = property,
                None => properties.push((name, property)),
            }
        }
        let mut required = self.required.clone();
        required.extend(
            child
                .required
                .iter()
                .filter(|name| !self.required.contains(name)),
        );
        Node {
            properties,
            required,
            ..child
        }
    }
}

fn properties(schema: &Value) -> Vec<(&str, &Value)> {
    schema["properties"]
        .as_object()
        .map(Map::iter)
        .into_iter()
        .flatten()
        .map(|(name, property)| (name.as_str(), property))
        .collect()
}

fn required(schema: &Value) -> Vec<&str> {
    schema["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

impl Renderer {
    fn group(&mut self) -> String {
        self.groups += 1;
        format!("rd-g{}", self.groups)
    }

    /// Renders a tab for each response, and the schema of each response.
    fn responses(&mut self, out: &mut String, responses: &Map<String, Value>) {
        self.max_tabs = responses.len();
        let group = self.group();
        out.push_str("<div class=\"rd-responses\">\n");
        radios(out, &group, responses.len());
        out.push_str("<div class=\"rd-tabs\" role=\"tablist\" aria-label=\"Response status\">");
        for (index, status) in responses.keys().enumerate() {
            out.push_str(&format!(
                "<label class=\"rd-tab rd-{}xx\" for=\"{group}-{index}\" role=\"tab\">{}</label>",
                &status[..1],
                escape(status)
            ));
        }
        out.push_str("</div>\n<div class=\"rd-panels\">\n");
        for response in responses.values() {
            out.push_str("<div class=\"rd-panel\"><div class=\"rd-content\">");
            if let Some(description) = response["description"].as_str() {
                out.push_str(&format!(
                    "<div class=\"rd-response-description\">{}</div>",
                    markdown(description)
                ));
            }
            out.push_str(&format!(
                "<div class=\"rd-body-title\">Body<span class=\"rd-media\">{PROBLEM_JSON}</span></div>\n"
            ));
            self.node(
                out,
                &Node::new(&response["content"][PROBLEM_JSON]["schema"]),
            );
            out.push_str("</div></div>\n");
        }
        out.push_str("</div></div>\n");
    }

    /// Renders the switcher of a `oneOf` node, or else the properties of the node.
    fn node(&mut self, out: &mut String, node: &Node) {
        if node.one_of.is_empty() {
            properties_list(out, node);
            return;
        }
        self.max_options = self.max_options.max(node.one_of.len());
        let group = self.group();
        let labels: Vec<String> = node.one_of.iter().map(label).collect();
        out.push_str("<div class=\"rd-oneof\">\n");
        radios(out, &group, labels.len());
        out.push_str("<div class=\"rd-switcher\"><div class=\"rd-badge\">One of:</div>");
        if labels.len() > MAX_SEGMENTS {
            out.push_str("<details class=\"rd-dd\"><summary class=\"rd-dd-trigger\">");
            for label in &labels {
                out.push_str(&format!(
                    "<span class=\"rd-dd-label\">{}</span>",
                    escape(label)
                ));
            }
            out.push_str("</summary><ul class=\"rd-dd-menu\" role=\"menu\">");
            for (index, label) in labels.iter().enumerate() {
                out.push_str(&format!(
                    "<li><label class=\"rd-dd-item\" for=\"{group}-{index}\" role=\"menuitem\">{}</label></li>",
                    escape(label)
                ));
            }
            out.push_str("</ul></details>");
        } else {
            out.push_str("<div class=\"rd-segmented\" role=\"tablist\">");
            for (index, label) in labels.iter().enumerate() {
                out.push_str(&format!(
                    "<label class=\"rd-segment\" for=\"{group}-{index}\" role=\"tab\" title=\"{0}\">{0}</label>",
                    escape(label)
                ));
            }
            out.push_str("</div>");
        }
        out.push_str("</div>\n<div class=\"rd-options\">\n");
        for option in node.one_of {
            let child = node.with_option(option);
            out.push_str("<div class=\"rd-option\">");
            if let Some(description) = child.description.or(node.description) {
                out.push_str(&format!(
                    "<div class=\"rd-description\">{}</div>",
                    markdown(description)
                ));
            }
            self.node(out, &child);
            out.push_str("</div>\n");
        }
        out.push_str("</div></div>\n");
    }

    /// The CSS that shows the selected tab and option. A radio group has one input for each tab or option, before
    /// the tabs and the panels. The input at position `n` selects the tab, the label and the panel at position `n`.
    fn selection_style(&self) -> String {
        let mut out = String::new();
        for n in 1..=self.max_tabs {
            let checked = format!(".rd .rd-responses > .rd-radio:nth-of-type({n}):checked");
            out.push_str(&format!(
                "{checked} ~ .rd-tabs .rd-tab:nth-child({n}) {{ font-weight: 700; \
                 color: var(--rd-text-primary); background-color: var(--rd-border-secondary); }}\n\
                 {checked} ~ .rd-panels > .rd-panel:nth-child({n}) {{ display: flex; }}\n\
                 .rd .rd-responses > .rd-radio:nth-of-type({n}):focus-visible ~ .rd-tabs .rd-tab:nth-child({n}) \
                 {{ outline: 2px solid #1f7cff; }}\n"
            ));
        }
        for n in 1..=self.max_options {
            let checked = format!(".rd .rd-oneof > .rd-radio:nth-of-type({n}):checked");
            out.push_str(&format!(
                "{checked} ~ .rd-switcher .rd-segment:nth-child({n}) {{ font-weight: 500; \
                 background: var(--rd-bg-inner); color: var(--rd-text-primary); cursor: default; }}\n\
                 {checked} ~ .rd-switcher .rd-dd-label:nth-child({n}) {{ display: inline; }}\n\
                 {checked} ~ .rd-switcher li:nth-child({n}) .rd-dd-item {{ background-color: var(--rd-bg-hover); }}\n\
                 {checked} ~ .rd-switcher li:nth-child({n}) .rd-dd-item::after {{ border-color: currentColor; }}\n\
                 {checked} ~ .rd-options > .rd-option:nth-child({n}) {{ display: flex; }}\n\
                 .rd .rd-oneof > .rd-radio:nth-of-type({n}):focus-visible ~ .rd-switcher \
                 :is(.rd-segment:nth-child({n}), li:nth-child({n}) .rd-dd-item) {{ outline: 2px solid #1f7cff; }}\n"
            ));
        }
        out
    }
}

/// Writes a group of radio inputs. The first input is checked, so the first tab or option is selected, as in Redoc.
fn radios(out: &mut String, group: &str, count: usize) {
    for index in 0..count {
        let checked = if index == 0 { " checked" } else { "" };
        out.push_str(&format!(
            "<input type=\"radio\" class=\"rd-radio\" name=\"{group}\" id=\"{group}-{index}\"{checked}>"
        ));
    }
    out.push('\n');
}

/// Returns the label of a `oneOf` option, as Redoc gives it: the `title` of the option or of its `allOf`, or else
/// its type.
fn label(option: &Value) -> String {
    option["title"]
        .as_str()
        .or_else(|| option["allOf"][0]["title"].as_str())
        .or_else(|| option["type"].as_str())
        .unwrap_or("object")
        .to_owned()
}

/// Renders the properties of a node, as the rows of Redoc: the name, the type and "required", followed by the
/// description, the allowed values and the first example.
fn properties_list(out: &mut String, node: &Node) {
    out.push_str("<div class=\"rd-props\">\n");
    for (name, property) in &node.properties {
        out.push_str(&format!(
            "<div class=\"rd-prop\"><div class=\"rd-row\"><span class=\"rd-name\"><span class=\"rd-pname\">{}</span>\
             </span><em class=\"rd-type\">{}</em>",
            escape(name),
            escape(&type_of(property))
        ));
        if node.required.contains(name) {
            out.push_str("<span class=\"rd-required\">required</span>");
        }
        out.push_str("</div>");
        if let Some(description) = property["description"].as_str() {
            out.push_str(&format!(
                "<div class=\"rd-description\">{}</div>",
                markdown(description)
            ));
        }
        if let Some(values) = property["enum"].as_array() {
            out.push_str(&format!(
                "<div class=\"rd-detail\"><span class=\"rd-detail-label\">{}</span>",
                if values.len() == 1 { "Value:" } else { "Enum:" }
            ));
            for value in values {
                out.push_str(&format!(
                    "<span class=\"rd-tag\">{}</span>",
                    escape(&value.to_string())
                ));
            }
            out.push_str("</div>");
        }
        if let Some(example) = property["examples"].get(0) {
            out.push_str(&format!(
                "<div class=\"rd-detail\"><span class=\"rd-detail-label\">Example:</span>\
                 <span class=\"rd-tag rd-example\">{}</span></div>",
                escape(&example.to_string())
            ));
        }
        out.push_str("</div>\n");
    }
    out.push_str("</div>\n");
}

/// Returns the type of a property as Redoc writes it, for example `string, (uri-reference)`, `string or null`,
/// `integer, >= 1` or `Array of strings`.
fn type_of(property: &Value) -> String {
    let names: Vec<&str> = match &property["type"] {
        Value::String(name) => vec![name],
        Value::Array(names) => names.iter().filter_map(Value::as_str).collect(),
        _ => vec!["object"],
    };
    let mut parts = vec![
        names
            .iter()
            .map(|name| match *name {
                "array" => format!("Array of {}s", type_of(&property["items"])),
                name => name.to_owned(),
            })
            .collect::<Vec<_>>()
            .join(" or "),
    ];
    if let Some(format) = property["format"].as_str() {
        parts.push(format!("({format})"));
    }
    if let Some(minimum) = property["minimum"].as_number() {
        parts.push(format!(">= {minimum}"));
    }
    parts.join(", ")
}

/// Renders a description as Redoc renders Markdown: a paragraph, with the text between backticks as inline code.
/// A newline becomes a space, because a blank line ends the HTML block.
fn markdown(prose: &str) -> String {
    let html: String = escape(&prose.replace('\n', " "))
        .split('`')
        .enumerate()
        .map(|(i, part)| {
            if i % 2 == 1 {
                format!("<code>{part}</code>")
            } else {
                part.to_owned()
            }
        })
        .collect();
    format!("<div class=\"rd-md\"><p>{html}</p></div>")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
