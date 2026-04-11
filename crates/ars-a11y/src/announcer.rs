//! Screen-reader announcement queue and live-region attribute helpers.

use alloc::{format, string::String, vec::Vec};

use ars_core::{AriaAttr, AttrMap, HtmlAttr};

/// Priority of a live announcement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnnouncementPriority {
    /// `aria-live="polite"` waits for the user to finish speaking before announcing.
    Polite,
    /// `aria-live="assertive"` interrupts the current speech immediately.
    Assertive,
}

/// A pending announcement in the queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Announcement {
    /// The text to announce.
    pub message: String,
    /// The urgency of the announcement.
    pub priority: AnnouncementPriority,
}

/// Service for screen reader announcements, coordinated by the DOM adapter.
#[derive(Debug)]
pub struct LiveAnnouncer {
    /// Queue of pending announcements.
    queue: Vec<Announcement>,
    /// Whether an announcement is currently being processed.
    announcing: bool,
    /// Priority of the currently active announcement, if any.
    active_priority: Option<AnnouncementPriority>,
    /// Toggle bit for `VoiceOver` deduplication workaround.
    voiceover_toggle: bool,
    /// Tracks the last announced message text.
    last_message: Option<String>,
    /// Delay before clearing the live region content (ms).
    clear_delay_ms: u32,
}

impl LiveAnnouncer {
    /// Create a new `LiveAnnouncer`. Call `ensure_dom()` in `ars-dom` before first use.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            queue: Vec::new(),
            announcing: false,
            active_priority: None,
            voiceover_toggle: false,
            last_message: None,
            clear_delay_ms: 7000,
        }
    }

    /// Announce a message with polite priority.
    ///
    /// The message will be announced when the user is idle.
    pub fn announce(&mut self, message: impl Into<String>) {
        self.announce_with_priority(message, AnnouncementPriority::Polite);
    }

    /// Announce a message with assertive priority.
    ///
    /// The message will interrupt current screen reader speech.
    pub fn announce_assertive(&mut self, message: impl Into<String>) {
        self.announce_with_priority(message, AnnouncementPriority::Assertive);
    }

    /// Announce with explicit priority.
    pub fn announce_with_priority(
        &mut self,
        message: impl Into<String>,
        priority: AnnouncementPriority,
    ) {
        let announcement = Announcement {
            message: message.into(),
            priority,
        };

        if priority == AnnouncementPriority::Assertive {
            self.queue
                .retain(|queued| queued.priority == AnnouncementPriority::Assertive);

            if self.active_priority == Some(AnnouncementPriority::Polite) {
                self.announcing = false;
                self.active_priority = None;
            }
        }

        self.queue.push(announcement);
        self.process_queue();
    }

    /// Clear all pending announcements.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    fn process_queue(&mut self) {
        if self.announcing || self.queue.is_empty() {
            return;
        }

        self.queue
            .sort_by_key(|announcement| core::cmp::Reverse(announcement.priority));

        let next = self.queue.remove(0);
        self.announcing = true;
        self.active_priority = Some(next.priority);

        let content = render_announcement_content(
            next.message.as_str(),
            self.last_message.as_deref(),
            &mut self.voiceover_toggle,
        );

        self.last_message = Some(next.message.clone());

        let _unused = (content, next.priority, self.clear_delay_ms);
    }

    /// Called by the ars-dom adapter after the live region update completes.
    pub fn notify_announced(&mut self) {
        self.announcing = false;
        self.active_priority = None;
        self.process_queue();
    }

    /// Returns `AttrMap` for the polite live region element.
    #[must_use]
    pub fn polite_region_attrs() -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, "ars-live-polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs.set(HtmlAttr::Data("ars-part"), "live-region");
        attrs.set(HtmlAttr::Class, "ars-visually-hidden");
        attrs
    }

    /// Returns `AttrMap` for the assertive live region element.
    ///
    /// Builds assertive attrs by extending the polite live-region structure and
    /// overriding the `id`, `aria-live`, and `aria-relevant` values.
    #[must_use]
    pub fn assertive_region_attrs() -> AttrMap {
        let mut attrs = Self::polite_region_attrs();
        attrs.set(HtmlAttr::Id, "ars-live-assertive");
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "assertive");
        attrs.set(HtmlAttr::Aria(AriaAttr::Relevant), "additions text");
        attrs
    }
}

impl Default for LiveAnnouncer {
    fn default() -> Self {
        Self::new()
    }
}

