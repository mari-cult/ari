use std::fmt;

/// A `@tool`.
pub struct Tool<'a> {
    pub module_name: &'a str,
    pub name: &'a str,
    pub args: &'a str,
    pub output: &'a str,
    pub documentation: &'a str,
}

impl<'a> fmt::Display for Tool<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("@tool\ndef ")?;
        fmt.write_str(self.name)?;
        fmt.write_str(self.args)?;
        fmt.write_str(" -> ")?;
        fmt.write_str(self.output)?;
        fmt.write_str(":\n    \"\"\"")?;
        fmt.write_str(self.documentation)?;
        fmt.write_str("\"\"\"\n    return ")?;
        fmt.write_str("bridged_")?;
        fmt.write_str(self.name)?;
        fmt.write_str(self.args)?;
        fmt.write_str("\n\n")?;

        Ok(())
    }
}
