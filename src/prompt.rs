//! Interactive prompting, and the rule that governs it: an agent or CI run has
//! nobody to answer a question, so the CLI refuses instead of guessing.

use std::io::{BufRead, IsTerminal, Write};

use anyhow::Context as _;

/// Fail unless stdin is a terminal, naming the flag that avoids the prompt.
pub fn require_terminal(non_interactive_flag: &str) -> anyhow::Result<()> {
    if std::io::stdin().is_terminal() {
        return Ok(());
    }
    anyhow::bail!("stdin is not a terminal, so `kf` cannot prompt; pass `{non_interactive_flag}`")
}

/// Ask a yes/no question on the terminal. Without a terminal there is nobody to
/// answer, so this errors rather than silently assuming either answer.
pub fn confirm(question: &str) -> anyhow::Result<bool> {
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("{question} Refusing without a terminal to ask on; pass --yes.");
    }
    print!("{question} [y/N] ");
    std::io::stdout()
        .flush()
        .context("could not write the confirmation prompt")?;
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("could not read the confirmation answer")?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

/// Print a question and read the answer. The caller has already established
/// that there is a terminal to ask on, usually via `require_terminal`.
pub fn ask(question: &str) -> anyhow::Result<String> {
    print!("{question}");
    std::io::stdout().flush().ok();
    read_line()
}

/// One line from stdin, without its trailing newline.
pub fn read_line() -> anyhow::Result<String> {
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .context("reading from stdin")?;
    // The newline is a control character: left in place it reaches the
    // `Authorization` header and makes it unparseable.
    let length = line.trim_end_matches(['\r', '\n']).len();
    line.truncate(length);
    Ok(line)
}
