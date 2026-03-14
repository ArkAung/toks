use crate::pipeline::{token_estimate, CollapseBlank, Pipeline, StripAnsi, Truncate};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimate_various() {
        // empty string
        assert_eq!(token_estimate(""), 0);
        // ascii: length 5 => (5+3)/4 = 2
        assert_eq!(token_estimate("hello"), 2);
        // mixed ascii and space
        assert_eq!(token_estimate("a b"), ("a b").len().div_ceil(4));
    }

    #[test]
    fn test_pipeline_strip_ansi() {
        let raw = "\x1b[31mred\x1b[0m text\x1b[1m bold\x1b[0m";
        let p = Pipeline::new().push(StripAnsi);
        let (out, _, _) = p.run(raw);
        assert_eq!(out, "red text bold");
    }

    #[test]
    fn test_pipeline_collapse_blank() {
        // line1, blank, blank, line2, blank blank, line3, trailing newline
        let raw = "line1\n\n\nline2\n\n\nline3\n";
        let p = Pipeline::new().push(CollapseBlank);
        let (out, _, _) = p.run(raw);
        // CollapseBlank reduces consecutive blank lines to a single blank line.
        // Trailing newline is dropped by lines().
        assert_eq!(out, "line1\n\nline2\n\nline3");
    }

    #[test]
    fn test_pipeline_truncate_various() {
        // case where number of lines <= max
        let raw = "single line";
        let p = Pipeline::new().push(Truncate { max: 10, tail: 2 });
        let (out, _, _) = p.run(raw);
        assert_eq!(out, "single line");

        // exact max lines
        let raw = (0..10)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let p = Pipeline::new().push(Truncate { max: 10, tail: 2 });
        let (out, _, _) = p.run(&raw);
        assert_eq!(out, raw); // no change

        // need truncation: 12 lines, max=8, tail=2 => head=6, omitted = 12-8=4
        let raw = (0..12)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let p = Pipeline::new().push(Truncate { max: 8, tail: 2 });
        let (out, _, _) = p.run(&raw);
        let expected = [
            "line0",
            "line1",
            "line2",
            "line3",
            "line4",
            "line5",
            "... 4 lines omitted ...",
            "line10",
            "line11",
        ]
        .join("\n");
        assert_eq!(out, expected);
    }

    #[test]
    fn test_pipeline_chain() {
        let raw = "\x1b[31mline1\x1b[0m\n\n\nline2\n";
        let p = Pipeline::new()
            .push(StripAnsi)
            .push(CollapseBlank)
            .push(Truncate { max: 5, tail: 2 });
        let (out, _, _) = p.run(raw);
        // StripAnsi -> "line1\n\n\nline2\n"
        // CollapseBlank -> "line1\n\nline2\n"
        // Now we have 3 lines (line1, blank, line2) which is <= max=5, so no truncation.
        assert_eq!(out, "line1\n\nline2");
    }

    #[test]
    fn test_pipeline_empty() {
        let raw = "";
        let p = Pipeline::new()
            .push(StripAnsi)
            .push(CollapseBlank)
            .push(Truncate::default());
        let (out, _, _) = p.run(raw);
        assert_eq!(out, "");
    }
}