fn render_announcement_content(
    message: &str,
    last_message: Option<&str>,
    voiceover_toggle: &mut bool,
) -> String {
    let is_repeat = last_message == Some(message);

    if is_repeat {
        *voiceover_toggle = !*voiceover_toggle;
        if *voiceover_toggle {
            format!("{message}\u{200D}")
        } else {
            String::from(message)
        }
    } else {
        *voiceover_toggle = false;
        String::from(message)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn announce_defaults_to_polite_and_begins_processing() {
        let mut announcer = LiveAnnouncer::new();

        announcer.announce("Polite update");

        assert!(announcer.announcing);
        assert!(announcer.queue.is_empty());
        assert_eq!(announcer.last_message.as_deref(), Some("Polite update"));
        assert!(!announcer.voiceover_toggle);
    }

    #[test]
    fn assertive_announcement_prunes_queued_polite_items() {
        let mut announcer = LiveAnnouncer::new();

        announcer.announce("Current polite");
        announcer.announce("Queued polite");
        announcer.announce_assertive("Critical alert");

        assert!(announcer.announcing);
        assert_eq!(announcer.active_priority, Some(AnnouncementPriority::Assertive));
        assert_eq!(announcer.last_message.as_deref(), Some("Critical alert"));
        assert!(announcer.queue.is_empty());
    }

    #[test]
    fn notify_announced_advances_the_queue() {
        let mut announcer = LiveAnnouncer::new();

        announcer.announce("First");
        announcer.announce("Second");

        announcer.notify_announced();

        assert!(announcer.announcing);
        assert!(announcer.queue.is_empty());
        assert_eq!(announcer.last_message.as_deref(), Some("Second"));
    }

    #[test]
    fn clear_removes_pending_announcements_without_breaking_future_use() {
        let mut announcer = LiveAnnouncer::new();

        announcer.announce("First");
        announcer.announce("Queued");
        announcer.clear();

        assert!(announcer.queue.is_empty());
        assert!(announcer.announcing);

        announcer.notify_announced();
        announcer.announce("After clear");

        assert!(announcer.announcing);
        assert_eq!(announcer.last_message.as_deref(), Some("After clear"));
    }

    #[test]
    fn assertive_announcements_preserve_fifo_order_within_priority() {
        let mut announcer = LiveAnnouncer::new();

        announcer.announce("Current polite");
        announcer.announce("Queued polite");
        announcer.announce_assertive("Critical alert 1");
        announcer.announce_assertive("Critical alert 2");

        assert_eq!(announcer.active_priority, Some(AnnouncementPriority::Assertive));
        assert_eq!(announcer.last_message.as_deref(), Some("Critical alert 1"));
        assert_eq!(
            announcer.queue,
            vec![Announcement {
                message: String::from("Critical alert 2"),
                priority: AnnouncementPriority::Assertive,
            },]
        );

        announcer.notify_announced();
        assert_eq!(announcer.active_priority, Some(AnnouncementPriority::Assertive));
        assert_eq!(announcer.last_message.as_deref(), Some("Critical alert 2"));
        assert!(announcer.queue.is_empty());

        announcer.notify_announced();
        assert_eq!(announcer.active_priority, None);
        assert_eq!(
            announcer.last_message.as_deref(),
            Some("Critical alert 2")
        );
    }

    #[test]
    fn assertive_preempts_active_polite_announcement() {
        let mut announcer = LiveAnnouncer::new();

        announcer.announce("Background update");
        assert_eq!(announcer.active_priority, Some(AnnouncementPriority::Polite));

        announcer.announce_assertive("Critical alert");

        assert!(announcer.announcing);
        assert_eq!(announcer.active_priority, Some(AnnouncementPriority::Assertive));
        assert_eq!(announcer.last_message.as_deref(), Some("Critical alert"));
        assert!(announcer.queue.is_empty());
    }

    #[test]
    fn repeated_identical_messages_toggle_voiceover_deduplication_state() {
        let mut announcer = LiveAnnouncer::new();

        announcer.announce("Test message");
        announcer.notify_announced();
        announcer.announce("Test message");

        assert!(announcer.voiceover_toggle);
        assert_eq!(announcer.last_message.as_deref(), Some("Test message"));

        announcer.notify_announced();
        announcer.announce("Test message");

        assert!(!announcer.voiceover_toggle);
    }

    #[test]
    fn voiceover_dedup_marker_is_emitted_on_first_repeat() {
        let mut toggle = false;

        let first = render_announcement_content("Test message", None, &mut toggle);
        let second = render_announcement_content("Test message", Some("Test message"), &mut toggle);
        let third = render_announcement_content("Test message", Some("Test message"), &mut toggle);

        assert_eq!(first, "Test message");
        assert_eq!(second, "Test message\u{200D}");
        assert_eq!(third, "Test message");
    }

    #[test]
    fn different_message_resets_voiceover_repeat_state() {
        let mut announcer = LiveAnnouncer::new();

        announcer.announce("Test message");
        announcer.notify_announced();
        announcer.announce("Test message");
        assert!(announcer.voiceover_toggle);

        announcer.notify_announced();
        announcer.announce("Different message");

        assert!(!announcer.voiceover_toggle);
        assert_eq!(announcer.last_message.as_deref(), Some("Different message"));

        announcer.notify_announced();
        announcer.announce("Test message");

        assert!(!announcer.voiceover_toggle);
        assert_eq!(announcer.last_message.as_deref(), Some("Test message"));
    }

    #[test]
    fn polite_region_attrs_match_spec() {
        let attrs = LiveAnnouncer::polite_region_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("ars-live-polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("live-region"));
        assert_eq!(attrs.get(&HtmlAttr::Class), Some("ars-visually-hidden"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Relevant)), None);
    }

    #[test]
    fn assertive_region_attrs_match_spec() {
        let attrs = LiveAnnouncer::assertive_region_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("ars-live-assertive"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Live)),
            Some("assertive")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Relevant)),
            Some("additions text")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("live-region"));
        assert_eq!(attrs.get(&HtmlAttr::Class), Some("ars-visually-hidden"));
    }

    #[test]
    fn clear_delay_defaults_to_seven_seconds() {
        let announcer = LiveAnnouncer::new();

        assert_eq!(announcer.clear_delay_ms, 7000);
    }

    #[test]
    fn clear_delay_can_be_overridden_in_module_construction() {
        let announcer = LiveAnnouncer {
            queue: Vec::new(),
            announcing: false,
            active_priority: None,
            voiceover_toggle: false,
            last_message: None,
            clear_delay_ms: 1200,
        };

        assert_eq!(announcer.clear_delay_ms, 1200);
    }

    #[test]
    fn default_matches_new() {
        let announcer = LiveAnnouncer::default();

        assert_eq!(announcer.queue, Vec::<Announcement>::new());
        assert!(!announcer.announcing);
        assert_eq!(announcer.active_priority, None);
        assert!(!announcer.voiceover_toggle);
        assert_eq!(announcer.last_message, None);
        assert_eq!(announcer.clear_delay_ms, 7000);
    }
}
