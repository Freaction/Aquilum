use std::sync::mpsc::{Receiver, TryRecvError, channel};

use super::super::markdown::State;
use super::Document;
use super::scan::{scan, walk};

type Edited = ((usize, usize), isize);

#[derive(Default)]
pub(super) struct Checkpoints {
    points: Vec<(usize, State)>,
    rx: Option<Receiver<Vec<(usize, State)>>>,
    edits: Vec<Edited>,
    covered: usize,
    wanted: usize,
    until: usize,
}

fn moved(at: usize, &((from, to), delta): &Edited) -> Option<usize> {
    match at {
        at if at <= from => Some(at),
        at if at > to => Some(at.wrapping_add_signed(delta)),
        _ => None,
    }
}

impl Checkpoints {
    fn start(&mut self, rope: crop::Rope, from: usize, state: State) {
        let (tx, rx) = channel();
        let until = self.wanted;
        (self.rx, self.edits, self.until) = (Some(rx), Vec::new(), until);
        std::thread::spawn(move || {
            let _zone = aq_trace::zone("фон: контрольные точки разбора");
            scan(rope, from, state, until, tx);
        });
    }

    fn poll(&mut self) -> bool {
        let Some(rx) = &self.rx else { return false };
        let before = self.points.len();
        let finished = loop {
            match rx.try_recv() {
                Ok(batch) => {
                    let edits = &self.edits;
                    self.points.extend(batch.into_iter().filter_map(|(at, state)| Some((edits.iter().try_fold(at, moved)?, state))));
                }
                Err(TryRecvError::Empty) => break false,
                Err(TryRecvError::Disconnected) => break true,
            }
        };
        if finished {
            (self.rx, self.edits) = (None, Vec::new());
            self.covered = self.covered.max(self.until);
        }
        finished || self.points.len() != before
    }

    fn reached(&self, byte: usize) -> bool {
        self.covered >= byte || self.points.last().is_some_and(|&(at, _)| at >= byte)
    }

    fn before(&self, byte: usize) -> Option<(usize, State)> {
        let at = self.points.partition_point(|(at, _)| *at <= byte);
        at.checked_sub(1).map(|i| self.points[i])
    }

    fn edited(&mut self, window: (usize, usize), delta: isize) {
        let edit = (window, delta);
        self.points.retain_mut(|(at, _)| moved(*at, &edit).map(|to| *at = to).is_some());
        self.covered = moved(self.covered, &edit).unwrap_or(window.0);
        if self.rx.is_some() {
            self.edits.push(edit);
        }
    }

    fn cut(&mut self, from: usize) {
        self.rx = None;
        self.points.retain(|(at, _)| *at < from);
        self.covered = self.covered.min(from);
    }
}

impl Document {
    pub(super) fn rescan(&mut self) {
        self.points = Checkpoints::default();
    }

    fn want(&mut self, byte: usize) {
        self.points.wanted = self.points.wanted.max(byte);
        if self.windowed && self.points.rx.is_none() && !self.points.reached(self.points.wanted) {
            let (from, state) = self.points.points.last().copied().unwrap_or_default();
            self.points.start(self.full.clone(), from, state);
        }
    }

    pub(super) fn checkpoint(&mut self, byte: usize) -> State {
        self.points.poll();
        self.want(byte);
        match self.points.before(byte) {
            Some((at, state)) if self.points.reached(byte) => walk(&self.full, at, state, 0, |start, _| start < byte),
            _ => State::default(),
        }
    }

    pub(super) fn refresh_entry(&mut self) {
        if self.base == 0 || !self.points.poll() {
            return;
        }
        self.want(self.base);
        if self.points.reached(self.base) {
            let entry = self.checkpoint(self.base);
            if entry != self.entry {
                self.entry = entry;
                self.classify();
            }
        }
    }

    pub(super) fn rescan_from(&mut self, at: usize) {
        if self.windowed {
            let (from, state) = self.points.before(at).unwrap_or_default();
            self.points.cut(from);
            self.points.start(self.full.clone(), from, state);
        }
    }

    pub(super) fn checkpoints_edited(&mut self, window: (usize, usize), delta: isize, exit_changed: bool) {
        self.points.edited(window, delta);
        if exit_changed {
            self.rescan_from(window.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Checkpoints, State, channel};

    #[test]
    fn points_of_a_running_scan_follow_later_edits_and_the_end_is_reached() {
        let (tx, rx) = channel();
        let mut points = Checkpoints { rx: Some(rx), until: 400, ..Checkpoints::default() };
        points.edited((100, 200), 5);
        tx.send([50, 150, 300].map(|at| (at, State::default())).to_vec()).unwrap();
        assert!(points.poll());
        assert_eq!(points.points.iter().map(|(at, _)| *at).collect::<Vec<_>>(), [50, 305]);
        assert!(!points.reached(400));
        drop(tx);
        assert!(points.poll());
        assert!(points.reached(400));
    }
}
