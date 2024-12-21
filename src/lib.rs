use regex::Regex;
use std::sync::OnceLock;

/// A single candidate in a `srcset`: a URL plus optional "width" or "density".
#[derive(Debug, Clone, PartialEq)]
pub struct ImageCandidate {
    pub url: String,
    pub width: Option<f64>,
    pub density: Option<f64>,
}

impl PartialOrd for ImageCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.width, self.density, other.width, other.density) {
            (Some(a), None, Some(b), None) => Some(a.partial_cmp(&b).unwrap()),
            (None, Some(a), None, Some(b)) => Some(a.partial_cmp(&b).unwrap()),
            _ => None,
        }
    }
}

/// Regex for matching srcset segments.
///
/// Explanation:
/// 1. `(\S*[^,\s])` captures a run of non-whitespace, stopping before `,` or space at the end,
///    which we treat as the `url`.
/// 2. `(\s+([\d.]+)(x|w))?` is optional (`?`) and captures:
///    - `([\d.]+)` which is the numeric part (value),
///    - `(x|w)` which indicates the descriptor (density or width).
///
/// The entire pattern is repeated globally on the input text.
static SRCSEG_PATTERN: &str = r"(\S*[^,\s])(\s+([\d.]+)(x|w))?";
static SRCSEG_REGEX: OnceLock<Regex> = OnceLock::new();

/// Parses an `srcset` string and returns a vector of `ImageCandidate`s.
///
/// # Examples
/// ```
/// let srcset = "image1.png 1x, image2.png 2x, image3.png 100w";
/// let candidates = srcset_parse::parse(srcset);
/// assert_eq!(candidates.len(), 3);
/// assert_eq!(candidates[0].density, Some(1.0));
/// assert_eq!(candidates[1].density, Some(2.0));
/// assert_eq!(candidates[2].width, Some(100.0));
/// ```
pub fn parse(srcset: &str) -> Vec<ImageCandidate> {
    let re = SRCSEG_REGEX.get_or_init(|| Regex::new(SRCSEG_PATTERN).expect("Invalid regex"));
    let mut results = Vec::new();

    for caps in re.captures_iter(srcset) {
        // Group 1: the `url`
        let url = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        // Group 3: the numeric value (e.g. "1", "2", "100")
        let value = caps.get(3).map(|m| m.as_str());
        // Group 4: the descriptor (e.g. "x" or "w")
        let descriptor = caps.get(4).map(|m| m.as_str());

        // Convert the captured numeric value to f64 if present
        let parsed_value = value.map(|v| v.parse::<f64>().unwrap_or_default());

        // Fill in the struct's fields based on the descriptor
        let (width, density) = match descriptor {
            Some("w") => (parsed_value, None),
            Some("x") => (None, parsed_value),
            _ => (None, None),
        };

        results.push(ImageCandidate {
            url,
            width,
            density,
        });
    }

    results
}

#[cfg(test)]
mod tests {

    use super::{parse, ImageCandidate};

    #[test]
    fn parses_srcset_strings() {
        let srcset = "cat-@2x.jpeg 2x, dog.jpeg 100w";
        let result = parse(srcset);
        assert_eq!(
            result,
            vec![
                ImageCandidate {
                    url: "cat-@2x.jpeg".to_string(),
                    width: None,
                    density: Some(2.0),
                },
                ImageCandidate {
                    url: "dog.jpeg".to_string(),
                    width: Some(100.0),
                    density: None,
                },
            ]
        );
    }

    #[test]
    fn ignores_extra_whitespaces() {
        let srcset = r#"
            foo-bar.png     2x ,
            bar-baz.png  100w
        "#;

        let result = parse(srcset);
        assert_eq!(
            result,
            vec![
                ImageCandidate {
                    url: "foo-bar.png".to_string(),
                    width: None,
                    density: Some(2.0),
                },
                ImageCandidate {
                    url: "bar-baz.png".to_string(),
                    width: Some(100.0),
                    density: None,
                },
            ]
        );
    }

    #[test]
    fn properly_parses_float_descriptors() {
        let srcset = "cat.jpeg 2.4x, dog.jpeg 1.5x";
        let result = parse(srcset);
        assert_eq!(
            result,
            vec![
                ImageCandidate {
                    url: "cat.jpeg".to_string(),
                    width: None,
                    density: Some(2.4),
                },
                ImageCandidate {
                    url: "dog.jpeg".to_string(),
                    width: None,
                    density: Some(1.5),
                },
            ]
        );
    }

    #[test]
    fn supports_urls_that_contain_comma() {
        let srcset = r#"
          https://foo.bar/w=100,h=200/dog.png  100w,
          https://baz.bar/cat.png?meow=yes     1024w
        "#;

        let result = parse(srcset);
        assert_eq!(
            result,
            vec![
                ImageCandidate {
                    url: "https://foo.bar/w=100,h=200/dog.png".to_string(),
                    width: Some(100.0),
                    density: None,
                },
                ImageCandidate {
                    url: "https://baz.bar/cat.png?meow=yes".to_string(),
                    width: Some(1024.0),
                    density: None,
                },
            ]
        );
    }

    #[test]
    fn supports_single_urls() {
        let srcset = "/cat.jpg";
        let result = parse(srcset);
        assert_eq!(
            result,
            vec![ImageCandidate {
                url: "/cat.jpg".to_string(),
                width: None,
                density: None,
            }]
        );
    }

    #[test]
    fn supports_optional_descriptors() {
        let srcset = "/cat.jpg, /dog.png 3x , /lol ";
        let result = parse(srcset);
        assert_eq!(
            result,
            vec![
                ImageCandidate {
                    url: "/cat.jpg".to_string(),
                    width: None,
                    density: None,
                },
                ImageCandidate {
                    url: "/dog.png".to_string(),
                    width: None,
                    density: Some(3.0),
                },
                ImageCandidate {
                    url: "/lol".to_string(),
                    width: None,
                    density: None,
                },
            ]
        );
    }
}
