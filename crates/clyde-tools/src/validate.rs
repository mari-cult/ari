use {
    super::{ToolError, ToolResult},
    std::ops::Range,
};

fn is_lowercase_or_hyphen(character: char) -> bool {
    matches!(character, 'a'..='z' | '_')
}

pub fn validate_tool_name(name: &str) -> ToolResult<()> {
    const RANGE: Range<usize> = 3..24;

    if RANGE.contains(&name.len()) && name.chars().all(is_lowercase_or_hyphen) {
        Ok(())
    } else {
        Err(ToolError::other(
            "Tool name must be 3 to 24 lowercase or hyphen characters.",
        ))
    }
}

pub fn validate_tool_description(description: &str) -> ToolResult<()> {
    const RANGE: Range<usize> = 10..64;

    if RANGE.contains(&description.chars().count()) {
        Ok(())
    } else {
        Err(ToolError::other(
            "Tool description must be 10 to 64 characters.",
        ))
    }
}
