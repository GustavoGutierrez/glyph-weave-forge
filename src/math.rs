#![cfg_attr(not(feature = "renderer-typst"), allow(dead_code))]

use crate::core::{ForgeError, Result};

pub(crate) fn normalize_markdown(markdown: &str) -> String {
    markdown.to_owned()
}

pub(crate) fn latex_to_typst_math(input: &str) -> Result<String> {
    if input.contains("\\begin{") || input.contains("\\end{") {
        return Err(ForgeError::Render {
            message: "unsupported LaTeX environment command (\\begin/\\end)".to_owned(),
        });
    }

    let mut parser = LatexParser::new(input);
    parser.parse_expression()
}

struct LatexParser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> LatexParser<'a> {
    fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    fn parse_expression(&mut self) -> Result<String> {
        let mut out = String::new();
        while let Some(ch) = self.peek_char() {
            match ch {
                '\\' => out.push_str(&self.parse_command()?),
                '{' => {
                    self.bump_char();
                    out.push('(');
                    out.push_str(&self.parse_until_closing_brace()?);
                    out.push(')');
                }
                '}' => break,
                _ => {
                    self.bump_char();
                    out.push(ch);
                }
            }
        }
        Ok(normalize_math_output(&out))
    }

    fn parse_until_closing_brace(&mut self) -> Result<String> {
        let mut out = String::new();
        loop {
            let Some(ch) = self.peek_char() else {
                return Err(ForgeError::Render {
                    message: "unbalanced braces in math expression".to_owned(),
                });
            };
            match ch {
                '\\' => out.push_str(&self.parse_command()?),
                '{' => {
                    self.bump_char();
                    out.push('(');
                    out.push_str(&self.parse_until_closing_brace()?);
                    out.push(')');
                }
                '}' => {
                    self.bump_char();
                    break;
                }
                _ => {
                    self.bump_char();
                    out.push(ch);
                }
            }
        }
        Ok(normalize_math_output(&out))
    }

    fn parse_group(&mut self) -> Result<String> {
        self.consume_required('{')?;
        self.parse_until_closing_brace()
    }

    fn parse_command(&mut self) -> Result<String> {
        self.consume_required('\\')?;
        let name = self.consume_command_name();

        match name.as_str() {
            "frac" => {
                let numerator = self.parse_group()?;
                let denominator = self.parse_group()?;
                Ok(format!("frac({numerator}, {denominator})"))
            }
            "sqrt" => {
                let arg = self.parse_group()?;
                Ok(format!("sqrt({arg})"))
            }
            "left" | "right" => Ok(String::new()),
            "," => Ok(" ".to_owned()),
            "qquad" => Ok("  ".to_owned()),
            "alpha" | "beta" | "gamma" | "delta" | "lambda" | "pi" | "sigma" => {
                Ok(format!(" {name} "))
            }
            "Omega" | "Delta" => Ok(format!(" {name} ")),
            "sum" => Ok(" sum ".to_owned()),
            "prod" => Ok(" prod ".to_owned()),
            "int" => Ok(" integral ".to_owned()),
            "lim" => Ok(" lim ".to_owned()),
            "pm" => Ok("+-".to_owned()),
            "infty" => Ok(" infinity ".to_owned()),
            "in" => Ok(" in ".to_owned()),
            "approx" => Ok("~=".to_owned()),
            "neq" => Ok("!=".to_owned()),
            "leq" | "le" => Ok("<=".to_owned()),
            "geq" | "ge" => Ok(">=".to_owned()),
            "to" => Ok("->".to_owned()),
            "cdot" => Ok("*".to_owned()),
            "times" => Ok(" times ".to_owned()),
            "sin" | "cos" | "exp" => Ok(format!(" {name} ")),
            "partial" => Ok(" partial ".to_owned()),
            "" => Err(ForgeError::Render {
                message: "incomplete command in math expression".to_owned(),
            }),
            unknown => Err(ForgeError::Render {
                message: format!("unsupported LaTeX command: \\{unknown}"),
            }),
        }
    }

    fn consume_required(&mut self, expected: char) -> Result<()> {
        match self.bump_char() {
            Some(ch) if ch == expected => Ok(()),
            _ => Err(ForgeError::Render {
                message: format!("malformed math expression near expected '{expected}'"),
            }),
        }
    }

    fn consume_command_name(&mut self) -> String {
        let Some(ch) = self.peek_char() else {
            return String::new();
        };

        if ch == ',' {
            self.bump_char();
            return ",".to_owned();
        }

        let mut out = String::new();
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphabetic() {
                out.push(ch);
                self.bump_char();
            } else {
                break;
            }
        }
        out
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
}

fn normalize_math_output(input: &str) -> String {
    let compact = input.split_whitespace().collect::<Vec<_>>().join(" ");
    split_multi_letter_identifiers(&compact)
}

fn split_multi_letter_identifiers(input: &str) -> String {
    let keep_words = [
        "sum", "prod", "integral", "lim", "sin", "cos", "exp", "alpha", "beta", "gamma", "delta",
        "lambda", "pi", "sigma", "Omega", "Delta", "partial", "sqrt", "frac", "infinity", "times",
        "in",
    ];

    let mut out = String::new();
    let mut word = String::new();

    let flush_word = |word: &mut String, out: &mut String| {
        if word.is_empty() {
            return;
        }
        if word.len() > 1 && !keep_words.contains(&word.as_str()) {
            for (idx, ch) in word.chars().enumerate() {
                if idx > 0 {
                    out.push(' ');
                }
                out.push(ch);
            }
        } else {
            out.push_str(word);
        }
        word.clear();
    };

    for ch in input.chars() {
        if ch.is_ascii_alphabetic() {
            word.push(ch);
        } else {
            flush_word(&mut word, &mut out);
            out.push(ch);
        }
    }
    flush_word(&mut word, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::latex_to_typst_math;

    #[test]
    fn converts_fraction_and_sqrt() {
        let out = latex_to_typst_math(r"\frac{1}{\sqrt{x}}").expect("conversion should succeed");
        assert_eq!(out, "frac(1, sqrt(x))");
    }

    #[test]
    fn converts_symbols_and_greek() {
        let out =
            latex_to_typst_math(r"\alpha + \beta \to \infty").expect("conversion should succeed");
        assert_eq!(out, "alpha + beta -> infinity");
    }

    #[test]
    fn preserves_membership_operator() {
        let out = latex_to_typst_math(r"x \in [0,L]").expect("conversion should succeed");
        assert_eq!(out, "x in [0,L]");
    }

    #[test]
    fn keeps_grouped_scripts() {
        let out = latex_to_typst_math(r"x^{n+1} + a_{ij}").expect("conversion should succeed");
        assert_eq!(out, "x^(n+1) + a_(i j)");
    }

    #[test]
    fn converts_sum_integral_and_spacing() {
        let out = latex_to_typst_math(r"\sum_{n=1}^{\infty} b_n \, dx + \int_0^L f(x)")
            .expect("conversion should succeed");
        assert_eq!(out, "sum _(n=1)^(infinity) b_n d x + integral _0^L f(x)");
    }

    #[test]
    fn errors_on_unsupported_command() {
        let err = latex_to_typst_math(r"\unknown{x}").expect_err("should fail on unknown command");
        assert!(err.to_string().contains("unsupported LaTeX command"));
    }

    #[test]
    fn errors_on_environment_commands() {
        let err = latex_to_typst_math(r"\begin{align}x\end{align}")
            .expect_err("should fail on environments");
        assert!(err.to_string().contains("unsupported LaTeX environment"));
    }
}
