//! The app's five screens and how one leads to the next, without I/O: the
//! Windows side turns each [`Effect`] into work on a background thread and
//! feeds the outcome back as an [`Event`]. Tested on any system.

use knurl_probe::CheckStep;
use knurl_report::Report;

/// The screens, in the order a person goes through them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Welcome,
    Choose,
    Checking,
    Result,
    Send,
}

/// The search for paired Sony headphones on the Choose screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Search {
    Searching,
    /// The names Windows shows, one per pair of headphones.
    Found(Vec<String>),
    /// None paired, or none switched on.
    NoneFound,
    /// The Windows lookup failed, with its reason.
    Failed(String),
}

/// Sending the report to the relay, on the Send screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sending {
    Idle,
    InProgress,
    /// The relay opened this issue.
    Sent(u64),
    /// Not sent, with the reason.
    Failed(String),
}

/// What happened: a click, or the outcome of work the app asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Start,
    SearchAgain,
    DevicesFound(Result<Vec<String>, String>),
    Choose(usize),
    Progress(CheckStep),
    Checked(Box<Report>),
    CheckAgain,
    ToSend,
    ToggleData,
    /// The file was written (its path) or not (the reason).
    Saved(Result<String, String>),
    Send,
    /// The relay opened an issue (its number) or the report was not sent.
    SendDone(Result<u64, String>),
    Back,
}

/// Work the Windows side must start after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Look for the paired Sony headphones.
    Search,
    /// Check the headphones at this index of the last search.
    Check(usize),
    /// Send the report to the relay.
    Send,
}

/// The state of the whole app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flow {
    pub page: Page,
    pub search: Search,
    /// The headphones being checked: index in the search and name.
    pub chosen: Option<(usize, String)>,
    /// The steps the check has started, in order.
    pub steps: Vec<CheckStep>,
    pub report: Option<Report>,
    /// The exact data that would be sent is shown.
    pub show_data: bool,
    pub saved: Option<Result<String, String>>,
    pub sending: Sending,
}

impl Default for Flow {
    fn default() -> Self {
        Flow {
            page: Page::Welcome,
            search: Search::Searching,
            chosen: None,
            steps: Vec::new(),
            report: None,
            show_data: false,
            saved: None,
            sending: Sending::Idle,
        }
    }
}

impl Flow {
    /// Applies an event and says what work to start, if any. An event that
    /// does not belong to the current screen (a late answer, a double click)
    /// changes nothing.
    pub fn handle(&mut self, event: Event) -> Option<Effect> {
        match (self.page, event) {
            (Page::Welcome, Event::Start) => {
                self.page = Page::Choose;
                self.search = Search::Searching;
                Some(Effect::Search)
            }
            (Page::Choose, Event::SearchAgain) if self.search != Search::Searching => {
                self.search = Search::Searching;
                Some(Effect::Search)
            }
            (Page::Choose, Event::DevicesFound(found)) if self.search == Search::Searching => {
                self.search = match found {
                    Ok(names) if names.is_empty() => Search::NoneFound,
                    Ok(names) => Search::Found(names),
                    Err(reason) => Search::Failed(reason),
                };
                None
            }
            (Page::Choose, Event::Choose(index)) => {
                let Search::Found(names) = &self.search else {
                    return None;
                };
                let name = names.get(index)?.clone();
                self.chosen = Some((index, name));
                self.start_check(index)
            }
            (Page::Checking, Event::Progress(step)) => {
                if !self.steps.contains(&step) {
                    self.steps.push(step);
                }
                None
            }
            (Page::Checking, Event::Checked(report)) => {
                self.report = Some(*report);
                self.page = Page::Result;
                None
            }
            (Page::Result, Event::CheckAgain) => {
                let (index, _) = self.chosen.clone()?;
                self.start_check(index)
            }
            (Page::Result, Event::ToSend) => {
                self.page = Page::Send;
                self.show_data = false;
                self.saved = None;
                None
            }
            (Page::Send, Event::ToggleData) => {
                self.show_data = !self.show_data;
                None
            }
            (Page::Send, Event::Saved(result)) => {
                self.saved = Some(result);
                None
            }
            // Once sent, a second click would open a second issue.
            (Page::Send, Event::Send)
                if self.report.is_some()
                    && matches!(self.sending, Sending::Idle | Sending::Failed(_)) =>
            {
                self.sending = Sending::InProgress;
                Some(Effect::Send)
            }
            (Page::Send, Event::SendDone(result)) if self.sending == Sending::InProgress => {
                self.sending = match result {
                    Ok(issue) => Sending::Sent(issue),
                    Err(reason) => Sending::Failed(reason),
                };
                None
            }
            (Page::Choose, Event::Back) => {
                self.page = Page::Welcome;
                None
            }
            (Page::Result, Event::Back) => {
                // The last search stays: the list is still there.
                self.page = Page::Choose;
                None
            }
            // Not while sending: the answer belongs to this screen.
            (Page::Send, Event::Back) if self.sending != Sending::InProgress => {
                self.page = Page::Result;
                None
            }
            _ => None,
        }
    }

