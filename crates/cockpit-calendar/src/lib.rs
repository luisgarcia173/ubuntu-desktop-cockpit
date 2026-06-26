use cockpit_domain::Event;

#[derive(Debug, Default)]
pub struct LocalEventsProvider;

impl LocalEventsProvider {
    pub fn events_for_today(events: &[Event]) -> Vec<Event> {
        let mut events = events.to_vec();
        events.sort_by(|left, right| left.time.cmp(&right.time));
        events
    }

    pub fn next_events(events: &[Event], limit: usize) -> Vec<Event> {
        Self::events_for_today(events)
            .into_iter()
            .take(limit)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_events_by_time() {
        let events = vec![event("13:30", "Daily"), event("10:00", "Aula Marc")];

        let sorted = LocalEventsProvider::events_for_today(&events);

        assert_eq!(sorted[0].title, "Aula Marc");
        assert_eq!(sorted[1].title, "Daily");
    }

    #[test]
    fn limits_next_events() {
        let events = vec![event("09:00", "One"), event("10:00", "Two")];

        let next = LocalEventsProvider::next_events(&events, 1);

        assert_eq!(next.len(), 1);
        assert_eq!(next[0].title, "One");
    }

    fn event(time: &str, title: &str) -> Event {
        Event {
            time: time.to_string(),
            title: title.to_string(),
            description: None,
        }
    }
}
