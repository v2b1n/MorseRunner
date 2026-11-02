// Copyright (C) 2004-2006 Alex Shovkoplyas, VE3NEA (referenced behavior).
// Copyright (C) 2026 Vladimir Vecgailis (Rust implementation).
// Rust reimplementation of behavior from Contest.pas, Station.pas, DxStn.pas,
// MyStn.pas, QrmStn.pas, QrnStn.pas and Log.pas (practice-mode engine).
// SPDX-License-Identifier: MPL-2.0
use crate::{
    dsp::{self, Agc, Fading, Modulator, Random, Receiver, BLOCK, NEVER, RATE},
    morse,
    operator::{self, has, Message, Operator, State},
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::f64::consts::TAU;
#[derive(Clone)]
pub struct Config {
    pub call: String,
    pub wpm: f32,
    pub pitch: f32,
    pub bandwidth: f32,
    pub rit: f32,
    pub volume: f32,
    pub noise: f32,
    pub monitor: f32,
    pub qsb: bool,
    pub flutter: bool,
    pub qrn: bool,
    pub qrm: bool,
    pub lids: bool,
    pub qsk: bool,
    pub single: bool,
    pub activity: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            call: "N0CALL".into(),
            wpm: 25.,
            pitch: 600.,
            bandwidth: 500.,
            rit: 0.,
            volume: 0.35,
            noise: 0.02,
            monitor: 0.75,
            qsb: false,
            flutter: false,
            qrn: false,
            qrm: false,
            lids: false,
            qsk: true,
            single: false,
            activity: 3,
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Record {
    pub time: String,
    pub call: String,
    pub rst: String,
    pub nr: String,
    pub true_call: String,
    pub true_rst: String,
    pub true_nr: String,
    pub prefix: String,
    pub error: String,
    pub sent: usize,
}
#[derive(Clone, Debug)]
pub struct ViewStation {
    pub call: String,
    pub nr: u32,
    pub rst: u32,
    pub pitch: f32,
    pub amp: f32,
    pub state: State,
    pub sending: bool,
}
#[derive(Clone)]
pub struct Snapshot {
    pub callers: Vec<ViewStation>,
    pub logs: Vec<Record>,
    pub revision: u64,
    pub transmitting: bool,
    pub text: String,
    pub elapsed: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RadioState {
    Listening,
    Copying,
    Preparing,
    Sending,
}
#[derive(Clone, Copy)]
enum Event {
    MsgSent,
    Timeout,
    MeStarted,
    MeFinished,
}
enum Kind {
    Dx {
        oper: Operator,
        qsb: Fading,
        error: bool,
    },
    Qrm {
        patience: i32,
    },
    Qrn,
}
struct Station {
    kind: Kind,
    call: String,
    his: String,
    nr: u32,
    rst: u32,
    wpm: f32,
    offset: f32,
    amplitude: f32,
    phase: f32,
    envelope: Vec<f32>,
    position: usize,
    timeout: i32,
    state: RadioState,
    alive: bool,
    report_completed: bool,
}
impl Station {
    fn dx(config: &Config, minute: f32, rng: &mut Random, call: String) -> Self {
        let oper = Operator::new(call.clone(), rng);
        let error = config.lids && rng.uniform() < 0.1;
        let wpm = (config.wpm * 0.5 * (1. + rng.uniform())).round_ties_even();
        let nr = 1 + (rng.uniform() * minute * oper.skills as f32).round_ties_even() as u32;
        let rst = if config.lids && rng.uniform() < 0.03 {
            559 + 10 * rng.int(4) as u32
        } else {
            599
        };
        let mut qsb = Fading::new(0.1, rng);
        let bandwidth = 0.1 + rng.uniform() / 2.;
        qsb.set_bandwidth(bandwidth, rng);
        if config.flutter && rng.uniform() < 0.3 {
            let bandwidth = 3. + rng.uniform() * 30.;
            qsb.set_bandwidth(bandwidth, rng);
        }
        let amplitude = 9000. + 18000. * (1. + rng.ushaped());
        let offset = rng.gauss(0., 300.).round_ties_even();
        Self {
            kind: Kind::Dx { oper, qsb, error },
            call,
            his: config.call.clone(),
            nr,
            rst,
            wpm,
            offset,
            amplitude,
            phase: 0.,
            envelope: vec![],
            position: 0,
            timeout: NEVER,
            state: RadioState::Copying,
            alive: true,
            report_completed: false,
        }
    }
    fn send(&mut self, messages: &[Message], rng: &mut Random) {
        let mut result = String::new();
        for message in messages {
            if *message == Message::None {
                self.state = RadioState::Listening;
                return;
            }
            let error = match &mut self.kind {
                Kind::Dx { error, .. } => error,
                _ => &mut false,
            };
            let text = operator::text(
                *message, &self.call, &self.his, self.rst, self.nr, error, rng,
            );
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(&text);
        }
        self.envelope = morse::envelope(&result, self.wpm, RATE);
        for v in &mut self.envelope {
            *v *= self.amplitude;
        }
        self.position = 0;
        self.phase = 0.;
        self.state = RadioState::Sending;
        self.timeout = NEVER;
    }
    fn event(
        &mut self,
        event: Event,
        me_sending: bool,
        messages: u32,
        his_call: &str,
        config: &Config,
        rng: &mut Random,
    ) {
        match &mut self.kind {
            Kind::Dx { oper, .. } => {
                if oper.state == State::Done {
                    return;
                }
                match event {
                    Event::MsgSent => {
                        self.timeout = if me_sending {
                            NEVER
                        } else {
                            oper.reply_timeout(rng)
                        };
                    }
                    Event::Timeout => {
                        if self.state == RadioState::Listening {
                            oper.receive(
                                Message::None.flag(),
                                his_call,
                                config.lids,
                                config.single,
                                rng,
                            );
                            if oper.state == State::Failed {
                                self.alive = false;
                                return;
                            }
                            self.state = RadioState::Preparing;
                        }
                        if self.state == RadioState::Preparing {
                            let messages = (0..oper.repeats)
                                .map(|_| oper.reply(rng))
                                .collect::<Vec<_>>();
                            self.send(&messages, rng);
                        }
                    }
                    Event::MeFinished => {
                        if self.state != RadioState::Sending {
                            let received = if self.state == RadioState::Copying
                                || has(messages, Message::Cq)
                                || has(messages, Message::Tu)
                                || has(messages, Message::Nil)
                            {
                                messages
                            } else {
                                Message::Garbage.flag()
                            };
                            oper.receive(received, his_call, config.lids, config.single, rng);
                            if oper.state == State::Failed {
                                self.alive = false;
                                return;
                            }
                            self.timeout = oper.send_delay(rng);
                            self.state = RadioState::Preparing;
                        }
                    }
                    Event::MeStarted => {
                        if self.state != RadioState::Sending {
                            self.state = RadioState::Copying;
                        }
                        self.timeout = NEVER;
                    }
                }
            }
            Kind::Qrm { patience } => match event {
                Event::MsgSent => {
                    *patience -= 1;
                    if *patience == 0 {
                        self.alive = false;
                    } else {
                        self.timeout =
                            rng.gauss(dsp::blocks(4.) as f32, 2.).round_ties_even() as i32;
                    }
                }
                Event::Timeout => self.send(&[Message::LongCq], rng),
                _ => {}
            },
            Kind::Qrn => {
                if matches!(event, Event::MsgSent) {
                    self.alive = false;
                }
            }
        }
    }
    fn tick(
        &mut self,
        me_sending: bool,
        messages: u32,
        his_call: &str,
        config: &Config,
        rng: &mut Random,
    ) {
        if self.state == RadioState::Sending && self.position >= self.envelope.len() {
            if matches!(&self.kind, Kind::Dx { oper, .. } if oper.state == State::NeedEnd) {
                self.report_completed = true;
            }
            self.envelope.clear();
            self.state = RadioState::Listening;
            self.event(Event::MsgSent, me_sending, messages, his_call, config, rng);
        } else if self.state != RadioState::Sending {
            if self.timeout > -1 {
                self.timeout -= 1;
            }
            if self.timeout == 0 {
                self.event(Event::Timeout, me_sending, messages, his_call, config, rng);
            }
        }
    }
    fn block(&mut self, config: &Config, rng: &mut Random) -> [f32; BLOCK] {
        let mut out = [0.; BLOCK];
        let n = (self.envelope.len() - self.position).min(BLOCK);
        out[..n].copy_from_slice(&self.envelope[self.position..self.position + n]);
        self.position += n;
        if let Kind::Dx { qsb, .. } = &mut self.kind {
            if config.qsb {
                qsb.apply(&mut out, rng);
            }
        }
        out
    }
}
struct Piece {
    message: Message,
    text: String,
    envelope: Vec<f32>,
    position: usize,
}
struct Me {
    pieces: VecDeque<Piece>,
    messages: u32,
    his_call: String,
    nr: u32,
    text: String,
}
impl Me {
    fn new() -> Self {
        Self {
            pieces: VecDeque::new(),
            messages: 0,
            his_call: String::new(),
            nr: 1,
            text: String::new(),
        }
    }
    fn sending(&self) -> bool {
        !self.pieces.is_empty()
    }
}
pub struct Engine {
    pub config: Config,
    pub active: bool,
    rng: Random,
    stations: Vec<Station>,
    me: Me,
    pending_completion: Option<(Vec<Message>, String)>,
    block_number: u64,
    receiver: Receiver,
    modulator: Modulator,
    agc: Agc,
    rit_phase: f32,
    pub logs: Vec<Record>,
    pub revision: u64,
    #[cfg(test)]
    test_calls: Option<Vec<String>>,
}
impl Engine {
    pub fn new(seed: u64) -> Self {
        Self {
            config: Config::default(),
            active: false,
            rng: Random(seed.max(1)),
            stations: vec![],
            me: Me::new(),
            pending_completion: None,
            block_number: 0,
            receiver: Receiver::new(500.),
            modulator: Modulator::new(600.),
            agc: Agc::new(),
            rit_phase: 0.,
            logs: vec![],
            revision: 0,
            #[cfg(test)]
            test_calls: None,
        }
    }
    pub fn start(&mut self, config: Config) {
        self.config = config;
        self.active = true;
        self.stations.clear();
        self.me = Me::new();
        self.pending_completion = None;
        self.block_number = 0;
        self.receiver = Receiver::new(self.config.bandwidth);
        self.modulator = Modulator::new(self.config.pitch);
        self.agc = Agc::new();
        self.rit_phase = 0.;
        self.logs.clear();
        self.revision += 1;
    }
    pub fn stop(&mut self) {
        self.active = false;
        self.pending_completion = None;
        self.stations.clear();
        self.me.pieces.clear();
    }
    fn elapsed(&self) -> f32 {
        self.block_number as f32 * BLOCK as f32 / RATE as f32
    }
    fn pick_call(&mut self) -> String {
        #[cfg(test)]
        if let Some(calls) = &self.test_calls {
            return calls[self.rng.int(calls.len())].clone();
        }
        let calls = crate::calls();
        calls[self.rng.int(calls.len())].clone()
    }
    fn add_caller(&mut self) {
        let call = self.pick_call();
        let station = Station::dx(&self.config, self.elapsed() / 60., &mut self.rng, call);
        self.stations.push(station);
    }
    fn notify(&mut self, event: Event) {
        let sending = self.me.sending();
        for s in self.stations.iter_mut().rev() {
            s.event(
                event,
                sending,
                self.me.messages,
                &self.me.his_call,
                &self.config,
                &mut self.rng,
            );
        }
    }
    fn me_finished(&mut self) {
        if !self.logs.is_empty() {
            self.trace_exchange("TX finished before receive");
        }
        if !self.config.single
            && (has(self.me.messages, Message::Cq)
                || (!self.logs.is_empty()
                    && has(self.me.messages, Message::Tu)
                    && has(self.me.messages, Message::MyCall)))
        {
            for _ in 0..self.rng.poisson(self.config.activity as f32 / 2.) {
                self.add_caller();
            }
        }
        self.notify(Event::MeFinished);
        if !self.logs.is_empty() {
            self.trace_exchange("TX finished after receive");
        }
    }
    fn trace_exchange(&self, event: &str) {
        if cfg!(test) {
            return;
        }
        use std::io::Write;
        let path = crate::data_dir();
        let _ = std::fs::create_dir_all(&path);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.join("qso-diagnostic.log"))
        {
            let states: Vec<_> = self
                .stations
                .iter()
                .filter_map(|s| match &s.kind {
                    Kind::Dx { oper, .. } => {
                        Some(format!("{}:{:?}/{:?}", s.call, oper.state, s.state))
                    }
                    _ => None,
                })
                .collect();
            let _ = writeln!(
                file,
                "{:.2} {} flags={} his={} queued={} last={:?} stations={:?}",
                self.elapsed(),
                event,
                self.me.messages,
                self.me.his_call,
                self.pending_completion.is_some(),
                self.logs.last().map(|q| (&q.call, &q.error)),
                states
            );
        }
    }
    /// ESM completion must not send a short TU over the tail of the report.
    /// Manual transmissions still use `send` and retain normal collision behavior.
    pub fn complete_exchange(&mut self, messages: &[Message], call: &str) {
        self.trace_exchange(&format!("ESM complete {} {:?}", call, messages));
        if !self.active {
            return;
        }
        if self.stations.iter().any(|s| {
            s.call == call
                && s.state == RadioState::Sending
                && matches!(&s.kind, Kind::Dx { oper, .. } if oper.state == State::NeedEnd)
        }) {
            self.pending_completion = Some((messages.to_vec(), call.to_owned()));
        } else {
            self.send(messages, call);
        }
    }
    pub fn send(&mut self, messages: &[Message], his: &str) {
        if !self.active {
            return;
        }
        let was_sending = self.me.sending();
        if !was_sending {
            self.me.messages = 0;
            self.me.text.clear();
        }
        for message in messages {
            if *message == Message::HisCall && !his.is_empty() {
                self.me.his_call = his.to_owned();
            }
            self.me.messages |= message.flag();
            let empty = self.me.pieces.is_empty();
            self.me.pieces.push_back(Piece {
                message: *message,
                text: String::new(),
                envelope: vec![],
                position: 0,
            });
            if empty {
                self.start_piece();
            }
        }
        if !was_sending {
            self.notify(Event::MeStarted);
        }
    }
    fn start_piece(&mut self) {
        if let Some(p) = self.me.pieces.front_mut() {
            let text = operator::text(
                p.message,
                &self.config.call,
                &self.me.his_call,
                599,
                self.me.nr,
                &mut false,
                &mut self.rng,
            );
            p.envelope = morse::envelope(&text, self.config.wpm, RATE);
            p.text = text.clone();
            self.me.text = text;
        }
    }
    pub fn update_call(&mut self, call: &str) -> bool {
        if call.is_empty() {
            return false;
        }
        for (i, p) in self.me.pieces.iter_mut().enumerate() {
            if p.message != Message::HisCall {
                continue;
            }
            let new = morse::envelope(call, self.config.wpm, RATE);
            if i > 0 || (new.len() >= p.position && new[..p.position] == p.envelope[..p.position]) {
                p.text = call.to_owned();
                p.envelope = new;
                self.me.his_call = call.to_owned();
                return true;
            }
        }
        false
    }
    pub fn abort_tx(&mut self) {
        self.pending_completion = None;
        if self.me.sending() {
            self.me.pieces.clear();
            self.me.messages = Message::Garbage.flag();
            self.me.text.clear();
            self.me_finished();
        }
    }
    pub fn log(&mut self, call: &str, rst: &str, nr: &str) {
        let call = call.replace('?', "");
        let seconds = self.elapsed() as u32;
        self.logs.push(Record {
            time: format!("{:02}:{:02}", seconds / 60, seconds % 60),
            prefix: morse::prefix(&call),
            call,
            rst: rst.into(),
            nr: nr.into(),
            true_call: String::new(),
            true_rst: String::new(),
            true_nr: String::new(),
            error: "NIL".into(),
            sent: self.me.nr as usize,
        });
        self.me.nr += 1;
        self.revision += 1;
        self.verify_last();
    }
    fn verify_last(&mut self) {
        let Some(last) = self.logs.last() else {
            return;
        };
        if let Some(i) = self.stations.iter().position(|s| {
            s.call == last.call && matches!(&s.kind,Kind::Dx{oper,..}if oper.state==State::Done || (oper.state==State::NeedEnd && s.report_completed))
        }) {
            let done = matches!(&self.stations[i].kind, Kind::Dx { oper, .. } if oper.state == State::Done);
            if !last.true_call.is_empty() {
                if done { self.stations.remove(i); }
                return;
            }
            let s = &self.stations[i];
            let n = self.logs.len() - 1;
            let duplicate = self.logs[..n]
                .iter()
                .any(|q| q.call == s.call && q.error.is_empty());
            let q = &mut self.logs[n];
            q.true_call = s.call.clone();
            q.true_rst = s.rst.to_string();
            q.true_nr = s.nr.to_string();
            q.error = if duplicate {
                "DUP"
            } else if q.rst.parse::<u32>().ok() != Some(s.rst) {
                "RST"
            } else if q.nr.parse::<u32>().ok() != Some(s.nr) {
                "NR"
            } else {
                ""
            }
            .into();
            self.revision += 1;
            if done { self.stations.remove(i); }
        }
    }
    fn qrm(&mut self) {
        let patience = 1 + self.rng.int(5) as i32;
        let call = self.pick_call();
        let amplitude = 5000. + 25000. * self.rng.uniform();
        let offset = self.rng.gauss(0., 300.).round_ties_even();
        let wpm = 30. + self.rng.int(20) as f32;
        let mut s = Station {
            kind: Kind::Qrm { patience },
            call,
            his: self.config.call.clone(),
            nr: 0,
            rst: 0,
            wpm,
            offset,
            amplitude,
            phase: 0.,
            envelope: vec![],
            position: 0,
            timeout: NEVER,
            state: RadioState::Listening,
            alive: true,
            report_completed: false,
        };
        let message = match self.rng.int(7) {
            0 => Message::Qrl,
            1 | 2 => Message::Qrl2,
            3..=5 => Message::LongCq,
            _ => Message::Qsy,
        };
        s.send(&[message], &mut self.rng);
        self.stations.push(s);
    }
    fn qrn(&mut self) {
        let n = dsp::blocks(self.rng.uniform()) as usize * BLOCK;
        let amplitude = 1e5 * 10_f32.powf(2. * self.rng.uniform());
        let mut envelope = vec![0.; n];
        for v in &mut envelope {
            if self.rng.uniform() < 0.01 {
                *v = (self.rng.uniform() - 0.5) * amplitude;
            }
        }
        self.stations.push(Station {
            kind: Kind::Qrn,
            call: String::new(),
            his: String::new(),
            nr: 0,
            rst: 0,
            wpm: 0.,
            offset: 0.,
            amplitude,
            phase: 0.,
            envelope,
            position: 0,
            timeout: NEVER,
            state: RadioState::Sending,
            alive: true,
            report_completed: false,
        });
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            callers: self
                .stations
                .iter()
                .filter_map(|s| {
                    if let Kind::Dx { oper, .. } = &s.kind {
                        Some(ViewStation {
                            call: s.call.clone(),
                            nr: s.nr,
                            rst: s.rst,
                            pitch: self.config.pitch + s.offset,
                            amp: s.amplitude / 45000.,
                            state: oper.state,
                            sending: s.state == RadioState::Sending,
                        })
                    } else {
                        None
                    }
                })
                .collect(),
            logs: self.logs.clone(),
            revision: self.revision,
            transmitting: self.me.sending(),
            text: self.me.text.clone(),
            elapsed: self.elapsed(),
        }
    }
    pub fn render(&mut self) -> [f32; BLOCK] {
        let mut out = [0.; BLOCK];
        if !self.active {
            return out;
        }
        self.block_number += 1;
        if self.block_number < 6 {
            return out;
        }
        let mut re = [0.; BLOCK];
        let mut im = [0.; BLOCK];
        let noiseamp = self.config.noise * 300000.;
        for i in 0..BLOCK {
            re[i] = 3. * noiseamp * (self.rng.uniform() - 0.5);
            im[i] = 3. * noiseamp * (self.rng.uniform() - 0.5);
        }
        if self.config.qrn {
            for r in &mut re {
                if self.rng.uniform() < 0.01 {
                    *r = 60. * 6000. * (self.rng.uniform() - 0.5);
                }
            }
            if self.rng.uniform() < 0.01 {
                self.qrn();
            }
        }
        if self.config.qrm && self.rng.uniform() < 0.0002 {
            self.qrm();
        }
        for s in &mut self.stations {
            if s.state != RadioState::Sending {
                continue;
            }
            let blk = s.block(&self.config, &mut self.rng);
            for i in 0..BLOCK {
                // Delphi stores phases as Single but evaluates expressions containing
                // TWO_PI and trig in extended/double precision before assigning them.
                let bfo = (s.phase as f64
                    - self.rit_phase as f64
                    - i as f64 * TAU * self.config.rit as f64 / RATE as f64)
                    as f32;
                re[i] = (re[i] as f64 + blk[i] as f64 * (bfo as f64).cos()) as f32;
                im[i] = (im[i] as f64 - blk[i] as f64 * (bfo as f64).sin()) as f32;
                s.phase += (TAU * s.offset as f64 / RATE as f64) as f32;
                if s.phase as f64 > TAU {
                    s.phase = (s.phase as f64 - TAU) as f32;
                }
            }
        }
        self.rit_phase = (self.rit_phase as f64
            + BLOCK as f64 * TAU * self.config.rit as f64 / RATE as f64)
            as f32;
        while self.rit_phase as f64 > TAU {
            self.rit_phase = (self.rit_phase as f64 - TAU) as f32;
        }
        while (self.rit_phase as f64) < -TAU {
            self.rit_phase = (self.rit_phase as f64 + TAU) as f32;
        }
        if let Some(piece) = self.me.pieces.front_mut() {
            let mut rfg = 1_f32;
            let monitor = 10_f32.powf((self.config.monitor - 0.75) * 4.);
            for i in 0..BLOCK {
                let v = piece
                    .envelope
                    .get(piece.position + i)
                    .copied()
                    .unwrap_or(0.);
                let blk = v * 300000.;
                if self.config.qsk {
                    if rfg > 1. - v {
                        rfg = 1. - v;
                    } else {
                        rfg = rfg * 0.997 + 0.003;
                    }
                    re[i] = monitor * blk + rfg * re[i];
                    im[i] = monitor * blk + rfg * im[i];
                } else {
                    re[i] = monitor * blk;
                    im[i] = monitor * blk;
                }
            }
            piece.position += BLOCK;
        }
        self.receiver.apply(&mut re, &mut im, self.config.bandwidth);
        for i in 0..BLOCK {
            let v = self.modulator.apply(re[i], im[i], self.config.pitch);
            out[i] = (self.agc.apply(v) / 32768. * self.config.volume).clamp(-1., 1.);
        }
        let finished = self
            .me
            .pieces
            .front()
            .is_some_and(|p| p.position >= p.envelope.len());
        if finished {
            self.me.pieces.pop_front();
            if !self.me.sending() {
                self.me_finished();
            } else {
                self.start_piece();
            }
        }
        let sending = self.me.sending();
        for s in self.stations.iter_mut().rev() {
            s.tick(
                sending,
                self.me.messages,
                &self.me.his_call,
                &self.config,
                &mut self.rng,
            );
        }
        if self.pending_completion.as_ref().is_some_and(|(_, call)| {
            !self
                .stations
                .iter()
                .any(|s| s.call == *call && s.state == RadioState::Sending)
        }) {
            let (messages, call) = self.pending_completion.take().unwrap();
            self.send(&messages, &call);
        }
        self.verify_last();
        self.stations.retain(|s| s.alive);
        if self.config.single
            && !self
                .stations
                .iter()
                .any(|s| matches!(&s.kind,Kind::Dx{oper,..}if oper.state!=State::Done))
        {
            self.add_caller();
            let s = self.stations.last_mut().unwrap();
            s.event(
                Event::MeFinished,
                self.me.sending(),
                Message::Cq.flag(),
                &self.me.his_call,
                &self.config,
                &mut self.rng,
            );
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run(e: &mut Engine, seconds: f32) {
        for _ in 0..dsp::blocks(seconds) {
            e.render();
        }
    }
    #[test]
    fn single_station_repeats_and_eventually_leaves() {
        let mut e = Engine::new(42);
        e.start(Config {
            single: true,
            wpm: 60.,
            ..Default::default()
        });
        run(&mut e, 1.0);
        let call = e.snapshot().callers[0].call.clone();
        assert!(e.snapshot().callers[0].sending);
        run(&mut e, 80.);
        assert!(e.snapshot().callers.iter().all(|s| s.call != call));
    }
    #[test]
    fn no_false_verified_qso_for_wrong_callsign() {
        let mut e = Engine::new(55);
        e.start(Config {
            single: true,
            wpm: 60.,
            ..Default::default()
        });
        run(&mut e, 5.);
        e.send(&[Message::HisCall, Message::Nr], "ZZ0WRONG");
        run(&mut e, 8.);
        e.send(&[Message::Tu], "ZZ0WRONG");
        e.log("ZZ0WRONG", "599", "1");
        run(&mut e, 3.);
        assert_eq!(e.logs[0].error, "NIL");
    }
    #[test]
    fn qsk_ducks_only_keyed_samples() {
        let mut e = Engine::new(12);
        e.start(Config {
            noise: 0.02,
            qsk: true,
            wpm: 40.,
            ..Default::default()
        });
        e.send(&[Message::MyCall], "");
        let mut peak = 0_f32;
        for _ in 0..100 {
            for v in e.render() {
                assert!(v.is_finite());
                peak = peak.max(v.abs());
            }
        }
        assert!(peak > 0.01 && peak <= 1.);
    }
    #[test]
    fn call_correction_can_extend_unsent_suffix() {
        let mut e = Engine::new(12);
        e.start(Config::default());
        e.send(&[Message::HisCall], "VE3");
        assert!(e.update_call("DL1ABC"));
        assert_eq!(e.me.his_call, "DL1ABC");
    }
}
#[cfg(test)]
mod reference_tests {
    use super::*;
    #[test]
    fn full_station_timeline_matches_original_delphi_classes() {
        let mut e = Engine::new(77);
        e.start(Config {
            wpm: 40.,
            activity: 6,
            lids: true,
            flutter: true,
            ..Default::default()
        });
        let mut chosen = String::new();
        let mut actual = String::new();
        for b in 1..=1500 {
            match b {
                10 | 750 | 900 => e.send(&[Message::Cq], ""),
                200 => {
                    if let Some(s) = e.stations.first() {
                        chosen = s.call.clone();
                        let partial = format!("{}?", &chosen[..chosen.len().min(3)]);
                        e.send(&[Message::HisCall], &partial);
                    }
                }
                300 => e.send(&[Message::HisCall, Message::Nr], &chosen),
                400 => e.send(&[Message::Qm], ""),
                500 => {
                    e.send(&[Message::Tu, Message::MyCall], "");
                    e.logs.push(Record {
                        time: String::new(),
                        call: String::new(),
                        rst: String::new(),
                        nr: String::new(),
                        true_call: String::new(),
                        true_rst: String::new(),
                        true_nr: String::new(),
                        prefix: String::new(),
                        error: "NIL".into(),
                        sent: 1,
                    });
                }
                850 => e.send(&[Message::Nil], ""),
                _ => {}
            }
            e.render();
            actual.push_str(&format!("{b}|{}|", u32::from(e.me.sending())));
            for s in &e.stations {
                if let Kind::Dx { oper, .. } = &s.kind {
                    actual.push_str(&format!(
                        "{},{},{},{},{},{},{},{},{};",
                        s.call,
                        oper.state as u32,
                        oper.patience,
                        s.state as u32,
                        s.timeout,
                        s.wpm as u32,
                        s.rst,
                        s.nr,
                        s.offset as i32
                    ));
                }
            }
            actual.push_str(&format!("|rng={}\n", e.rng.0));
        }
        let expected = include_str!("../tests/fixtures/station-timeline.txt");
        let mismatch = actual.lines().zip(expected.lines()).find(|(a, b)| a != b);
        assert!(
            mismatch.is_none(),
            "First station timeline difference (Rust, Pascal): {mismatch:?}"
        );
    }
}
#[cfg(test)]
mod audio_reference_test {
    use super::*;
    fn replay(config: Config, count: usize) -> (String, Vec<f32>) {
        let mut e = Engine::new(77);
        e.start(config);
        let mut chosen = String::new();
        let mut trace = String::new();
        let mut audio = vec![];
        for b in 1..=count {
            match b {
                10 | 750 | 900 => e.send(&[Message::Cq], ""),
                100 => e.qrm(),
                200 => {
                    if let Some(s) = e.stations.first() {
                        chosen = s.call.clone();
                        let partial = format!("{}?", &chosen[..chosen.len().min(3)]);
                        e.send(&[Message::HisCall], &partial);
                    }
                }
                300 => e.send(&[Message::HisCall, Message::Nr], &chosen),
                400 => e.send(&[Message::Qm], ""),
                500 => {
                    e.send(&[Message::Tu, Message::MyCall], "");
                    e.logs.push(Record {
                        time: String::new(),
                        call: String::new(),
                        rst: String::new(),
                        nr: String::new(),
                        true_call: String::new(),
                        true_rst: String::new(),
                        true_nr: String::new(),
                        prefix: String::new(),
                        error: "NIL".into(),
                        sent: 1,
                    });
                }
                850 => e.send(&[Message::Nil], ""),
                _ => {}
            }
            audio.extend(e.render());
            trace.push_str(&format!("{b}|{}|", u32::from(e.me.sending())));
            for s in &e.stations {
                if let Kind::Dx { oper, .. } = &s.kind {
                    trace.push_str(&format!(
                        "{},{},{},{},{},{},{},{},{};",
                        s.call,
                        oper.state as u32,
                        oper.patience,
                        s.state as u32,
                        s.timeout,
                        s.wpm as u32,
                        s.rst,
                        s.nr,
                        s.offset as i32
                    ));
                }
            }
            trace.push_str(&format!("|rng={}\n", e.rng.0));
        }
        (trace, audio)
    }
    fn verify(trace: &str, audio: &[f32], expected_trace: &str, bytes: &[u8]) {
        let mismatch = trace
            .lines()
            .zip(expected_trace.lines())
            .find(|(a, b)| a != b);
        assert!(
            mismatch.is_none(),
            "Noisy timeline difference (Rust, Pascal): {mismatch:?}"
        );
        assert_eq!(trace.lines().count(), expected_trace.lines().count());
        assert_eq!(audio.len() * 4, bytes.len());
        let mut error2 = 0_f64;
        let mut signal2 = 0_f64;
        let mut max = 0_f32;
        for (a, b) in audio.iter().zip(bytes.as_chunks::<4>().0.iter()) {
            let expected = f32::from_le_bytes(*b) / 32768. * 0.35;
            max = max.max((a - expected).abs());
            error2 += (*a as f64 - expected as f64).powi(2);
            signal2 += (expected as f64).powi(2);
        }
        let relative_rms = (error2 / signal2).sqrt();
        eprintln!(
            "Whole-path audio: max absolute sample error={max}, relative RMS error={relative_rms}"
        );
        assert!(
            relative_rms < 0.0001 && max < 0.0001,
            "Full audio path error {relative_rms}, max {max}"
        );
    }
    fn config() -> Config {
        Config {
            wpm: 40.,
            activity: 6,
            lids: true,
            flutter: true,
            qsb: true,
            qrn: true,
            qrm: true,
            ..Default::default()
        }
    }
    #[test]
    fn noisy_qsk_session_matches_original_contest_get_audio() {
        let (trace, audio) = replay(config(), 1500);
        verify(
            &trace,
            &audio,
            include_str!("../tests/fixtures/audio-timeline.txt"),
            include_bytes!("../tests/fixtures/full-audio.f32"),
        );
    }
    #[test]
    fn qsk_off_narrow_receiver_and_negative_rit_match_original() {
        let (trace, audio) = replay(
            Config {
                qsk: false,
                bandwidth: 200.,
                pitch: 800.,
                rit: -90.,
                ..config()
            },
            800,
        );
        verify(
            &trace,
            &audio,
            include_str!("../tests/fixtures/audio-timeline-qsk-off.txt"),
            include_bytes!("../tests/fixtures/full-audio-qsk-off.f32"),
        );
    }
}
#[cfg(test)]
mod qso_integration_tests {
    use super::*;
    #[test]
    fn acknowledged_contact_still_checks_received_number() {
        let mut e = Engine::new(91);
        e.start(Config::default());
        let mut station = Station::dx(&e.config, 1., &mut e.rng, "DL1ABC".into());
        if let Kind::Dx { oper, .. } = &mut station.kind {
            oper.state = State::NeedEnd;
        }
        station.report_completed = true;
        let rst = station.rst.to_string();
        let wrong_nr = (station.nr + 1).to_string();
        e.stations.push(station);
        e.log("DL1ABC", &rst, &wrong_nr);
        assert_eq!(e.logs[0].error, "NR");
        assert_eq!(
            e.stations.len(),
            1,
            "Logging must not terminate the radio exchange"
        );
    }
    #[test]
    fn verified_contact_requires_completed_two_way_exchange() {
        complete_contact(false, true);
    }
    #[test]
    fn early_enter_waits_for_report_and_verifies_without_another_cq() {
        complete_contact(true, true);
    }
    #[test]
    fn pileup_enter_verifies_without_another_cq() {
        complete_contact(true, false);
    }
    fn complete_contact(early: bool, single: bool) {
        let mut e = Engine::new(91);
        // Keep this protocol test independent from the user's replaceable callsign dataset.
        e.test_calls = Some(vec!["DL1ABC".into()]);
        e.start(Config {
            single,
            activity: 8,
            wpm: 60.,
            ..Default::default()
        });
        if !single {
            e.send(&[Message::Cq], "");
        }
        let mut target = None;
        let mut heard_report = false;
        let mut sent = false;
        for b in 0..2000 {
            e.render();
            let snap = e.snapshot();
            if early && target.is_none() {
                for station in &mut e.stations {
                    station.wpm = 10.;
                }
            }
            if let Some(s) = snap.callers.first() {
                if target.is_none() && !s.sending && !snap.transmitting && s.state == State::NeedQso
                {
                    target = Some((s.call.clone(), s.rst, s.nr));
                    e.send(&[Message::HisCall, Message::Nr], &s.call);
                } else if target.as_ref().is_some_and(|t| t.0 == s.call) {
                    if s.sending && s.state == State::NeedEnd {
                        heard_report = true;
                    }
                    if (early || !s.sending) && !snap.transmitting && !sent {
                        if heard_report && s.state == State::NeedEnd {
                            let (call, rst, nr) = target.clone().unwrap();
                            if early {
                                e.complete_exchange(&[Message::Tu, Message::MyCall], &call);
                            }
                            e.log(&call, &rst.to_string(), &nr.to_string());
                            sent = true;
                            if !early {
                                assert!(e.logs[0].error.is_empty(), "Completed remote acknowledgement must verify immediately when logged");
                                e.complete_exchange(&[Message::Tu, Message::MyCall], &call);
                            } else {
                                assert_eq!(
                                    e.logs[0].error, "NIL",
                                    "Must hear the entire remote report first"
                                );
                            }
                        } else if b % 50 == 0 && s.state == State::NeedNr {
                            e.send(&[Message::Nr], &s.call);
                        }
                    }
                }
            }
            if sent && e.logs[0].error.is_empty() {
                assert!(!e.logs[0].true_call.is_empty());
                return;
            }
        }
        panic!("No verified contact for target {target:?}: {:?}", e.logs);
    }
}