    fn start_check(&mut self, index: usize) -> Option<Effect> {
        self.page = Page::Checking;
        self.steps.clear();
        self.report = None;
        // A new report can be sent; the last one's outcome belonged to it.
        self.sending = Sending::Idle;
        Some(Effect::Check(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knurl_report::{Answers, REPORT_FORMAT, ServiceResult, Services, Verdict};

    fn report(model: &str) -> Box<Report> {
        Box::new(Report {
            format: REPORT_FORMAT,
            tool: "knurl-probe 0.1.0".into(),
            date: "2026-11-18".into(),
            checked_at: "2026-11-18T10:00:00Z".into(),
            os: "Windows 10.0.26100".into(),
            device_name: model.into(),
            services: Services {
                v1: ServiceResult::Opened,
                v2: ServiceResult::NotOffered,
            },
            answers: Answers {
                model: Some(model.into()),
                ..Answers::default()
            },
            verdict: Verdict::NewModel,
            frames: Vec::new(),
            problems: Vec::new(),
        })
    }

    #[test]
    fn a_whole_run_goes_through_the_five_screens() {
        let mut f = Flow::default();
        assert_eq!(f.page, Page::Welcome);
        assert_eq!(f.handle(Event::Start), Some(Effect::Search));
        assert_eq!((f.page, &f.search), (Page::Choose, &Search::Searching));
        let names = vec!["WH-1000XM3".to_string(), "WH-1000XM6".to_string()];
        assert_eq!(f.handle(Event::DevicesFound(Ok(names.clone()))), None);
        assert_eq!(f.search, Search::Found(names));
        assert_eq!(f.handle(Event::Choose(1)), Some(Effect::Check(1)));
        assert_eq!(f.page, Page::Checking);
        assert_eq!(f.chosen, Some((1, "WH-1000XM6".to_string())));
        f.handle(Event::Progress(CheckStep::Opening));
        f.handle(Event::Progress(CheckStep::Asking));
        f.handle(Event::Progress(CheckStep::Asking));
        assert_eq!(f.steps, [CheckStep::Opening, CheckStep::Asking]);
        f.handle(Event::Checked(report("WH-1000XM6")));
        assert_eq!(f.page, Page::Result);
        assert!(f.report.is_some());
        f.handle(Event::ToSend);
        assert_eq!(f.page, Page::Send);
        f.handle(Event::ToggleData);
        assert!(f.show_data);
        f.handle(Event::Saved(Ok("report.json".into())));
        assert_eq!(f.saved, Some(Ok("report.json".into())));
    }

    #[test]
    fn no_headphones_and_a_failed_lookup_can_be_searched_again() {
        let mut f = Flow::default();
        f.handle(Event::Start);
        f.handle(Event::DevicesFound(Ok(Vec::new())));
        assert_eq!(f.search, Search::NoneFound);
        assert_eq!(f.handle(Event::Choose(0)), None, "nothing to choose");
        assert_eq!(f.handle(Event::SearchAgain), Some(Effect::Search));
        assert_eq!(f.handle(Event::SearchAgain), None, "already searching");
        f.handle(Event::DevicesFound(Err("Bluetooth is off".into())));
        assert_eq!(f.search, Search::Failed("Bluetooth is off".into()));
        assert_eq!(f.handle(Event::SearchAgain), Some(Effect::Search));
    }

    #[test]
    fn late_or_misplaced_events_change_nothing() {
        let mut f = Flow::default();
        let before = f.clone();
        for e in [
            Event::Choose(0),
            Event::Progress(CheckStep::Asking),
            Event::Checked(report("WH-1000XM3")),
            Event::ToggleData,
            Event::Saved(Err("x".into())),
            Event::Back,
        ] {
            assert_eq!(f.handle(e), None);
        }
        assert_eq!(f, before);
        f.handle(Event::Start);
        f.handle(Event::DevicesFound(Ok(vec!["WH-1000XM3".into()])));
        assert_eq!(f.handle(Event::Choose(5)), None, "index out of range");
        assert_eq!(f.page, Page::Choose);
        // A second answer to the same search is ignored.
        f.handle(Event::DevicesFound(Ok(Vec::new())));
        assert_eq!(f.search, Search::Found(vec!["WH-1000XM3".into()]));
    }

    #[test]
    fn send_once_retry_after_a_failure_never_twice_after_success() {
        let mut f = Flow::default();
        f.handle(Event::Start);
        f.handle(Event::DevicesFound(Ok(vec!["WH-1000XM6".into()])));
        f.handle(Event::Choose(0));
        f.handle(Event::Checked(report("WH-1000XM6")));
        assert_eq!(f.handle(Event::Send), None, "not on the result screen");
        f.handle(Event::ToSend);
        assert_eq!(f.handle(Event::Send), Some(Effect::Send));
        assert_eq!(f.sending, Sending::InProgress);
        assert_eq!(f.handle(Event::Send), None, "already sending");
        assert_eq!(f.handle(Event::Back), None, "no way back while sending");
        assert_eq!(f.page, Page::Send);
        f.handle(Event::SendDone(Err("timeout".into())));
        assert_eq!(f.sending, Sending::Failed("timeout".into()));
        assert_eq!(f.handle(Event::Send), Some(Effect::Send), "try again");
        f.handle(Event::SendDone(Ok(12)));
        assert_eq!(f.sending, Sending::Sent(12));
        assert_eq!(
            f.handle(Event::Send),
            None,
            "a second issue is never opened"
        );
        // A late answer changes nothing.
        f.handle(Event::SendDone(Err("late".into())));
        assert_eq!(f.sending, Sending::Sent(12));
        f.handle(Event::Back);
        f.handle(Event::ToSend);
        assert_eq!(f.sending, Sending::Sent(12), "the same report stays sent");
        assert_eq!(f.handle(Event::Send), None);
        f.handle(Event::Back);
        f.handle(Event::CheckAgain);
        f.handle(Event::Checked(report("WH-1000XM6")));
        f.handle(Event::ToSend);
        assert_eq!(f.sending, Sending::Idle, "a new report can be sent");
    }

    #[test]
    fn back_and_check_again() {
        let mut f = Flow::default();
        f.handle(Event::Start);
        f.handle(Event::DevicesFound(Ok(vec!["WH-1000XM3".into()])));
        f.handle(Event::Choose(0));
        assert_eq!(f.handle(Event::Back), None, "no way back while checking");
        assert_eq!(f.page, Page::Checking);
        f.handle(Event::Progress(CheckStep::Opening));
        f.handle(Event::Checked(report("WH-1000XM3")));
        assert_eq!(f.handle(Event::CheckAgain), Some(Effect::Check(0)));
        assert!(f.steps.is_empty() && f.report.is_none());
        f.handle(Event::Checked(report("WH-1000XM3")));
        f.handle(Event::ToSend);
        f.handle(Event::Saved(Ok("a".into())));
        f.handle(Event::Back);
        assert_eq!(f.page, Page::Result);
        f.handle(Event::ToSend);
        assert_eq!(f.saved, None, "a new visit starts clean");
        f.handle(Event::Back);
        f.handle(Event::Back);
        assert_eq!(f.page, Page::Choose);
        assert_eq!(f.search, Search::Found(vec!["WH-1000XM3".into()]));
        f.handle(Event::Back);
        assert_eq!(f.page, Page::Welcome);
    }
}
