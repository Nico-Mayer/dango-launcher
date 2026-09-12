//! One place that decides what a placeholder is.
//!
//! A template is plain text with `{{ name }}` holes in it. A fixed vocabulary
//! fills itself in; every other name is an argument the user is asked for. That
//! one rule is the whole design, and it works because the engine already
//! computes the set of names a template references.
//!
//! Single braces are not significant, so a snippet of code needs no escaping.
//! Doubled braces are, which is why the create form shows what a template will
//! ask for: text that became a placeholder by accident is visible while it is
//! still being edited.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use minijinja::{Environment, Value};

/// Names that resolve without asking the user. `query` is reserved too, but it
/// is supplied by whoever is rendering rather than resolved here.
pub const RESERVED: &[&str] = &["clipboard", "selection", "date", "uuid", "cursor", "query"];

/// Where the caret should end up, carried through rendering as a character no
/// user would type. Stripped before the text is ever seen.
const CARET_SENTINEL: char = '\u{E000}';

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TemplateError {
    #[error("{0}")]
    Parse(String),
    #[error("{0}")]
    Render(String),
}

/// The values a render needs. Anything absent renders as empty text rather than
/// failing, because a snippet using the clipboard is still worth pasting when
/// the clipboard is empty.
#[derive(Debug, Default, Clone)]
pub struct Values {
    pub clipboard: Option<String>,
    pub selection: Option<String>,
    pub query: Option<String>,
    pub arguments: HashMap<String, String>,
}

