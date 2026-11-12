//! Hegel IP control wire protocol (ASCII over TCP, `\r` terminated).
//!
//! Format: `-<k>.<param>\r` with k = p (power), v (volume), i (input),
//! m (mute), r (reset).

pub const AMP_PORT: u16 = 50001;
pub const MAX_VOLUME: u8 = 99;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Power,
    Volume,
    Input,
    Mute,
    Reset,
}

impl Command {
    /// Three character mnemonic, e.g. `-v.`.
    pub fn mnemonic(self) -> &'static str {
        match self {
            Command::Power => "-p.",
            Command::Volume => "-v.",
            Command::Input => "-i.",
            Command::Mute => "-m.",
            Command::Reset => "-r.",
        }
    }

    pub fn from_mnemonic(s: &str) -> Option<Command> {
        match s {
            "-p." => Some(Command::Power),
            "-v." => Some(Command::Volume),
            "-i." => Some(Command::Input),
            "-m." => Some(Command::Mute),
            "-r." => Some(Command::Reset),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Param {
    Value(u8),
    Toggle,
    Up,
    Down,
    Query,
    Stop,
}

impl Param {
    pub fn code(self) -> String {
        match self {
            Param::Value(v) => v.to_string(),
            Param::Toggle => "t".into(),
            Param::Up => "u".into(),
            Param::Down => "d".into(),
            Param::Query => "?".into(),
            Param::Stop => "~".into(),
        }
    }

    /// Parses the payload of a command (without terminator).
    pub fn parse(s: &str) -> Option<Param> {
        match s {
            "t" => Some(Param::Toggle),
            "u" => Some(Param::Up),
            "d" => Some(Param::Down),
            "?" => Some(Param::Query),
            "~" => Some(Param::Stop),
            _ => parse_value(s).map(Param::Value),
        }
    }
}

fn parse_value(s: &str) -> Option<u8> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Builds a wire message including the trailing `\r`. Volume values are clamped to 99.
pub fn build(cmd: Command, param: Param) -> String {
    let param = match (cmd, param) {
        (Command::Volume, Param::Value(v)) => Param::Value(v.min(MAX_VOLUME)),
        (_, p) => p,
    };
    format!("{}{}\r", cmd.mnemonic(), param.code())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub cmd: Command,
    pub value: u8,
}

/// Parses a status line such as `-v.42\r`. Never panics; returns None for garbage.
pub fn parse_status(line: &str) -> Option<Status> {
    let line = line.trim_end_matches(['\r', '\n']);
    let head = line.get(..3)?;
    let cmd = Command::from_mnemonic(head)?;
    let value = parse_value(line.get(3..)?)?;
    Some(Status { cmd, value })
}

/// Parses a command line sent to the amplifier, e.g. `-v.u\r`.
pub fn parse_command(line: &str) -> Option<(Command, Param)> {
    let line = line.trim_end_matches(['\r', '\n']);
    let cmd = Command::from_mnemonic(line.get(..3)?)?;
    let param = Param::parse(line.get(3..)?)?;
    Some((cmd, param))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mute_and_others() {
        assert_eq!(
            parse_status("-m.1\r"),
            Some(Status {
                cmd: Command::Mute,
                value: 1
            })
        );
        assert_eq!(
            parse_status("-v.42\r\n"),
            Some(Status {
                cmd: Command::Volume,
                value: 42
            })
        );
        assert_eq!(
            parse_status("-p.0"),
            Some(Status {
                cmd: Command::Power,
                value: 0
            })
        );
        assert_eq!(
            parse_status("-i.10\r"),
            Some(Status {
                cmd: Command::Input,
                value: 10
            })
        );
    }

    #[test]
    fn parse_malformed() {
        for s in [
            "", "-", "-m.", "-m.\r", "-m.x\r", "-m.-1\r", "-m.1x", "-z.1\r", "garbage", "-v.999\r",
            "-v.?\r", "ö", "-ö.1", "-m.1 \r", "-m. 1",
        ] {
            assert_eq!(parse_status(s), None, "{s:?}");
        }
    }

    #[test]
    fn build_commands() {
        assert_eq!(build(Command::Power, Param::Query), "-p.?\r");
        assert_eq!(build(Command::Mute, Param::Toggle), "-m.t\r");
        assert_eq!(build(Command::Volume, Param::Up), "-v.u\r");
        assert_eq!(build(Command::Volume, Param::Down), "-v.d\r");
        assert_eq!(build(Command::Input, Param::Value(7)), "-i.7\r");
        assert_eq!(build(Command::Reset, Param::Stop), "-r.~\r");
        assert_eq!(build(Command::Volume, Param::Value(42)), "-v.42\r");
    }

    #[test]
    fn volume_clamped() {
        assert_eq!(build(Command::Volume, Param::Value(100)), "-v.99\r");
        assert_eq!(build(Command::Volume, Param::Value(255)), "-v.99\r");
        assert_eq!(build(Command::Input, Param::Value(200)), "-i.200\r");
    }

    #[test]
    fn command_roundtrip() {
        for p in [
            Param::Value(5),
            Param::Toggle,
            Param::Up,
            Param::Down,
            Param::Query,
            Param::Stop,
        ] {
            let s = build(Command::Mute, p);
            assert_eq!(parse_command(&s), Some((Command::Mute, p)));
        }
        assert_eq!(parse_command("-x.1\r"), None);
    }
}
