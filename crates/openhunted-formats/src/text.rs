// SPDX-License-Identifier: GPL-3.0-or-later
//! Listas de texto (`.RES`): linhas CRLF em Latin-1.

/// Divide um `.RES` em linhas (CRLF ou LF), decodificando Latin-1.
/// Linhas vazias são preservadas; só a última é descartada se vier vazia.
pub fn parse_text_list(data: &[u8]) -> Vec<String> {
    let mut lines: Vec<String> = data
        .split(|&b| b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l))
        .map(|l| l.iter().map(|&b| b as char).collect())
        .collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_keeps_blank_lines() {
        assert_eq!(parse_text_list(b"a\r\n\r\nb\r\n"), vec!["a", "", "b"]);
    }

    #[test]
    fn text_decodes_latin1_and_lone_lf() {
        assert_eq!(parse_text_list(b"ol\xe1\nfim"), vec!["olá", "fim"]);
    }
}