impl Values {
    pub fn with_arguments(arguments: HashMap<String, String>) -> Self {
        Self {
            arguments,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub text: String,
    /// Characters from the start of `text`, not bytes: the caret is moved with
    /// arrow keys, which count neither bytes nor graphemes.
    pub caret: Option<usize>,
}

/// A parsed template. Parsing is done once, at save time, so a template that
/// cannot be read is refused where it is written rather than when it is used.
#[derive(Debug, Clone)]
pub struct Template {
    source: String,
    referenced: Vec<String>,
    arguments: Vec<String>,
}

impl Template {
    pub fn parse(source: &str) -> Result<Self, TemplateError> {
        let template = environment()
            .template_from_str(source)
            .map_err(|error| TemplateError::Parse(error.to_string()))?;

        let referenced = order_by_first_use(source, template.undeclared_variables(false));
        let arguments = referenced
            .iter()
            .filter(|name| !is_reserved(name))
            .cloned()
            .collect();

        Ok(Self {
            source: source.to_string(),
            referenced,
            arguments,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Names with no source but the user, in the order the template uses them.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    /// Everything the user has to be asked for. That is the arguments plus
    /// `query`, which is reserved so that a quicklink can rely on its meaning,
    /// but which nothing can resolve on the user's behalf.
    pub fn prompts(&self) -> Vec<String> {
        self.referenced
            .iter()
            .filter(|name| name.as_str() == "query" || !is_reserved(name))
            .cloned()
            .collect()
    }

    pub fn uses(&self, name: &str) -> bool {
        self.referenced.iter().any(|used| used == name)
    }

    pub fn render(&self, values: &Values) -> Result<Rendered, TemplateError> {
        self.render_with(values, |value| value)
    }

    /// Renders for a URL, encoding every value put into it so that a query
    /// containing `&` or `?` cannot change the URL's structure. Only the values
    /// are encoded; the template's own punctuation is left alone.
    pub fn render_url(&self, values: &Values) -> Result<Rendered, TemplateError> {
        self.render_with(values, |value| {
            minijinja::filters::urlencode(&Value::from(value.as_str())).unwrap_or(value)
        })
    }

    fn render_with(
        &self,
        values: &Values,
        encode: impl Fn(String) -> String,
    ) -> Result<Rendered, TemplateError> {
        let mut context: BTreeMap<&str, Value> = BTreeMap::new();
        for name in &self.referenced {
            let resolved = match name.as_str() {
                "clipboard" => encode(values.clipboard.clone().unwrap_or_default()),
                "selection" => encode(values.selection.clone().unwrap_or_default()),
                "query" => encode(values.query.clone().unwrap_or_default()),
                "date" => today(),
                "uuid" => uuid::Uuid::new_v4().to_string(),
                "cursor" => CARET_SENTINEL.to_string(),
                other => encode(values.arguments.get(other).cloned().unwrap_or_default()),
            };
            context.insert(name.as_str(), Value::from(resolved));
        }

        let template = environment()
            .template_from_str(&self.source)
            .map_err(|error| TemplateError::Parse(error.to_string()))?;
        let rendered = template
            .render(Value::from(context))
            .map_err(|error| TemplateError::Render(error.to_string()))?;

        Ok(split_caret(rendered))
    }
}

pub fn is_reserved(name: &str) -> bool {
    RESERVED.contains(&name)
}

fn environment() -> &'static Environment<'static> {
    static ENVIRONMENT: OnceLock<Environment<'static>> = OnceLock::new();
    // No loader is installed, so `include` and `extends` have nothing to reach.
    ENVIRONMENT.get_or_init(Environment::new)
}

fn today() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let format = time::macros::format_description!("[year]-[month]-[day]");
    now.format(&format).unwrap_or_default()
}

/// The engine reports the names it sees as an unordered set, and a form whose
/// fields shuffle between openings is unusable. Ordering is recovered by
/// walking the `{{ ... }}` spans in the source, so a name that merely appears
/// in prose does not decide the order.
fn order_by_first_use(source: &str, names: std::collections::HashSet<String>) -> Vec<String> {
    let mut ordered: Vec<String> = Vec::with_capacity(names.len());
    let mut rest: Vec<String> = names.iter().cloned().collect();
    rest.sort();

    let bytes = source.as_bytes();
    let mut index = 0;
    while let Some(start) = find_from(bytes, b"{{", index) {
        let Some(end) = find_from(bytes, b"}}", start + 2) else {
            break;
        };
        let span = &source[start + 2..end];
        for name in &rest {
            if !ordered.contains(name) && mentions(span, name) {
                ordered.push(name.clone());
            }
        }
        index = end + 2;
    }

    // A name used only inside a block rather than an expression still has to
    // appear, and deterministically.
    for name in rest {
        if !ordered.contains(&name) {
            ordered.push(name);
        }
    }
    ordered
}

fn find_from(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

/// Matches the name as a whole identifier, so `user` is not found inside
/// `username`.
fn mentions(span: &str, name: &str) -> bool {
    let mut rest = span;
    while let Some(offset) = rest.find(name) {
        let before = rest[..offset].chars().next_back();
        let after = rest[offset + name.len()..].chars().next();
        let boundary = |c: Option<char>| !matches!(c, Some(c) if c.is_alphanumeric() || c == '_');
        if boundary(before) && boundary(after) {
            return true;
        }
        rest = &rest[offset + name.len()..];
    }
    false
}

fn split_caret(rendered: String) -> Rendered {
    if !rendered.contains(CARET_SENTINEL) {
        return Rendered {
            text: rendered,
            caret: None,
        };
    }
    let caret = rendered
        .chars()
        .take_while(|c| *c != CARET_SENTINEL)
        .count();
    Rendered {
        text: rendered.replace(CARET_SENTINEL, ""),
        caret: Some(caret),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(source: &str, values: &Values) -> Rendered {
        Template::parse(source).unwrap().render(values).unwrap()
    }

    fn arguments(source: &str) -> Vec<String> {
        Template::parse(source).unwrap().arguments().to_vec()
    }

    #[test]
    fn a_template_without_placeholders_renders_back_to_itself() {
        let source = "Nothing to see here.";
        assert_eq!(render(source, &Values::default()).text, source);
    }

    #[test]
    fn single_braces_are_not_placeholders() {
        let source = "fn main() { let x = Foo { a: 1 }; }";
        assert_eq!(arguments(source), Vec::<String>::new());
        assert_eq!(render(source, &Values::default()).text, source);
    }

    #[test]
    fn a_placeholder_is_replaced() {
        let values = Values::with_arguments(HashMap::from([("name".into(), "Nico".into())]));
        assert_eq!(render("Hi {{ name }}!", &values).text, "Hi Nico!");
    }

    #[test]
    fn the_clipboard_and_selection_resolve_without_being_asked_for() {
        let values = Values {
            clipboard: Some("copied".into()),
            selection: Some("chosen".into()),
            ..Default::default()
        };
        assert_eq!(
            arguments("{{ clipboard }} {{ selection }}"),
            Vec::<String>::new()
        );
        assert_eq!(
            render("{{ clipboard }} {{ selection }}", &values).text,
            "copied chosen"
        );
    }

    #[test]
    fn a_missing_clipboard_or_selection_renders_as_empty_text() {
        let rendered = render("[{{ clipboard }}][{{ selection }}]", &Values::default());
        assert_eq!(rendered.text, "[][]");
    }

    #[test]
    fn the_date_placeholder_resolves_to_today() {
        let rendered = render("{{ date }}", &Values::default());
        assert_eq!(rendered.text, today());
        assert_eq!(rendered.text.len(), 10, "an ISO date, not a timestamp");
    }

    #[test]
    fn the_uuid_placeholder_differs_every_time() {
        let first = render("{{ uuid }}", &Values::default()).text;
        let second = render("{{ uuid }}", &Values::default()).text;
        assert_ne!(first, second);
        assert!(uuid::Uuid::parse_str(&first).is_ok());
    }

    #[test]
    fn an_unreserved_name_is_an_argument() {
        assert_eq!(arguments("Dear {{ name }},"), vec!["name".to_string()]);
    }

    #[test]
    fn a_template_of_reserved_names_has_no_arguments() {
        assert_eq!(
            arguments("{{ date }} {{ uuid }} {{ cursor }}"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn arguments_are_ordered_by_first_use() {
        assert_eq!(
            arguments("{{ zebra }} then {{ apple }} then {{ mango }}"),
            vec!["zebra".to_string(), "apple".into(), "mango".into()]
        );
    }

    #[test]
    fn the_order_is_the_same_every_time() {
        let source = "{{ zebra }} {{ apple }} {{ mango }} {{ cherry }} {{ pear }}";
        let first = arguments(source);
        for _ in 0..20 {
            assert_eq!(arguments(source), first, "a shuffling form is unusable");
        }
    }

    #[test]
    fn a_repeated_argument_is_reported_once_and_filled_everywhere() {
        assert_eq!(
            arguments("{{ city }} to {{ city }} via {{ stop }}"),
            vec!["city".to_string(), "stop".into()]
        );
        let values = Values::with_arguments(HashMap::from([
            ("city".into(), "Berlin".into()),
            ("stop".into(), "Kassel".into()),
        ]));
        assert_eq!(
            render("{{ city }} to {{ city }} via {{ stop }}", &values).text,
            "Berlin to Berlin via Kassel"
        );
    }

    #[test]
    fn a_name_is_matched_whole_not_as_a_substring() {
        assert_eq!(
            arguments("{{ username }} {{ user }}"),
            vec!["username".to_string(), "user".into()]
        );
    }

    #[test]
    fn query_is_prompted_for_even_though_it_is_reserved() {
        let template = Template::parse("https://example.com/s?q={{ query }}").unwrap();
        assert!(
            template.arguments().is_empty(),
            "it is reserved, so it is not an argument"
        );
        assert_eq!(
            template.prompts(),
            vec!["query".to_string()],
            "but nothing else can supply it"
        );
    }

    #[test]
    fn prompts_keep_template_order_alongside_arguments() {
        let template = Template::parse("{{ city }} {{ query }} {{ date }} {{ name }}").unwrap();
        assert_eq!(
            template.prompts(),
            vec!["city".to_string(), "query".into(), "name".into()]
        );
    }

    #[test]
    fn a_blank_argument_renders_as_empty_text() {
        let values = Values::with_arguments(HashMap::from([("name".into(), String::new())]));
        assert_eq!(render("[{{ name }}]", &values).text, "[]");
    }

    #[test]
    fn an_argument_that_was_never_supplied_renders_as_empty_text() {
        assert_eq!(render("[{{ name }}]", &Values::default()).text, "[]");
    }

    #[test]
    fn the_cursor_placeholder_leaves_no_text_and_reports_its_position() {
        let rendered = render("before{{ cursor }}after", &Values::default());
        assert_eq!(rendered.text, "beforeafter");
        assert_eq!(rendered.caret, Some(6));
    }

    #[test]
    fn a_template_without_a_cursor_reports_no_position() {
        assert_eq!(render("plain", &Values::default()).caret, None);
    }

    #[test]
    fn only_the_first_cursor_is_honoured() {
        let rendered = render("a{{ cursor }}b{{ cursor }}c", &Values::default());
        assert_eq!(rendered.text, "abc");
        assert_eq!(rendered.caret, Some(1));
    }

    #[test]
    fn the_caret_counts_characters_not_bytes() {
        let rendered = render("däng☃{{ cursor }}o", &Values::default());
        assert_eq!(rendered.text, "däng☃o");
        assert_eq!(rendered.caret, Some(5));
    }

    #[test]
    fn a_url_query_with_spaces_is_encoded() {
        let template = Template::parse("https://example.com/search?q={{ query }}").unwrap();
        let values = Values {
            query: Some("rust lang".into()),
            ..Default::default()
        };
        assert_eq!(
            template.render_url(&values).unwrap().text,
            "https://example.com/search?q=rust%20lang"
        );
    }

    #[test]
    fn a_url_query_cannot_change_the_urls_structure() {
        let template = Template::parse("https://example.com/s?q={{ query }}&safe=1").unwrap();
        let values = Values {
            query: Some("a&b?c#d".into()),
            ..Default::default()
        };
        let rendered = template.render_url(&values).unwrap().text;
        assert!(!rendered.contains("a&b"), "got {rendered}");
        assert!(rendered.ends_with("&safe=1"), "got {rendered}");
        assert_eq!(rendered.matches('#').count(), 0, "got {rendered}");
    }

    #[test]
    fn rendering_outside_a_url_does_not_encode() {
        let values = Values {
            clipboard: Some("a & b".into()),
            ..Default::default()
        };
        assert_eq!(render("{{ clipboard }}", &values).text, "a & b");
    }

    #[test]
    fn a_template_that_cannot_be_parsed_is_refused_with_a_message() {
        let error = Template::parse("unclosed {{ name").unwrap_err();
        assert!(matches!(error, TemplateError::Parse(_)));
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn a_doubled_brace_expression_becomes_a_visible_argument() {
        // The finding from the spike: this saves cleanly, so the create form has
        // to show that it will ask for `matrix`.
        assert_eq!(
            arguments("runs-on: ${{ matrix.os }}"),
            vec!["matrix".to_string()]
        );
    }

    #[test]
    fn an_escaped_doubled_brace_has_no_placeholders() {
        let source = "{% raw %}runs-on: ${{ matrix.os }}{% endraw %}";
        assert_eq!(arguments(source), Vec::<String>::new());
        assert_eq!(
            render(source, &Values::default()).text,
            "runs-on: ${{ matrix.os }}"
        );
    }

    #[test]
    fn uses_reports_a_reserved_name_the_template_references() {
        let template = Template::parse("https://example.com?q={{ query }}").unwrap();
        assert!(template.uses("query"));
        assert!(!template.uses("clipboard"));
    }

    #[test]
    fn rendering_meets_the_budget() {
        let body = "lorem ipsum dolor sit amet ".repeat(40);
        let source = format!(
            "{body}{}",
            (0..10)
                .map(|i| format!("{{{{ name{i} }}}} "))
                .collect::<String>()
        );
        assert!(source.len() >= 1000, "the budget is for a real template");
        let template = Template::parse(&source).unwrap();
        let values = Values::with_arguments(
            (0..10)
                .map(|i| (format!("name{i}"), "value".to_string()))
                .collect(),
        );

        let start = std::time::Instant::now();
        template.render(&values).unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(5),
            "rendering took {elapsed:?}, the budget is 5ms"
        );
    }
}
