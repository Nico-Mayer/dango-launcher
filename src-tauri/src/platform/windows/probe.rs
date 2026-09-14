//! A throwaway probe: what does the focused element actually expose?
//!
//! The whole Windows direct-selection plan rests on applications answering UI
//! Automation with a text pattern. Custom-rendered editors may not, and the one
//! that started this change is exactly that sort. This samples the focused
//! element every few seconds and prints what it found, so the answer is measured
//! rather than assumed.
//!
//! `cargo test probe_focused_selection -- --ignored --nocapture` and focus each
//! application in turn while it runs.

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use uiautomation::patterns::UITextPattern;
    use uiautomation::UIAutomation;

    #[test]
    #[ignore]
    fn probe_focused_selection() {
        let samples = std::env::var("DANGO_PROBE_SAMPLES")
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(10u32);
        let every = Duration::from_secs(
            std::env::var("DANGO_PROBE_EVERY")
                .ok()
                .and_then(|text| text.parse().ok())
                .unwrap_or(3),
        );

        let automation = UIAutomation::new().expect("UI Automation is available");
        println!("probing {samples} times, every {}s", every.as_secs());

        for sample in 1..=samples {
            std::thread::sleep(every);
            let started = Instant::now();
            let element = match automation.get_focused_element() {
                Ok(element) => element,
                Err(error) => {
                    println!("[{sample}] no focused element: {error}");
                    continue;
                }
            };
            let name = element.get_name().unwrap_or_default();
            let class = element.get_classname().unwrap_or_default();
            let control = element
                .get_control_type()
                .map(|control| format!("{control:?}"))
                .unwrap_or_default();

            match element.get_pattern::<UITextPattern>() {
                Err(error) => println!(
                    "[{sample}] {class}/{control} name={name:?} NO TEXT PATTERN ({error}) in {:?}",
                    started.elapsed()
                ),
                Ok(pattern) => match pattern.get_selection() {
                    Err(error) => println!(
                        "[{sample}] {class}/{control} name={name:?} pattern but no selection ({error}) in {:?}",
                        started.elapsed()
                    ),
                    Ok(ranges) => {
                        let text: Vec<String> = ranges
                            .iter()
                            .map(|range| range.get_text(200).unwrap_or_default())
                            .collect();
                        println!(
                            "[{sample}] {class}/{control} name={name:?} ranges={} text={:?} in {:?}",
                            ranges.len(),
                            text,
                            started.elapsed()
                        );
                    }
                },
            }
        }
    }
}
