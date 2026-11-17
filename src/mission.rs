//! The original's mission scripts, its `.tok` files: tokenised source, a line a statement.
//! `docs/tok-format.md` describes them.

/// How long the head of a script is: the size of what follows, and how many lines.
const HEAD: usize = 8;
/// What a token starts with, by what it is.
const WORD: u8 = 0x80;
const VARIABLE: u8 = 0x81;
const KIND: u8 = 0x82;
const SPEECH: u8 = 0x83;
/// From this number up a word is a keyword or an operator; below it, a function.
const FIRST_KEYWORD: u16 = 128;
/// What the number of a kind of unit or building is more than its place in the rules' lists.
const FIRST_KIND: u16 = 111;

/// One piece of a statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// A function, by its number.
    Function(u16),
    /// A keyword or an operator, by its number.
    Keyword(u16),
    /// A variable, by its number.
    Variable(u16),
    /// A kind of unit or building, by its place among the entries of the rules'
    /// `[BuildingTypes]` and `[UnitTypes]` lists in the order of the file.
    Kind(u16),
    /// A line of speech, by its number.
    Speech(u16),
    /// What is written as it is: digits, commas, brackets.
    Text(String),
}

/// Reads a script into its lines, each the tokens of one statement: empty for a blank line
/// of the source. `None` if it is not a script.
pub fn read(script: &[u8]) -> Option<Vec<Vec<Token>>> {
    let field = |at: usize| {
        let bytes = script.get(at..at + 4)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?) as usize)
    };
    let (size, lines) = (field(0)?, field(4)?);
    let body = script.get(HEAD..HEAD + size)?;
    let read: Vec<Vec<Token>> = body.split(|&byte| byte == 0).map(tokens).collect();
    // The last line's zero leaves an empty piece after it.
    (read.len() == lines + 1).then(|| read[..lines].to_vec())
}

/// The tokens of one line.
fn tokens(line: &[u8]) -> Vec<Token> {
    // A number kept seven bits to a byte, lowest first, in bytes with their top bit set.
    let number = |bytes: &[u8]| {
        bytes
            .iter()
            .rev()
            .fold(0, |sum, &byte| sum << 7 | u16::from(byte & 0x7f))
    };
    let mut read = Vec::new();
    let mut at = 0;
    while at < line.len() {
        let long = match line[at] {
            WORD | KIND | SPEECH => 2,
            VARIABLE => 1,
            _ => 0,
        };
        let marked = line
            .get(at + 1..at + 1 + long)
            .filter(|bytes| long > 0 && bytes.iter().all(|&byte| byte >= 0x80));
        match marked {
            Some(bytes) => {
                let n = number(bytes);
                read.push(match line[at] {
                    WORD if n < FIRST_KEYWORD => Token::Function(n),
                    WORD => Token::Keyword(n),
                    VARIABLE => Token::Variable(n),
                    KIND => Token::Kind(n.saturating_sub(FIRST_KIND)),
                    _ => Token::Speech(n),
                });
                at += 1 + long;
            }
            None => {
                let end = line[at + 1..]
                    .iter()
                    .position(|&byte| byte >= 0x80)
                    .map_or(line.len(), |far| at + 1 + far);
                read.push(Token::Text(
                    String::from_utf8_lossy(&line[at..end]).into_owned(),
                ));
                at = end;
            }
        }
    }
    read
}

/// The lines of speech a script plays, in the order it names them, each once.
pub fn speeches(lines: &[Vec<Token>]) -> Vec<u16> {
    let mut named = Vec::new();
    for token in lines.iter().flatten() {
        if let Token::Speech(line) = token
            && !named.contains(line)
        {
            named.push(*line);
        }
    }
    named
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A script of these lines, with its head.
    fn script(lines: &[&[u8]]) -> Vec<u8> {
        let body: Vec<u8> = lines
            .iter()
            .flat_map(|line| line.iter().copied().chain([0]))
            .collect();
        let mut script = (body.len() as u32).to_le_bytes().to_vec();
        script.extend((lines.len() as u32).to_le_bytes());
        script.extend(body);
        script
    }

    #[test]
    fn a_script_reads_into_lines_of_functions_keywords_variables_kinds_and_text() {
        let lines: [&[u8]; 4] = [
            // v6 = F5(1)
            &[
                0x81, 0x86, 0x80, 0xb4, 0x81, 0x80, 0x85, 0x80, b'(', b'1', b')',
            ],
            &[],
            // K139(v0,<kind 138>,v13,0,0,1)
            b"\x80\x8b\x81(\x81\x80,\x82\x8a\x81,\x81\x8d,0,0,1)",
            // F45(<speech 642>)
            b"\x80\xad\x80(\x83\x82\x85)",
        ];
        let read = read(&script(&lines)).unwrap();
        assert_eq!(read.len(), 4);
        assert_eq!(
            read[0],
            [
                Token::Variable(6),
                Token::Keyword(180),
                Token::Function(5),
                Token::Text("(1)".to_string()),
            ]
        );
        assert!(read[1].is_empty(), "a blank line of the source");
        assert_eq!(read[2][0], Token::Keyword(139));
        assert_eq!(read[2][2], Token::Variable(0));
        // 138 in the file: the 27th of the rules' lists, the Atreides construction yard.
        assert_eq!(read[2][4], Token::Kind(27));
        assert_eq!(read[2].last(), Some(&Token::Text(",0,0,1)".to_string())));
        assert_eq!(read[3][0], Token::Function(45));
        assert_eq!(speeches(&read), [642]);
    }

    #[test]
    fn what_is_not_a_script_is_not_read() {
        assert_eq!(read(&[1, 2, 3]), None);
        let mut cut = script(&[b"1", b"2"]);
        cut.truncate(cut.len() - 1);
        assert_eq!(read(&cut), None, "shorter than it says");
        let mut wrong = script(&[b"1", b"2"]);
        wrong[4] = 5;
        assert_eq!(read(&wrong), None, "not as many lines as it says");
    }
}
