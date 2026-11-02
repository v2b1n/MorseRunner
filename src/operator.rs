// Copyright (C) 2004-2006 Alex Shovkoplyas, VE3NEA (referenced behavior).
// Copyright (C) 2026 Vladimir Vecgailis (Rust implementation).
// Rust reimplementation of operator behavior from DxOper.pas and message
// formatting rules from Station.pas.
// SPDX-License-Identifier: MPL-2.0
use crate::dsp::{blocks, Random, NEVER};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    None,
    Cq,
    Nr,
    Tu,
    MyCall,
    HisCall,
    B4,
    Qm,
    Nil,
    Garbage,
    RNr,
    RNr2,
    DeCall,
    DeCall2,
    DeCallNr,
    DeCallNr2,
    NrQm,
    LongCq,
    CallNr2,
    Qrl,
    Qrl2,
    Qsy,
    Agn,
}
impl Message {
    pub fn flag(self) -> u32 {
        1 << (self as u32)
    }
}
pub fn has(messages: u32, message: Message) -> bool {
    messages & message.flag() != 0
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    NeedPrevEnd,
    NeedQso,
    NeedNr,
    NeedCall,
    NeedCallNr,
    NeedEnd,
    Done,
    Failed,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CallMatch {
    No,
    Yes,
    Almost,
}
/// Exact weighted dynamic-programming matcher from TDxOperator.IsMyCall.
pub fn match_call(heard: &str, actual: &str, lids: bool) -> CallMatch {
    let c = heard.as_bytes();
    let c0 = actual.as_bytes();
    let mut m = vec![vec![0; c0.len() + 1]; c.len() + 1];
    for x in 1..=c.len() {
        m[x][0] = m[x - 1][0] + 2;
    }
    for x in 1..=c.len() {
        for y in 1..=c0.len() {
            let t = m[x][y - 1]
                + if x < c.len() && c[x - 1] != b'?' {
                    2
                } else {
                    0
                };
            let l = m[x - 1][y] + if c[x - 1] != b'?' { 2 } else { 0 };
            let d = m[x - 1][y - 1]
                + if c[x - 1] != c0[y - 1] && c[x - 1] != b'?' {
                    2
                } else {
                    0
                };
            m[x][y] = t.min(l).min(d);
        }
    }
    let mut result = match m[c.len()][c0.len()] {
        0 => CallMatch::Yes,
        1 | 2 => CallMatch::Almost,
        _ => CallMatch::No,
    };
    if !lids && c.len() == 2 && result == CallMatch::Almost {
        result = CallMatch::No;
    }
    if result == CallMatch::Yes && (c.len() != c0.len() || heard.contains('?')) {
        result = CallMatch::Almost;
    }
    if heard.replace('?', "").len() < 2 {
        result = CallMatch::No;
    }
    result
}
pub struct Operator {
    pub call: String,
    pub skills: u32,
    pub patience: i32,
    pub repeats: usize,
    pub state: State,
}
impl Operator {
    pub fn new(call: String, rng: &mut Random) -> Self {
        Self {
            call,
            skills: 1 + rng.int(3) as u32,
            patience: 5,
            repeats: 1,
            state: State::NeedPrevEnd,
        }
    }
    pub fn set_state(&mut self, state: State, single: bool, rng: &mut Random) {
        self.state = state;
        self.patience = if state == State::NeedQso {
            rng.rayleigh(4.).round_ties_even() as i32
        } else {
            5
        };
        self.repeats = if state == State::NeedQso && !single && rng.uniform() < 0.1 {
            2
        } else {
            1
        };
    }
    pub fn send_delay(&self, rng: &mut Random) -> i32 {
        if self.state == State::NeedPrevEnd {
            NEVER
        } else {
            blocks(0.1 + 0.5 * rng.uniform())
        }
    }
    pub fn reply_timeout(&self, rng: &mut Random) -> i32 {
        let n = blocks(6. - self.skills as f32) as f32;
        rng.gauss(n, n / 2.).round_ties_even() as i32
    }
    pub fn decrease_patience(&mut self) {
        if self.state == State::Done {
            return;
        }
        self.patience -= 1;
        if self.patience < 1 {
            self.state = State::Failed;
        }
    }
    // Retain upstream state branches for source-to-port review.
    #[allow(clippy::collapsible_match)]
    pub fn receive(
        &mut self,
        messages: u32,
        his_call: &str,
        lids: bool,
        single: bool,
        rng: &mut Random,
    ) {
        use State::*;
        if has(messages, Message::Cq) {
            match self.state {
                NeedPrevEnd => self.set_state(NeedQso, single, rng),
                NeedQso => self.decrease_patience(),
                NeedNr | NeedCall | NeedCallNr => self.state = Failed,
                NeedEnd => self.state = Done,
                _ => {}
            }
            return;
        }
        if has(messages, Message::Nil) {
            match self.state {
                NeedPrevEnd => self.set_state(NeedQso, single, rng),
                NeedQso => self.decrease_patience(),
                NeedNr | NeedCall | NeedCallNr | NeedEnd => self.state = Failed,
                _ => {}
            }
            return;
        }
        if has(messages, Message::HisCall) {
            let mut matched = match_call(his_call, &self.call, lids);
            if lids && his_call.len() > 3 {
                match matched {
                    CallMatch::Yes => {
                        if rng.uniform() < 0.01 {
                            matched = CallMatch::Almost;
                        }
                    }
                    CallMatch::Almost => {
                        if rng.uniform() < 0.04 {
                            matched = CallMatch::Yes;
                        }
                    }
                    _ => {}
                }
            }
            match matched {
                CallMatch::Yes => match self.state {
                    NeedPrevEnd | NeedQso | NeedCallNr => self.set_state(NeedNr, single, rng),
                    NeedCall => self.set_state(NeedEnd, single, rng),
                    _ => {}
                },
                CallMatch::Almost => match self.state {
                    NeedPrevEnd | NeedQso | NeedNr => self.set_state(NeedCallNr, single, rng),
                    NeedEnd => self.set_state(NeedCall, single, rng),
                    _ => {}
                },
                CallMatch::No => match self.state {
                    NeedQso => self.state = NeedPrevEnd,
                    NeedNr | NeedCall | NeedCallNr => self.state = Failed,
                    NeedEnd => self.state = Done,
                    _ => {}
                },
            }
        }
        if has(messages, Message::B4) {
            match self.state {
                NeedPrevEnd | NeedQso => self.set_state(NeedQso, single, rng),
                NeedNr | NeedEnd => self.state = Failed,
                _ => {}
            }
        }
        if has(messages, Message::Nr) {
            match self.state {
                NeedQso => self.state = NeedPrevEnd,
                NeedNr => {
                    if rng.uniform() < 0.9 {
                        self.set_state(NeedEnd, single, rng);
                    }
                }
                NeedCallNr => {
                    if rng.uniform() < 0.9 {
                        self.set_state(NeedCall, single, rng);
                    }
                }
                _ => {}
            }
        }
        if has(messages, Message::Tu) {
            match self.state {
                NeedPrevEnd => self.set_state(NeedQso, single, rng),
                NeedEnd => self.state = Done,
                _ => {}
            }
        }
        if !lids && messages == Message::Garbage.flag() {
            self.state = NeedPrevEnd;
        }
        if self.state != NeedPrevEnd {
            self.decrease_patience();
        }
    }
    pub fn reply(&self, rng: &mut Random) -> Message {
        use Message::*;
        use State::*;
        match self.state {
            NeedPrevEnd | Done | Failed => None,
            NeedQso => MyCall,
            NeedNr => {
                if self.patience == 4 || rng.uniform() < 0.3 {
                    NrQm
                } else {
                    Agn
                }
            }
            NeedCall => {
                if rng.uniform() > 0.5 {
                    DeCallNr
                } else if rng.uniform() > 0.25 {
                    DeCallNr2
                } else {
                    CallNr2
                }
            }
            NeedCallNr => {
                if rng.uniform() > 0.5 {
                    DeCall
                } else {
                    DeCall2
                }
            }
            NeedEnd => {
                if self.patience < 4 {
                    Nr
                } else if rng.uniform() < 0.9 {
                    RNr
                } else {
                    RNr2
                }
            }
        }
    }
}
/// Cut numbers and the original one-shot digit error + EEEEE correction.
pub fn report(rst: u32, nr: u32, with_error: &mut bool, rng: &mut Random) -> String {
    let mut result = format!("{rst}{nr:03}");
    if *with_error {
        let mut chars: Vec<_> = result.chars().collect();
        let mut i = chars.len() - 1;
        if !('2'..='7').contains(&chars[i]) {
            i -= 1;
        }
        if ('2'..='7').contains(&chars[i]) {
            chars[i] =
                char::from_u32((chars[i] as i32 + if rng.uniform() < 0.5 { -1 } else { 1 }) as u32)
                    .unwrap();
            result = chars.into_iter().collect();
            result.push_str(&format!("EEEEE {nr:03}"));
        }
        *with_error = false;
    }
    result = result
        .replace("599", "5NN")
        .replace("000", "TTT")
        .replace("00", "TT");
    if rng.uniform() < 0.4 {
        result = result.replace('0', "O");
    } else if rng.uniform() < 0.97 {
        result = result.replace('0', "T");
    }
    if rng.uniform() < 0.97 {
        result = result.replace('9', "N");
    }
    result
}
pub fn text(
    message: Message,
    my: &str,
    his: &str,
    rst: u32,
    nr: u32,
    error: &mut bool,
    rng: &mut Random,
) -> String {
    use Message::*;
    let template = match message {
        None => "",
        Cq => "CQ <my> TEST",
        Nr => "<#>",
        Tu => "TU",
        MyCall => "<my>",
        HisCall => "<his>",
        B4 => "QSO B4",
        Qm => "?",
        Nil => "NIL",
        Garbage => "",
        RNr => "R <#>",
        RNr2 => "R <#> <#>",
        DeCall => "DE <my>",
        DeCall2 => "DE <my> <my>",
        DeCallNr => "DE <my> <#>",
        DeCallNr2 => "DE <my> <my> <#>",
        NrQm => "NR?",
        LongCq => "CQ CQ TEST <my> <my> TEST",
        CallNr2 => "<my> <my> <#>",
        Qrl => "QRL?",
        Qrl2 => "QRL?   QRL?",
        Qsy => "<his>  QSY QSY",
        Agn => "AGN",
    };
    let mut result = template.to_owned();
    if result.contains("<#>") {
        result = result.replacen("<#>", &report(rst, nr, error, rng), 1);
        // Pascal evaluates NrAsText even when no second placeholder remains.
        // Preserve those RNG draws: later operator timings share this stream.
        let repeated_report = report(rst, nr, error, rng);
        result = result.replace("<#>", &repeated_report);
    }
    result.replace("<my>", my).replace("<his>", his)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matching_original_wildcards() {
        assert_eq!(match_call("DL1ABC", "DL1ABC", false), CallMatch::Yes);
        assert_eq!(match_call("1AB", "DL1ABC", false), CallMatch::Almost);
        assert_eq!(match_call("DL?ABC", "DL1ABC", false), CallMatch::Almost);
        assert_eq!(match_call("W9XYZ", "DL1ABC", false), CallMatch::No);
        assert_eq!(match_call("?", "DL1ABC", true), CallMatch::No);
    }
    #[test]
    fn corrected_call_and_report_required() {
        let mut rng = Random(57);
        let mut op = Operator::new("DL1ABC".into(), &mut rng);
        op.receive(Message::Cq.flag(), "", false, false, &mut rng);
        op.receive(Message::HisCall.flag(), "DL1?", false, false, &mut rng);
        assert_eq!(op.state, State::NeedCallNr);
        op.receive(Message::Nr.flag(), "DL1?", false, false, &mut rng);
        assert_eq!(op.state, State::NeedCall);
        op.receive(Message::Tu.flag(), "DL1?", false, false, &mut rng);
        assert_ne!(op.state, State::Done);
        op.receive(Message::HisCall.flag(), "DL1ABC", false, false, &mut rng);
        assert_eq!(op.state, State::NeedEnd);
        op.receive(Message::Tu.flag(), "DL1ABC", false, false, &mut rng);
        assert_eq!(op.state, State::Done);
    }
    #[test]
    fn impatient_station_leaves() {
        let mut rng = Random(73);
        let mut op = Operator::new("DL1ABC".into(), &mut rng);
        op.set_state(State::NeedNr, false, &mut rng);
        for _ in 0..5 {
            op.receive(Message::None.flag(), "", false, false, &mut rng);
        }
        assert_eq!(op.state, State::Failed);
    }
    #[test]
    fn nr_question_first_repeat() {
        let mut r = Random(33);
        let mut op = Operator::new("W9XYZ".into(), &mut r);
        op.state = State::NeedNr;
        op.patience = 4;
        assert_eq!(op.reply(&mut r), Message::NrQm);
    }
    #[test]
    fn single_digit_error_then_correction() {
        let mut r = Random(33);
        let mut error = true;
        let s = report(599, 236, &mut error, &mut r);
        assert!(s.contains("EEEEE 236"));
        assert!(!error);
        assert!(!report(599, 236, &mut error, &mut r).contains("EEEEE"));
    }
}
#[cfg(test)]
mod reference_tests {
    use super::*;
    #[test]
    fn transitions_match_upstream_pascal_matrix() {
        let states = [
            State::NeedPrevEnd,
            State::NeedQso,
            State::NeedNr,
            State::NeedCall,
            State::NeedCallNr,
            State::NeedEnd,
            State::Done,
            State::Failed,
        ];
        let masks = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 36, 40, 12, 24, 6, 34];
        let calls = ["DL1ABC", "DL1?", "1AB", "?", "DL1ABX", "W9XYZ"];
        let mut mismatches = vec![];
        for line in include_str!("../tests/fixtures/operator-cases.csv").lines() {
            let v: Vec<i32> = line.split(',').map(|n| n.parse().unwrap()).collect();
            let mut rng = Random(v[0] as u64);
            let mut op = Operator {
                call: "DL1ABC".into(),
                skills: 2,
                patience: 5,
                repeats: 1,
                state: states[v[1] as usize],
            };
            op.receive(
                masks[v[2] as usize],
                calls[v[3] as usize],
                v[4] != 0,
                v[5] != 0,
                &mut rng,
            );
            let reply = op.reply(&mut rng) as i32;
            let delay = op.send_delay(&mut rng);
            let timeout = op.reply_timeout(&mut rng);
            let actual = [
                op.state as i32,
                op.patience,
                op.repeats as i32,
                reply,
                delay,
                timeout,
            ];
            if actual != v[6..] {
                mismatches.push(format!("{line} -> {actual:?}"));
            }
        }
        assert!(
            mismatches.is_empty(),
            "{} cases differ: {:?}",
            mismatches.len(),
            &mismatches[..mismatches.len().min(12)]
        );
    }
}
