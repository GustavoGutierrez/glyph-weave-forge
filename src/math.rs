pub(crate) fn preprocess_math_blocks(markdown: &str) -> String {
    let mut result = String::new();
    let mut in_math_block = false;

    for line in markdown.lines() {
        if line.trim() == "$$" {
            if in_math_block {
                result.push_str("```\n");
            } else {
                result.push_str("```math\n");
            }
            in_math_block = !in_math_block;
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}
